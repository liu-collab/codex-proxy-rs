//! 验证调用表在成功、远端错误和超时的结果交付前同步撤销回调资源

use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use gateway_plugin_runtime::{RpcError, RpcLimits, RpcSession};
use gateway_plugin_sdk::Stage;
use serde_json::json;

use super::session::{LifecycleCallbacks, prepared_session};

struct GatedFinish {
    ready: Arc<tokio::sync::Notify>,
    entered: std::sync::mpsc::Sender<()>,
    started: AtomicBool,
    finished: AtomicBool,
    released: Mutex<bool>,
    release: Condvar,
}

impl gateway_plugin_runtime::CallbackHandler for GatedFinish {
    fn finish(&self, _: &gateway_plugin_sdk::CallContext) {
        if self.started.swap(true, Ordering::AcqRel) {
            return;
        }
        self.entered.send(()).unwrap();
        // 测试屏障模拟撤销处理中被调度器暂停，不依赖恰好命中的线程交错
        let released = self.released.lock().unwrap();
        let (released, timeout) = self
            .release
            .wait_timeout_while(released, Duration::from_secs(5), |released| !*released)
            .unwrap();
        assert!(!timeout.timed_out() && *released);
        self.finished.store(true, Ordering::Release);
    }

    fn call(
        &self,
        _: gateway_plugin_sdk::CallContext,
        method: String,
        _: serde_json::Value,
        _: Vec<u8>,
    ) -> futures::future::BoxFuture<
        'static,
        Result<gateway_plugin_runtime::RpcReply, gateway_plugin_sdk::PluginFault>,
    > {
        let ready = Arc::clone(&self.ready);
        Box::pin(async move {
            assert_eq!(method, "test.stream_ready");
            ready.notified().await;
            Ok(gateway_plugin_runtime::RpcReply {
                result: json!({}),
                payload: vec![],
            })
        })
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dropping_stream_waits_for_resource_revocation_even_after_end_or_process_failure() {
    for method in ["stream_single", "stream_single_crash"] {
        let (_cache, prepared, handshake, processes) = prepared_session();
        let (entered, entry) = std::sync::mpsc::channel();
        let callbacks = Arc::new(GatedFinish {
            ready: Arc::new(tokio::sync::Notify::new()),
            entered,
            started: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            released: Mutex::new(false),
            release: Condvar::new(),
        });
        let session = Arc::new(
            RpcSession::start(
                prepared,
                handshake,
                RpcLimits::default(),
                &processes,
                callbacks.clone(),
            )
            .await
            .unwrap(),
        );
        let stream = session
            .call_stream(
                method,
                session.context(Stage::Request, Duration::from_secs(5)),
                json!({}),
                vec![],
            )
            .await
            .unwrap();
        // 拿到流后才允许对端结束；整个屏障控制不依赖被暂停的 I/O 调度器
        callbacks.ready.notify_one();
        entry.recv_timeout(Duration::from_secs(5)).unwrap();
        let observed = Arc::clone(&callbacks);
        let (started, received) = std::sync::mpsc::channel();
        let (completed, completion) = std::sync::mpsc::channel();
        let dropped = std::thread::spawn(move || {
            started.send(()).unwrap();
            drop(stream);
            completed
                .send(observed.finished.load(Ordering::Acquire))
                .unwrap();
        });
        received.recv_timeout(Duration::from_secs(1)).unwrap();
        // 撤销屏障会暂停 I/O 调度线程，不能用同一 runtime 的计时器释放它
        let early = completion.recv_timeout(Duration::from_millis(100));
        *callbacks.released.lock().unwrap() = true;
        callbacks.release.notify_one();
        let was_early = early.is_ok();
        let revoked =
            early.unwrap_or_else(|_| completion.recv_timeout(Duration::from_secs(1)).unwrap());
        dropped.join().unwrap();
        session.shutdown(Duration::from_secs(1)).await;
        assert!(
            !was_early && revoked,
            "{method}: stream drop must not outrun callback revocation"
        );
    }
}

#[tokio::test]
async fn terminal_results_revoke_callback_resources_before_waking_callers() {
    for method in ["echo", "deny", "hang"] {
        let (_cache, prepared, handshake, processes) = prepared_session();
        let callbacks = Arc::new(LifecycleCallbacks::default());
        let session = RpcSession::start(
            prepared,
            handshake,
            RpcLimits::default(),
            &processes,
            callbacks.clone(),
        )
        .await
        .unwrap();
        let timeout = if method == "hang" {
            Duration::from_millis(50)
        } else {
            Duration::from_secs(2)
        };
        let result = session
            .call(
                method,
                session.context(Stage::Request, timeout),
                json!({}),
                Vec::new(),
            )
            .await;
        match method {
            "echo" => assert!(result.is_ok()),
            "deny" => assert!(matches!(result, Err(RpcError::Remote(_)))),
            "hang" => assert!(matches!(result, Err(RpcError::Timeout))),
            _ => unreachable!(),
        }
        assert_eq!(callbacks.begun.load(Ordering::Relaxed), 1);
        assert!(
            callbacks.finished.load(Ordering::Relaxed) >= 1,
            "调用结果已经交付，资源权限必须先同步撤销：{method}"
        );
        session.shutdown(Duration::from_secs(1)).await;
    }
}
