//! 客户端失败轮次的过期标记与原子条件清理

use std::time::Duration;

use futures::future::BoxFuture;
use gateway_core::{
    engine::{
        ModelRequestId,
        client_retry::{ClientRetryKey, ClientRetryStore},
    },
    error::{StoreError as CoreStoreError, StoreErrorKind},
};
use redis::aio::ConnectionManager;
use sha2::{Digest, Sha256};

use super::namespace;
use crate::StoreResult;

const CLEAR_SCRIPT: &str =
    "if redis.call('GET',KEYS[1])==ARGV[1] then return redis.call('DEL',KEYS[1]) else return 0 end";

#[derive(Clone)]
pub struct RedisClientRetryRepository {
    connection: ConnectionManager,
    namespace: String,
}

impl RedisClientRetryRepository {
    pub fn new(connection: ConnectionManager, key_namespace: &str) -> StoreResult<Self> {
        Ok(Self {
            connection,
            namespace: format!("{}:client-retry:v1", namespace(key_namespace)?),
        })
    }

    fn key(&self, key: &ClientRetryKey) -> String {
        let mut digest = Sha256::new();
        for part in [
            key.client_key_id.as_str(),
            key.turn.thread_id(),
            key.turn.turn_id(),
        ] {
            digest.update((part.len() as u64).to_be_bytes());
            digest.update(part.as_bytes());
        }
        format!("{}:{}", self.namespace, hex::encode(digest.finalize()))
    }
}

impl ClientRetryStore for RedisClientRetryRepository {
    fn load<'a>(
        &'a self,
        key: &'a ClientRetryKey,
    ) -> BoxFuture<'a, Result<Option<ModelRequestId>, CoreStoreError>> {
        Box::pin(async move {
            let mut connection = self.connection.clone();
            let value = redis::cmd("GET")
                .arg(self.key(key))
                .query_async::<Option<String>>(&mut connection)
                .await
                .map_err(unavailable)?;
            value
                .map(ModelRequestId::new)
                .transpose()
                .map_err(|error| CoreStoreError::caused_by(StoreErrorKind::InvalidData, error))
        })
    }

    fn record_failure<'a>(
        &'a self,
        key: &'a ClientRetryKey,
        version: &'a ModelRequestId,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<(), CoreStoreError>> {
        Box::pin(async move {
            let mut connection = self.connection.clone();
            redis::cmd("SET")
                .arg(self.key(key))
                .arg(version.as_str())
                .arg("PX")
                .arg(u64::try_from(ttl.as_millis()).unwrap_or(u64::MAX))
                .query_async::<()>(&mut connection)
                .await
                .map_err(unavailable)
        })
    }

    fn clear_failure<'a>(
        &'a self,
        key: &'a ClientRetryKey,
        expected: &'a ModelRequestId,
    ) -> BoxFuture<'a, Result<(), CoreStoreError>> {
        Box::pin(async move {
            let mut connection = self.connection.clone();
            // 小脚本直接 EVAL，避免冷启动 NOSCRIPT 补发增加终态交付的网络往返
            redis::cmd("EVAL")
                .arg(CLEAR_SCRIPT)
                .arg(1)
                .arg(self.key(key))
                .arg(expected.as_str())
                .query_async::<u64>(&mut connection)
                .await
                .map(|_| ())
                .map_err(unavailable)
        })
    }
}

fn unavailable(error: redis::RedisError) -> CoreStoreError {
    CoreStoreError::caused_by(StoreErrorKind::Unavailable, error)
}
