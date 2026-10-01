// 새로고침의 블로킹 작업이 Tauri IPC 스레드에서 실행되지 않는지 검증한다.
#[test]
fn refresh_commands_are_async() {
    let source = include_str!("../src/adapters/inbound/tauri_commands.rs");
    for command in [
        "list_sessions",
        "check_environment_cmd",
        "get_codex_status_cmd",
    ] {
        assert!(
            source.contains(&format!("pub async fn {command}(")),
            "{command} must not block the IPC thread"
        );
        let body = source
            .split(&format!("pub async fn {command}("))
            .nth(1)
            .unwrap()
            .split("#[tauri::command]")
            .next()
            .unwrap();
        assert!(
            body.contains("tokio::task::spawn_blocking"),
            "{command} must offload blocking work"
        );
    }
}

#[test]
fn listing_sessions_yields_while_the_blocking_worker_is_busy() {
    use codex_session_manager_lib::adapters::inbound::tauri_commands::list_sessions;
    use std::future::{poll_fn, Future};
    use std::task::Poll;
    use std::time::Duration;

    let home = tempfile::tempdir().unwrap();
    let previous_home = std::env::var_os("CODEX_HOME");
    let previous_session_home = std::env::var_os("CODEX_SESSION_HOME");
    std::env::set_var("CODEX_HOME", home.path().join(".codex"));
    std::env::set_var("CODEX_SESSION_HOME", home.path());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let blocker = runtime.spawn_blocking(move || {
        started_tx.send(()).unwrap();
        let _ = release_rx.recv_timeout(Duration::from_secs(5));
    });
    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();

    runtime.block_on(async {
        let mut listing = Box::pin(list_sessions());
        poll_fn(|cx| {
            assert!(
                listing.as_mut().poll(cx).is_pending(),
                "session scanning must wait on the worker rather than run on the executor"
            );
            Poll::Ready(())
        })
        .await;
        tokio::time::sleep(Duration::from_millis(1)).await;
        release_tx.send(()).unwrap();
        assert!(listing.await.unwrap().is_empty());
        blocker.await.unwrap();
    });
    for (key, previous) in [
        ("CODEX_HOME", previous_home),
        ("CODEX_SESSION_HOME", previous_session_home),
    ] {
        if let Some(value) = previous {
            std::env::set_var(key, value);
        } else {
            std::env::remove_var(key);
        }
    }
}
