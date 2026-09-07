use advisor_review::config::ProviderConfig;

#[test]
fn provider_metadata_never_contains_credentials() {
    let config = ProviderConfig {
        name: Some("openrouter".into()),
        model: Some("test-model".into()),
        api_key: Some("test-secret".into()),
        ..Default::default()
    };
    let metadata = serde_json::to_string(&config.metadata()).unwrap();
    assert!(!metadata.contains("test-secret"));
    assert!(!format!("{config:?}").contains("test-secret"));
}
