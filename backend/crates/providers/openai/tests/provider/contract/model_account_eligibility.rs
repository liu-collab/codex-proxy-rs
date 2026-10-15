//! Grok 的 ChatGPT OAuth 选号边界与 API Key 未知模型透传合同

use super::*;

fn operation(model: &str) -> Operation {
    let payload = ProtocolPayload::json_object(
        "openai",
        Map::from_iter([
            ("model".to_owned(), json!(model)),
            ("input".to_owned(), json!("hello")),
        ]),
    )
    .unwrap()
    .with_context(Map::from_iter([("use_websocket".to_owned(), json!(false))]));
    Operation::Generate(GenerateRequest::from_protocol_payload(payload))
}

#[tokio::test]
async fn grok_models_never_acquire_chatgpt_oauth_accounts() {
    let oauth = MockServer::start().await;
    let store = Arc::new(MemoryAccountStore::default());
    create_account(&store, "acct_provider_contract").await;
    let leases = Arc::new(TestLeaseCoordinator::default());
    let provider = provider_with_affinity_and_base_url_and_leases(
        &store,
        Arc::default(),
        oauth.uri(),
        leases.clone(),
    );
    for model in ["grok-4.7", "xai/grok-4.7", "GROK-NEW", "grok"] {
        let error = provider
            .clone()
            .execute(
                planned_request_for_model("openai", operation(model), model),
                context("req_grok_oauth_rejected", CancellationToken::new()),
            )
            .await
            .err()
            .expect("Grok cannot select a ChatGPT account");
        assert_eq!(error.kind(), ProviderErrorKind::NoEligibleAccount);
    }
    assert!(leases.requests.lock().unwrap().is_empty());
    assert!(oauth.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn mixed_pool_routes_grok_to_api_key_despite_oauth_weight() {
    let api = MockServer::start().await;
    let oauth = MockServer::start().await;
    let store = Arc::new(MemoryAccountStore::default());
    create_account(&store, "acct_scope_new").await;
    store.set_scheduling("acct_scope_new", None, AccountWeight::new(100).unwrap());
    store
        .seed_api_key(
            "acct_provider_contract",
            api.uri(),
            provider_openai::credential::ResponsesTransport::Http,
        )
        .await;
    store.set_scheduling(
        "acct_provider_contract",
        None,
        AccountWeight::new(1).unwrap(),
    );
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(CAPTURE_COMPLETED_SSE, "text/event-stream"),
        )
        .expect(1)
        .mount(&api)
        .await;
    let mut stream = provider_with_base_url(&store, oauth.uri())
        .execute(
            planned_request_for_model("openai", operation("grok-4.7"), "grok-4.7"),
            context("req_grok_api_key", CancellationToken::new()),
        )
        .await
        .unwrap();
    assert_eq!(
        stream.metadata().provider_account_id().as_str(),
        "acct_provider_contract"
    );
    while let Some(event) = stream.next().await {
        event.unwrap();
    }
    let requests = api.received_requests().await.unwrap();
    assert_eq!(captured_request_body(&requests[0])["model"], "grok-4.7");
    assert!(oauth.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn unlisted_models_remain_available_without_catalog_discovery() {
    for api_key in [false, true] {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryAccountStore::default());
        if api_key {
            store
                .seed_api_key(
                    "acct_provider_contract",
                    server.uri(),
                    provider_openai::credential::ResponsesTransport::Http,
                )
                .await;
        } else {
            create_account(&store, "acct_provider_contract").await;
        }
        Mock::given(method("POST"))
            .and(path(if api_key {
                "/responses"
            } else {
                "/codex/responses"
            }))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(CAPTURE_COMPLETED_SSE, "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let mut stream = provider_with_base_url(&store, server.uri())
            .execute(
                planned_request_for_model(
                    "openai",
                    operation("custom-unlisted"),
                    "custom-unlisted",
                ),
                context("req_unlisted", CancellationToken::new()),
            )
            .await
            .unwrap();
        while let Some(event) = stream.next().await {
            event.unwrap();
        }
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            captured_request_body(&requests[0])["model"],
            "custom-unlisted"
        );
    }
}
