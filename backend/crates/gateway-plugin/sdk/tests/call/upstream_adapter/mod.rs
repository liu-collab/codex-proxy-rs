//! 验证上游适配器注册与调用合同

mod codec;

#[test]
fn registration_round_trip_preserves_explicit_account_scopes() {
    use gateway_plugin_sdk::call::upstream_adapter::UpstreamAdapterRegistration;
    use serde_json::json;

    for accounts in [json!([]), json!(["acct_one", "acct_two"])] {
        let wire = json!({"adapters":[{
            "id":"example", "provider":"openai", "base_url":"https://example.invalid/",
            "paths":[{"path":"responses","purpose":"inference"}],
            "authentication_kinds":["oauth"], "transport":"http_sse", "protocol":"openai",
            "account_ids":accounts
        }]});
        let registration: UpstreamAdapterRegistration = serde_json::from_value(wire).unwrap();
        let serialized = serde_json::to_value(registration).unwrap();
        let round_trip: UpstreamAdapterRegistration = serde_json::from_value(serialized).unwrap();
        assert_eq!(
            serde_json::to_value(&round_trip.adapters[0].account_ids).unwrap(),
            accounts
        );
    }
}
