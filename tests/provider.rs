use advisor_review::config::ProviderConfig;

#[test]
fn provider_metadata_never_contains_credentials() {
    unsafe { std::env::set_var("ADVISOR_REVIEW_API_KEY", "test-secret") };
    let config = ProviderConfig::from_values(Some("openrouter".into()), Some("test-model".into()));
    let metadata = serde_json::to_string(&config.metadata()).unwrap();
    assert!(!metadata.contains("test-secret"));
    unsafe { std::env::remove_var("ADVISOR_REVIEW_API_KEY") };
}
