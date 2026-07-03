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
fn domain_session_does_not_depend_on_session_dto() {
    let source = read("src-tauri/src/domain/session.rs");
    assert!(
        !source.contains("crate::types::Session"),
        "domain session rules should not import transport/application Session DTO"
    );
    assert!(
        !source.contains("storage_type"),
        "domain session rules should return domain state instead of mutating DTO storage_type"
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

#[test]
fn cli_does_not_call_outbound_infrastructure_directly() {
    let source = read("src-tauri/src/bin/cli.rs");
    for forbidden in [
        "scanner::",
        "config::",
        "cloud::",
        "summary::",
        "resume::",
        "environment::",
        "terminal::",
    ] {
        assert!(
            !source.contains(forbidden),
            "CLI inbound adapter should delegate through application services, found {forbidden}"
        );
    }
}

#[test]
fn frontend_components_do_not_call_ipc_or_tauri_plugins_directly() {
    let components_dir = repo_root().join("src").join("components");
    for entry in fs::read_dir(components_dir).expect("components dir") {
        let path = entry.expect("component entry").path();
        if path.extension().and_then(|s| s.to_str()) != Some("tsx") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("component source");
        assert!(
            !source.contains("ipc."),
            "{} should delegate IPC workflows to frontend application services",
            path.display()
        );
        assert!(
            !source.contains("@tauri-apps"),
            "{} should receive external side effects through props/application services",
            path.display()
        );
    }
}

#[test]
fn frontend_components_do_not_access_browser_storage_directly() {
    let components_dir = repo_root().join("src").join("components");
    for entry in fs::read_dir(components_dir).expect("components dir") {
        let path = entry.expect("component entry").path();
        if path.extension().and_then(|s| s.to_str()) != Some("tsx") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("component source");
        assert!(
            !source.contains("localStorage"),
            "{} should delegate browser storage access to a frontend adapter/helper",
            path.display()
        );
    }
}

#[test]
fn frontend_session_application_is_split_by_workflow() {
    for required in [
        "src/application/useSessionData.ts",
        "src/application/useSessionSelection.ts",
        "src/application/useSessionCommands.ts",
    ] {
        assert!(
            repo_root().join(required).exists(),
            "frontend application workflow should be split into {required}"
        );
    }

    let source = read("src/application/useSessionManager.ts");
    for forbidden in [
        "async function handleResume",
        "async function confirmDelete",
        "async function handleToggleArchive",
        "async function handleToggleCloud",
        "async function handleGenerateSummary",
        "async function handleToggleFavorite",
    ] {
        assert!(
            !source.contains(forbidden),
            "useSessionManager should compose workflow hooks instead of owning command handlers: {forbidden}"
        );
    }
}

#[test]
fn frontend_application_hooks_do_not_import_tauri_runtime_directly() {
    let application_dir = repo_root().join("src").join("application");
    for entry in fs::read_dir(application_dir).expect("application dir") {
        let path = entry.expect("application entry").path();
        let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if ext != "ts" && ext != "tsx" {
            continue;
        }
        let source = fs::read_to_string(&path).expect("application source");
        for forbidden in ["@tauri-apps", "@/lib/ipc"] {
            assert!(
                !source.contains(forbidden),
                "{} should depend on a frontend gateway instead of concrete Tauri runtime imports: {forbidden}",
                path.display()
            );
        }
    }
}

#[test]
fn application_services_do_not_own_background_runtime_workers() {
    let source = read("src-tauri/src/application/auto_summary_service.rs");
    for forbidden in ["std::thread::spawn", "std::thread::sleep", "AtomicBool"] {
        assert!(
            !source.contains(forbidden),
            "application service should expose use cases, not runtime worker mechanics: {forbidden}"
        );
    }
}

#[test]
fn blocking_session_delete_commands_are_offloaded_from_ipc_thread() {
    let source = read("src-tauri/src/adapters/inbound/tauri_commands.rs");
    assert!(
        source.contains("pub async fn delete_session("),
        "delete_session should be async so blocking filesystem/CLI work does not freeze the window"
    );
    assert!(
        source.contains("pub async fn delete_sessions("),
        "delete_sessions should be async so bulk deletion does not freeze the window"
    );

    let delete_session_body = source
        .split("pub async fn delete_session(")
        .nth(1)
        .and_then(|tail| tail.split("#[tauri::command]").next())
        .unwrap_or("");
    assert!(
        delete_session_body.contains("tokio::task::spawn_blocking"),
        "delete_session should run blocking deletion work on a blocking thread"
    );

    let delete_sessions_body = source
        .split("pub async fn delete_sessions(")
        .nth(1)
        .and_then(|tail| tail.split("#[tauri::command]").next())
        .unwrap_or("");
    assert!(
        delete_sessions_body.contains("tokio::task::spawn_blocking"),
        "delete_sessions should run blocking bulk deletion work on a blocking thread"
    );
}

#[test]
fn cloud_adapter_does_not_persist_application_metadata_directly() {
    let source = read("src-tauri/src/cloud.rs");
    for forbidden in [
        "use crate::config::{load_config, upsert_session_meta}",
        "upsert_session_meta(",
        "crate::config::update_settings",
    ] {
        assert!(
            !source.contains(forbidden),
            "cloud adapter should only perform cloud file operations; application service should persist settings/meta: {forbidden}"
        );
    }
}

#[test]
fn infrastructure_modules_are_not_public_runtime_api() {
    let source = read("src-tauri/src/lib.rs");
    for forbidden in [
        "pub mod cloud;",
        "pub mod codex_status;",
        "pub mod config;",
        "pub mod debuglog;",
        "pub mod environment;",
        "pub mod resume;",
        "pub mod scanner;",
        "pub mod summary;",
        "pub mod terminal;",
        "pub mod update;",
    ] {
        assert!(
            !source.contains(forbidden),
            "runtime infrastructure should not be exposed as public API: {forbidden}"
        );
    }
}

#[test]
fn session_ports_are_split_by_external_capability() {
    let source = read("src-tauri/src/application/ports.rs");
    for required in [
        "pub trait SessionScanPort",
        "pub trait SessionMetadataPort",
        "pub trait CloudSyncPort",
        "pub trait ResumePort",
        "pub trait SummaryPort",
        "pub trait SessionMessagePort",
    ] {
        assert!(
            source.contains(required),
            "application ports should be split by capability: {required}"
        );
    }

    let aggregate = source
        .split("pub trait SessionPorts")
        .nth(1)
        .and_then(|tail| tail.split("pub trait System").next())
        .unwrap_or("");
    assert!(
        !aggregate.contains("fn "),
        "SessionPorts should compose capability traits instead of owning method declarations"
    );

    let scan_port = source
        .split("pub trait SessionScanPort")
        .nth(1)
        .and_then(|tail| tail.split("pub trait SessionCommandPort").next())
        .unwrap_or("");
    assert!(
        !scan_port.contains("get_session_messages"),
        "SessionScanPort should not own message retrieval capability"
    );

    let message_port = source
        .split("pub trait SessionMessagePort")
        .nth(1)
        .and_then(|tail| tail.split("pub trait SessionPorts").next())
        .unwrap_or("");
    assert!(
        message_port.contains("fn get_session_messages"),
        "SessionMessagePort should declare the message retrieval capability directly"
    );
}
