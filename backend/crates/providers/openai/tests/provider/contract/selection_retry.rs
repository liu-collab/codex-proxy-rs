//! 验证真实 OpenAI Provider 经 Core 重试收尾时保留上游错误与本地会话边界

use super::session_binding::turn_request;
use super::*;
use async_trait::async_trait;
use gateway_core::engine::provider::ProviderRegistry;
use gateway_core::engine::{
    AttemptCoordinator, AttemptRecord, EngineError, ExecutionStore, GatewayEngine,
    IntermediateFailure, ModelRequestFinalization, NewModelRequest, RecoveryReport,
};
use gateway_core::error::StoreError;
use gateway_core::routing::AccountRoutingSnapshot;

struct NoopExecutionStore;

#[async_trait]
impl ExecutionStore for NoopExecutionStore {
    async fn create_model_request(&self, _: NewModelRequest) -> Result<(), StoreError> {
        Ok(())
    }

    async fn record_attempt(&self, _: AttemptRecord) -> Result<(), StoreError> {
        Ok(())
    }

    async fn mark_send_state(
        &self,
        _: &ModelRequestId,
        _: UpstreamSendState,
    ) -> Result<(), StoreError> {
        Ok(())
    }

    async fn mark_downstream_committed(
        &self,
        _: &ModelRequestId,
        _: SystemTime,
        _: Option<u16>,
    ) -> Result<(), StoreError> {
        Ok(())
    }

    async fn record_client_status(&self, _: &ModelRequestId, _: u16) -> Result<(), StoreError> {
        Ok(())
    }

    async fn record_intermediate_failure(&self, _: IntermediateFailure) -> Result<(), StoreError> {
        Ok(())
    }

    async fn finalize_model_request(&self, _: ModelRequestFinalization) -> Result<(), StoreError> {
        Ok(())
    }

    async fn recover_expired(&self, _: SystemTime) -> Result<RecoveryReport, StoreError> {
        Ok(RecoveryReport::default())
    }
}

#[tokio::test]
async fn cyber_policy_rotation_preserves_upstream_response_and_next_request_stays_local() {
    let store = Arc::new(MemoryAccountStore::default());
    create_account(&store, "acct_subagent_a").await;
    let server = MockServer::start().await;
    let body = json!({"error":{"code":"cyber_policy","type":"invalid_request_error","message":"synthetic policy rejection"}});
    Mock::given(method("POST"))
        .and(path("/codex/responses"))
        .respond_with(ResponseTemplate::new(400).set_body_json(&body))
        .mount(&server)
        .await;
    let provider = provider_with_affinity_and_base_url(
        &store,
        Arc::new(MemorySessionAffinity::default()),
        server.uri(),
    );
    let providers =
        ProviderRegistry::new([provider as Arc<dyn gateway_core::engine::provider::Provider>])
            .unwrap();
    let coordinator =
        AttemptCoordinator::new(GatewayEngine::new(Arc::new(NoopExecutionStore), providers));
    let operation = turn_request("policy-session", "policy-turn");
    let kind = ProviderKind::new("openai").unwrap();
    let snapshot = RuntimeSnapshot::new(
        ConfigRevision::new(1).unwrap(),
        SettingsValues::new(3, "smart", Default::default(), None, None)
            .with_request_location(global_request_location(), true),
        vec![kind.clone()],
        vec![ProviderModel::new(
            kind,
            UpstreamModelId::new("gpt-5.4").unwrap(),
            ModelCapabilities::new(BTreeSet::from([operation.kind()]), Some(32_000))
                .with_upstream_feature_validation(),
        )],
        Vec::new(),
    )
    .unwrap();
    let plan = snapshot
        .plan(
            &PublicModelId::new("gpt-5.4").unwrap(),
            &operation,
            contract_account_scope(),
            &RoutingContext::default(),
        )
        .unwrap();
    let request = |id| NewModelRequest {
        admission_decision_ms: None,
        id: ModelRequestId::new(id).unwrap(),
        client_api_key_name_snapshot: None,
        client_api_key_ref: ClientApiKeyId::new("key_openai_contract").unwrap(),
        config_revision: ConfigRevision::new(1).unwrap(),
        routing: AccountRoutingSnapshot::all(),
        protocol: "openai".to_owned(),
        operation: operation.kind(),
        endpoint: "responses".to_owned(),
        client_transport: "http_sse".to_owned(),
        requested_model: Some(PublicModelId::new("gpt-5.4").unwrap()),
        client_ip: None,
        user_agent: None,
        reasoning_effort: None,
        reasoning_preset: None,
        request_kind: None,
        subagent_kind: None,
        compact: false,
        continuation: Default::default(),
        image_generation_requested: false,
        started_at: SystemTime::now(),
        deadline_at: (SystemTime::now() + Duration::from_secs(10)).into(),
    };
    let mut session = coordinator
        .start(
            request("req_policy_first"),
            operation.clone(),
            plan.clone(),
            None,
            None,
            CancellationToken::new(),
        )
        .await
        .unwrap();
    let error = session
        .collect_uncommitted()
        .await
        .expect_err("policy rejection must surface");
    let EngineError::Provider(error) = error else {
        panic!("expected Provider error: {error:?}")
    };
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert_eq!(error.upstream_status(), Some(400));
    assert_eq!(
        error.client_visible_upstream_error().unwrap().message(),
        "synthetic policy rejection"
    );
    let response = error
        .client_visible_upstream_response()
        .expect("original HTTP rejection must survive empty account selection");
    assert_eq!(
        serde_json::from_slice::<Value>(response.body()).unwrap(),
        body
    );
    assert_eq!(response.status(), 400);
    // 同一会话的新请求没有可恢复的上游失败，只返回本地策略提示
    let mut next = coordinator
        .start(
            request("req_policy_next"),
            operation.clone(),
            plan,
            None,
            None,
            CancellationToken::new(),
        )
        .await
        .unwrap();
    let EngineError::Provider(error) = next.collect_uncommitted().await.unwrap_err() else {
        panic!("expected local selection failure");
    };
    assert_eq!(error.kind(), ProviderErrorKind::SessionPolicyRestricted);
    assert!(error.retry_is_prohibited());
    assert_eq!(error.upstream_status(), None);
    assert!(error.client_visible_upstream_response().is_none());
    assert_eq!(
        error.client_visible_upstream_error().unwrap().code(),
        Some("cyber_policy")
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}
