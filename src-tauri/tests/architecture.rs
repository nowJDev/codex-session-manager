// 헥사고날 아키텍처 경계가 다시 흐려지지 않도록 소스 의존성을 검증한다.
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri parent")
        .to_path_buf()
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_root().join(path)).expect(path)
}

#[test]
fn tauri_commands_do_not_call_outbound_infrastructure_directly() {
    let source = read("src-tauri/src/adapters/inbound/tauri_commands.rs");
    for forbidden in [
        "crate::cloud::",
        "crate::environment::",
        "crate::codex_status::",
        "crate::update::",
        "crate::debuglog::",
        "std::fs::",
        "std::process::Command",
    ] {
        assert!(
            !source.contains(forbidden),
            "inbound adapter should delegate through application services, found {forbidden}"
        );
    }
}

#[test]
fn domain_transcript_does_not_depend_on_json_transport_shape() {
    let source = read("src-tauri/src/domain/transcript.rs");
    assert!(
        !source.contains("serde_json"),
        "domain transcript rules should not import serde_json::Value"
    );
    assert!(
        !source.contains("Value"),
        "domain transcript rules should use internal transcript types, not transport values"
    );
}

#[test]
fn frontend_app_does_not_orchestrate_ipc_workflows_directly() {
    let source = read("src/App.tsx");
    assert!(
        !source.contains("ipc."),
        "App.tsx should delegate session workflows to a frontend application hook/service"
    );
}
