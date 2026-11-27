use advisor_review::{
    config::ProviderConfig,
    guidelines::RuleRegistry,
    model::{DocumentTarget, RuleDefinition},
    providers::{ConfiguredProvider, SemanticProvider},
    review,
};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};

fn mock(status: u16, body: String) -> (String, thread::JoinHandle<Value>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/review", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(err)
                    if err.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(10))
                }
                Err(err) => panic!("mock did not receive request: {err}"),
            }
        };
        // On macOS and other BSDs, an accepted socket inherits the listener's
        // non-blocking mode, so reads fail with WouldBlock without this.
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut length = 0;
        let mut headers = serde_json::Map::new();
        loop {
            let mut line = String::new();
            assert!(reader.read_line(&mut line).unwrap() > 0);
            if line == "\r\n" {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                headers.insert(name.trim().to_ascii_lowercase(), json!(value.trim()));
                if name.eq_ignore_ascii_case("content-length") {
                    length = value.trim().parse().unwrap();
                }
            }
        }
        let mut request = vec![0; length];
        reader.read_exact(&mut request).unwrap();
        write!(stream, "HTTP/1.1 {status} Mock\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        let mut request: Value = serde_json::from_slice(&request).unwrap();
        request["_headers"] = Value::Object(headers);
        request
    });
    (url, handle)
}

fn rule() -> RuleDefinition {
    serde_yaml::from_str("id: caption\nscope: figure\nkind: semantic-text\ndescription: Check caption.\ncheck: {type: semantic}").unwrap()
}

fn target() -> DocumentTarget {
    DocumentTarget {
        id: "figure-1".into(),
        target_type: "figure".into(),
        text: "Caption: measured response".into(),
        ..Default::default()
    }
}

fn provider(name: &str, endpoint: String) -> ConfiguredProvider {
    ConfiguredProvider {
        config: ProviderConfig {
            name: Some(name.into()),
            model: Some("test-model".into()),
            endpoint: Some(endpoint),
            api_key: Some("test-secret".into()),
            ..Default::default()
        },
    }
}

fn pass_response() -> String {
    json!({"choices":[{"message":{"content":json!({"status":"pass","evidence":"Caption: measured response","explanation":"Clear"}).to_string()}}]}).to_string()
}

#[test]
fn openrouter_attribution_headers_are_optional_and_provider_specific() {
    let (endpoint, server) = mock(200, pass_response());
    let mut openrouter = provider("openrouter", endpoint);
    openrouter.config.openrouter_http_referer = Some("https://example.org".into());
    openrouter.config.openrouter_app_title = Some("Advisor Review".into());
    assert!(openrouter.review(&rule(), &target()).unwrap().is_none());
    let request = server.join().unwrap();
    assert_eq!(request["_headers"]["http-referer"], "https://example.org");
    assert_eq!(request["_headers"]["x-openrouter-title"], "Advisor Review");

    let (endpoint, server) = mock(200, pass_response());
    let openrouter = provider("openrouter", endpoint);
    assert!(openrouter.review(&rule(), &target()).unwrap().is_none());
    let request = server.join().unwrap();
    assert!(request["_headers"].get("http-referer").is_none());
    assert!(request["_headers"].get("x-openrouter-title").is_none());

    let (endpoint, server) = mock(200, pass_response());
    let mut openai = provider("openai", endpoint);
    openai.config.openrouter_http_referer = Some("https://example.org".into());
    openai.config.openrouter_app_title = Some("Advisor Review".into());
    assert!(openai.review(&rule(), &target()).unwrap().is_none());
    let request = server.join().unwrap();
    assert!(request["_headers"].get("http-referer").is_none());
    assert!(request["_headers"].get("x-openrouter-title").is_none());
}

#[test]
fn configuration_rejects_unknown_providers_and_requires_models() {
    for name in ["openai", "anthropic", "openrouter", "ollama"] {
        let mut config = provider(name, "http://localhost".into()).config;
        assert!(config.validate().is_ok());
        config.model = None;
        assert!(config.validate().is_err());
    }
    assert!(
        provider("typo", "http://localhost".into())
            .config
            .validate()
            .is_err()
    );
    assert!(
        provider("openai", "file:///secret".into())
            .config
            .validate()
            .is_err()
    );
    let mut local = provider("OLLAMA", "http://localhost".into());
    local.config.api_key = None;
    assert!(local.can_handle(&rule()));
    let mut vision = rule();
    vision.kind = "cross-modal".into();
    assert!(!local.can_handle(&vision));
}

#[cfg(unix)]
#[test]
fn installed_cli_providers_use_fake_executables_and_shared_evidence_validation() {
    use std::{fs, os::unix::fs::PermissionsExt};
    let dir = tempfile::tempdir().unwrap();
    let result = r#"{"summary":"Reviewed caption.","findings":[{"status":"concern","explanation":"The caption is underspecified.","evidence":[{"target_id":"figure-1","quote":"measured response"}]}]}"#;
    let claude_script = dir.path().join("claude-mock");
    fs::write(
        &claude_script,
        format!(
            "#!/bin/sh\ncat >/dev/null\nprintf '%s' '{{\"result\":{}}}'\n",
            serde_json::to_string(result).unwrap()
        ),
    )
    .unwrap();
    fs::set_permissions(&claude_script, fs::Permissions::from_mode(0o755)).unwrap();
    let codex_script = dir.path().join("codex-mock");
    fs::write(&codex_script, format!("#!/bin/sh\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = \"--output-last-message\" ]; then shift; out=\"$1\"; fi\n  shift\ndone\ncat >/dev/null\nprintf '%s' {} > \"$out\"\n", serde_json::to_string(result).unwrap())).unwrap();
    fs::set_permissions(&codex_script, fs::Permissions::from_mode(0o755)).unwrap();
    for (name, binary) in [
        ("claude-code", &claude_script),
        ("codex-cli", &codex_script),
    ] {
        let config = ProviderConfig {
            name: Some(name.into()),
            claude_code_bin: (name == "claude-code").then(|| binary.display().to_string()),
            codex_bin: (name == "codex-cli").then(|| binary.display().to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_ok(), "model may use CLI default");
        let provider = ConfiguredProvider { config };
        let outcome = provider
            .review_context(&rule(), &target(), &[], "target", 32_768)
            .unwrap();
        assert_eq!(outcome.findings.len(), 1);
        assert_eq!(
            outcome.findings[0].evidence_spans[0].quote,
            "measured response"
        );
    }

    let failing_script = dir.path().join("claude-fail");
    fs::write(&failing_script, "#!/bin/sh\ncat >/dev/null\nexit 7\n").unwrap();
    fs::set_permissions(&failing_script, fs::Permissions::from_mode(0o755)).unwrap();
    let failed = ConfiguredProvider {
        config: ProviderConfig {
            name: Some("claude-code".into()),
            claude_code_bin: Some(failing_script.display().to_string()),
            ..Default::default()
        },
    }
    .review_context(&rule(), &target(), &[], "target", 32_768)
    .unwrap_err();
    assert_eq!(failed.kind, "provider_error");
    assert!(failed.message.contains("status 7"));
}

#[test]
fn openai_compatible_and_anthropic_responses_are_parsed() {
    for name in ["openai", "anthropic", "openrouter", "ollama"] {
        let result = json!({"status":"violation", "evidence":"caption", "explanation":"Needs units", "confidence":0.8, "suggestion":"Add units"}).to_string();
        let body = if name == "anthropic" {
            json!({"content":[{"type":"text", "text":result}]})
        } else {
            json!({"choices":[{"message":{"content":result}}]})
        };
        let (endpoint, server) = mock(200, body.to_string());
        let finding = provider(name, endpoint)
            .review(&rule(), &target())
            .unwrap()
            .unwrap();
        assert_eq!(finding.status, "violation");
        assert_eq!(finding.evidence, "caption");
        let request = server.join().unwrap();
        assert_eq!(request["model"], "test-model");
        assert!(
            request["messages"][0]["content"]
                .as_str()
                .unwrap()
                .contains("Caption: measured response")
        );
    }
}

#[test]
fn malformed_semantic_results_are_issues_even_for_pass() {
    for content in [
        "not JSON".to_string(),
        "{}".into(),
        json!({"status":"unexpected", "evidence":"e", "explanation":"x"}).to_string(),
        json!({"status":"violation", "evidence":"e", "explanation":"x", "confidence":1.5})
            .to_string(),
        json!({"status":"violation", "evidence":"e", "explanation":"x", "confidence":1.00000001})
            .to_string(),
        json!({"status":"pass", "evidence":"", "explanation":""}).to_string(),
    ] {
        let (endpoint, server) = mock(
            200,
            json!({"choices":[{"message":{"content":content}}]}).to_string(),
        );
        let error = provider("openai", endpoint)
            .review(&rule(), &target())
            .unwrap_err();
        assert_eq!(error.kind, "provider_response");
        server.join().unwrap();
    }
}

#[test]
fn pass_produces_no_finding_and_http_failures_preserve_deterministic_results() {
    let content =
        json!({"status":"pass", "evidence":"caption has units", "explanation":"Rule met"})
            .to_string();
    let (endpoint, server) = mock(
        200,
        json!({"choices":[{"message":{"content":content}}]}).to_string(),
    );
    assert!(
        provider("openai", endpoint)
            .review(&rule(), &target())
            .unwrap()
            .is_none()
    );
    server.join().unwrap();
    for status in [429, 500] {
        let (endpoint, server) = mock(status, "{}".into());
        let deterministic = serde_yaml::from_str(
            "id: literal\nscope: figure\nkind: text\ncheck: {type: forbid, pattern: Caption}",
        )
        .unwrap();
        let registry = RuleRegistry {
            active: vec![deterministic, rule()],
            ..Default::default()
        };
        let (findings, issues) =
            review::run(&[target()], &registry, Some(&provider("openai", endpoint)));
        assert_eq!(findings.len(), 1);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, "provider_error");
        assert!(!issues[0].message.contains("test-secret"));
        server.join().unwrap();
    }
}

#[test]
fn findings_receive_unique_ids_across_targets_and_rule_kinds() {
    struct Fake;
    impl SemanticProvider for Fake {
        fn can_handle(&self, _: &RuleDefinition) -> bool {
            true
        }
        fn review(
            &self,
            rule: &RuleDefinition,
            target: &DocumentTarget,
        ) -> Result<Option<advisor_review::model::ReviewFinding>, advisor_review::model::ReviewIssue>
        {
            Ok(Some(advisor_review::model::ReviewFinding {
                id: "same".into(),
                rule_id: rule.id.clone(),
                target: target.clone(),
                ..Default::default()
            }))
        }
    }
    let deterministic = serde_yaml::from_str(
        "id: literal\nscope: figure\nkind: text\ncheck: {type: forbid, pattern: Caption}",
    )
    .unwrap();
    let registry = RuleRegistry {
        active: vec![deterministic, rule()],
        ..Default::default()
    };
    let (findings, _) = review::run(
        &[
            target(),
            DocumentTarget {
                id: "figure-2".into(),
                ..target()
            },
        ],
        &registry,
        Some(&Fake),
    );
    assert_eq!(findings.len(), 4);
    assert_eq!(
        findings
            .iter()
            .map(|f| &f.id)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        4
    );
}

#[test]
fn contextual_requests_return_multiple_findings_with_validated_source_evidence() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("main.tex"),
        "\\section{Results}\nIt improved. The measured gain was 5%.",
    )
    .unwrap();
    let targets =
        advisor_review::latex::parse_project(&dir.path().join("main.tex"), dir.path()).unwrap();
    let sentence = targets
        .iter()
        .find(|t| t.target_type == "sentence")
        .unwrap();
    let paragraph = targets
        .iter()
        .find(|t| t.target_type == "paragraph")
        .unwrap();
    let rule: RuleDefinition = serde_yaml::from_str("id: clarity\nscope: sentence\ncontext: paragraph\nkind: semantic-text\ndescription: Check clarity.\ncheck: {type: semantic}").unwrap();
    let body = json!({"summary":"Two clarity issues", "findings":[
        {"status":"concern", "explanation":"Unclear referent", "evidence":[{"target_id":sentence.id,"quote":"It","start":0}]},
        {"status":"suggestion", "explanation":"Specify what improved", "evidence":[{"target_id":paragraph.id,"quote":"measured gain"}]}
    ]}).to_string();
    for name in ["openai", "anthropic", "openrouter", "ollama"] {
        let envelope = if name == "anthropic" {
            json!({"content":[{"text":body}]})
        } else {
            json!({"choices":[{"message":{"content":body}}]})
        };
        let (endpoint, server) = mock(200, envelope.to_string());
        let result = provider(name, endpoint)
            .review_context(&rule, sentence, &[paragraph.clone()], "direct", 32768)
            .unwrap();
        assert_eq!(result.findings.len(), 2);
        assert!(result.findings.iter().all(|f| f.target.id == sentence.id));
        assert_eq!(
            result.findings[0].evidence_spans[0].sources[0].start_line,
            2
        );
        let request = server.join().unwrap();
        let prompt = request["messages"][0]["content"].as_str().unwrap();
        let payload: Value = serde_json::from_str(prompt.split_once('\n').unwrap().1).unwrap();
        assert_eq!(payload["target"]["text"], sentence.text);
        assert_eq!(payload["context"][0]["text"], paragraph.text);
    }
}

#[test]
fn contextual_evidence_rejects_unknown_ids_hallucinations_and_bad_offsets() {
    for evidence in [
        json!({"target_id":"unknown","quote":"Caption"}),
        json!({"target_id":"figure-1","quote":"not in text"}),
        json!({"target_id":"figure-1","quote":"Caption","start":1}),
        json!({"target_id":"figure-1","quote":""}),
    ] {
        let content = json!({"summary":"Issue", "findings":[{"status":"concern","explanation":"Problem","evidence":[evidence]}]}).to_string();
        let (endpoint, server) = mock(
            200,
            json!({"choices":[{"message":{"content":content}}]}).to_string(),
        );
        assert_eq!(
            provider("openai", endpoint)
                .review_context(&rule(), &target(), &[], "direct", 32768)
                .unwrap_err()
                .kind,
            "provider_response"
        );
        server.join().unwrap();
    }
}

#[test]
fn overlapping_quotes_require_offsets_and_unicode_offsets_are_checked() {
    for (text, quote, start, accepted) in [
        ("aaa", "aa", None, false),
        ("aaa", "aa", Some(1), true),
        ("ééé", "éé", None, false),
        ("ééé", "éé", Some(1), false),
        ("ééé", "éé", Some(2), true),
    ] {
        let mut cited_target = target();
        cited_target.text = text.into();
        let evidence = json!({"target_id":cited_target.id,"quote":quote,"start":start});
        let content = json!({"summary":"Issue", "findings":[{"status":"concern","explanation":"Problem","evidence":[evidence]}]}).to_string();
        let (endpoint, server) = mock(
            200,
            json!({"choices":[{"message":{"content":content}}]}).to_string(),
        );
        let result = provider("openai", endpoint).review_context(
            &rule(),
            &cited_target,
            &[],
            "direct",
            32768,
        );
        server.join().unwrap();
        assert_eq!(result.is_ok(), accepted, "text={text:?}, start={start:?}");
    }
}

#[test]
fn contextual_budget_is_checked_before_network_and_empty_findings_pass() {
    let p = provider("openai", "http://127.0.0.1:1".into());
    assert_eq!(
        p.review_context(&rule(), &target(), &[], "direct", 1)
            .unwrap_err()
            .kind,
        "budget"
    );
    let content = json!({"summary":"Rule met", "findings":[]}).to_string();
    let (endpoint, server) = mock(
        200,
        json!({"choices":[{"message":{"content":content}}]}).to_string(),
    );
    assert!(
        provider("openai", endpoint)
            .review_context(&rule(), &target(), &[], "direct", 32768)
            .unwrap()
            .findings
            .is_empty()
    );
    server.join().unwrap();
}
