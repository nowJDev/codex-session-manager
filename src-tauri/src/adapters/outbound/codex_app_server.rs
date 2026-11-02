// Codex app-server의 표준 입출력 RPC와 등록 세션 조회를 담당한다.
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::runtime::Runtime;

#[derive(Debug, Deserialize)]
pub(crate) struct RpcError {
    pub(crate) code: i64,
    pub(crate) message: String,
    #[cfg(test)]
    pub(crate) data: Option<Value>,
}
impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "app-server RPC {}: {}", self.code, self.message)
    }
}
impl std::error::Error for RpcError {}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisteredThread {
    pub(crate) id: String,
    pub(crate) parent_thread_id: Option<String>,
    pub(crate) agent_nickname: Option<String>,
    pub(crate) cwd: String,
    pub(crate) path: Option<String>,
    #[serde(default = "legacy_history_mode")]
    pub(crate) history_mode: String,
    #[serde(default)]
    pub(crate) preview: String,
    pub(crate) name: Option<String>,
    pub(crate) cli_version: String,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
    pub(crate) source: Value,
    pub(crate) ephemeral: bool,
    #[serde(skip)]
    pub(crate) archived: bool,
}

fn legacy_history_mode() -> String {
    "legacy".into()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page<T> {
    data: Vec<T>,
    next_cursor: Option<String>,
}

pub(crate) struct AppServer {
    runtime: Runtime,
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: Lines<BufReader<ChildStdout>>,
    stderr: Arc<Mutex<VecDeque<u8>>>,
    stderr_task: tokio::task::JoinHandle<()>,
    next_id: u64,
    request_timeout: Duration,
    poisoned: bool,
}

impl AppServer {
    pub(crate) fn connect() -> Result<Self> {
        let cli = crate::environment::locate_codex()
            .ok_or_else(|| anyhow!("codex CLI를 찾을 수 없음"))?;
        let mut command = Command::new(cli);
        command.args(["app-server", "--stdio"]);
        Self::from_command(
            command,
            &crate::scanner::codex_home(),
            Duration::from_secs(20),
            Duration::from_secs(30),
        )
    }

    fn from_command(
        mut command: Command,
        home: &Path,
        startup_timeout: Duration,
        request_timeout: Duration,
    ) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()?;
        command
            .env("CODEX_HOME", home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x0800_0000);
        let mut child = {
            let _guard = runtime.enter();
            command.spawn().context("codex app-server 실행 실패")?
        };
        let stdin = child.stdin.take().context("app-server stdin missing")?;
        let stdout =
            BufReader::new(child.stdout.take().context("app-server stdout missing")?).lines();
        let mut stderr_reader = child.stderr.take().context("app-server stderr missing")?;
        let stderr = Arc::new(Mutex::new(VecDeque::new()));
        let stderr_tail = Arc::clone(&stderr);
        let stderr_task = runtime.spawn(async move {
            let mut chunk = [0_u8; 4096];
            loop {
                match stderr_reader.read(&mut chunk).await {
                    Ok(0) | Err(_) => break,
                    Ok(count) => {
                        let mut tail = stderr_tail.lock().unwrap_or_else(|e| e.into_inner());
                        tail.extend(&chunk[..count]);
                        let excess = tail.len().saturating_sub(8192);
                        tail.drain(..excess);
                    }
                }
            }
        });
        let mut server = Self {
            runtime,
            child,
            stdin: Some(stdin),
            stdout,
            stderr,
            stderr_task,
            next_id: 1,
            request_timeout: startup_timeout,
            poisoned: false,
        };
        server.request(
            "initialize",
            json!({
                "clientInfo": {"name":"codex-session-manager", "version":env!("CARGO_PKG_VERSION")},
                "capabilities": {"experimentalApi":true}
            }),
        )?;
        let bytes = b"{\"method\":\"initialized\"}\n";
        server.runtime.block_on(async {
            tokio::time::timeout(startup_timeout, async {
                let stdin = server.stdin.as_mut().context("app-server stdin closed")?;
                stdin.write_all(bytes).await?;
                stdin.flush().await?;
                Ok::<(), anyhow::Error>(())
            })
            .await
            .map_err(|_| anyhow!("app-server initialized timeout"))?
        })?;
        server.request_timeout = request_timeout;
        Ok(server)
    }

    pub(crate) fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        if self.poisoned {
            return Err(anyhow!(
                "app-server transport is unusable after an earlier failure"
            ));
        }
        let id = self.next_id;
        self.next_id += 1;
        let mut bytes = serde_json::to_vec(&json!({"id":id,"method":method,"params":params}))?;
        bytes.push(b'\n');
        let result = self.runtime.block_on(async {
            tokio::time::timeout(self.request_timeout, async {
                let stdin = self.stdin.as_mut().context("app-server stdin closed")?;
                stdin
                    .write_all(&bytes)
                    .await
                    .context("app-server request write failed")?;
                stdin.flush().await?;
                loop {
                    let line = self
                        .stdout
                        .next_line()
                        .await?
                        .context("app-server EOF before response")?;
                    let response: Value = serde_json::from_str(&line)
                        .context("malformed app-server JSON response")?;
                    if response.get("id").is_none()
                        && response.get("method").and_then(Value::as_str).is_some()
                    {
                        continue;
                    }
                    if response.get("id") != Some(&json!(id)) {
                        return Err(anyhow!("app-server response id mismatch"));
                    }
                    if response.get("result").is_some() == response.get("error").is_some() {
                        return Err(anyhow!(
                            "malformed app-server response: expected result or error"
                        ));
                    }
                    if let Some(error) = response.get("error") {
                        return Err(serde_json::from_value::<RpcError>(error.clone())
                            .context("malformed app-server RPC error")?
                            .into());
                    }
                    return Ok(response["result"].clone());
                }
            })
            .await
            .map_err(|_| anyhow!("app-server {method} timeout"))?
        });
        if let Err(error) = result {
            if error.downcast_ref::<RpcError>().is_none() {
                self.poisoned = true;
            }
            let tail: Vec<u8> = self
                .stderr
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .copied()
                .collect();
            if tail.is_empty() {
                return Err(error);
            }
            let context = format!(
                "{error}; app-server stderr: {}",
                String::from_utf8_lossy(&tail)
            );
            return Err(error.context(context));
        }
        result
    }

    pub(crate) fn list_threads(&mut self) -> Result<Vec<RegisteredThread>> {
        let mut threads = Vec::new();
        let mut ids = HashSet::new();
        for archived in [false, true] {
            let mut cursor = None;
            let mut cursors = HashSet::new();
            loop {
                let response = self.request("thread/list", json!({
                    "archived":archived, "cursor":cursor, "limit":100,
                    "sortKey":"created_at", "sortDirection":"desc", "modelProviders":[],
                    "useStateDbOnly":true,
                    "sourceKinds":["cli","vscode","exec","appServer","subAgent","subAgentReview","subAgentCompact","subAgentThreadSpawn","subAgentOther","unknown"]
                }))?;
                let page: Page<RegisteredThread> =
                    serde_json::from_value(response).context("malformed thread/list page")?;
                for mut thread in page.data {
                    thread.archived = archived;
                    if ids.insert(thread.id.clone()) {
                        threads.push(thread);
                    }
                }
                match page.next_cursor {
                    Some(next) => {
                        if !cursors.insert(next.clone()) {
                            return Err(anyhow!("thread/list repeated pagination cursor"));
                        }
                        cursor = Some(next);
                    }
                    None => break,
                }
            }
        }
        Ok(threads)
    }

    pub(crate) fn read_excerpt(&mut self, thread_id: &str) -> Result<String> {
        let first: Page<Value> = serde_json::from_value(self.request(
            "thread/items/list",
            json!({
                "threadId":thread_id, "limit":100, "sortDirection":"asc"
            }),
        )?)
        .context("malformed thread/items/list page")?;
        let mut seen = HashSet::new();
        let head = conversation_text(first.data, &mut seen)?;
        let tail = if first.next_cursor.is_none() {
            if head.chars().count() <= 12000 {
                return Ok(head);
            }
            head.clone()
        } else {
            let mut last: Page<Value> = serde_json::from_value(self.request(
                "thread/items/list",
                json!({
                    "threadId":thread_id, "limit":100, "sortDirection":"desc"
                }),
            )?)
            .context("malformed thread/items/list page")?;
            last.data.reverse();
            conversation_text(last.data, &mut seen)?
        };
        let head: String = head.chars().take(5999).collect();
        let tail: String = tail
            .chars()
            .rev()
            .take(5999)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        Ok(format!("{head}\n\n{tail}"))
    }
}

fn conversation_text(entries: Vec<Value>, seen: &mut HashSet<(String, String)>) -> Result<String> {
    let mut messages = Vec::new();
    for entry in entries {
        let item = entry
            .get("item")
            .and_then(Value::as_object)
            .context("malformed thread item entry")?;
        let text = match item.get("type").and_then(Value::as_str) {
            Some("userMessage") => {
                let content = item
                    .get("content")
                    .and_then(Value::as_array)
                    .context("malformed userMessage content")?;
                let mut parts = Vec::new();
                for part in content {
                    if part.get("type").and_then(Value::as_str) == Some("text") {
                        parts.push(
                            part.get("text")
                                .and_then(Value::as_str)
                                .context("malformed userMessage text")?,
                        );
                    }
                }
                parts.join("\n")
            }
            Some("agentMessage") => item
                .get("text")
                .and_then(Value::as_str)
                .context("malformed agentMessage text")?
                .to_owned(),
            Some(_) => continue,
            None => return Err(anyhow!("malformed thread item type")),
        };
        if text.trim().is_empty()
            || crate::domain::transcript::is_injected_instruction_message(&text)
        {
            continue;
        }
        let id = item
            .get("id")
            .and_then(Value::as_str)
            .context("malformed thread item id")?;
        let turn = entry
            .get("turnId")
            .and_then(Value::as_str)
            .context("malformed thread item turnId")?;
        if seen.insert((turn.to_owned(), id.to_owned())) {
            messages.push(text);
        }
    }
    Ok(messages.join("\n\n"))
}

impl Drop for AppServer {
    fn drop(&mut self) {
        self.stdin.take();
        self.runtime.block_on(async {
            if !matches!(
                tokio::time::timeout(Duration::from_millis(500), self.child.wait()).await,
                Ok(Ok(_))
            ) {
                #[cfg(windows)]
                if let (Some(pid), Some(system_root)) =
                    (self.child.id(), std::env::var_os("SystemRoot"))
                {
                    // npm의 cmd shim 손자가 파이프를 유지하면 런타임 종료도 지연된다.
                    let mut taskkill =
                        Command::new(Path::new(&system_root).join("System32/taskkill.exe"));
                    taskkill
                        .args(["/PID", &pid.to_string(), "/T", "/F"])
                        .creation_flags(0x0800_0000)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .kill_on_drop(true);
                    let _ = tokio::time::timeout(Duration::from_secs(2), taskkill.status()).await;
                }
                let _ = self.child.start_kill();
                let _ = tokio::time::timeout(Duration::from_secs(2), self.child.wait()).await;
            }
        });
        self.stderr_task.abort();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[cfg(windows)]
    fn fixture(steps: Value, timeout: std::time::Duration) -> (tempfile::TempDir, AppServer) {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("server.ps1");
        let script_text = format!(
            r#"$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = New-Object System.Text.UTF8Encoding($false)
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$steps = ConvertFrom-Json @'
{steps}
'@
$log = Join-Path $env:CODEX_HOME 'requests.jsonl'
foreach ($step in $steps) {{
  $line = [Console]::ReadLine()
  Add-Content -LiteralPath $log -Value $line -Encoding UTF8
  $request = ConvertFrom-Json $line
  if ($request.method -ne $step.method) {{ throw "Unexpected method $($request.method)" }}
  if ($step.method -eq 'initialized') {{ continue }}
  if ($step.delay) {{ Start-Sleep -Milliseconds $step.delay }}
  if ($step.stderr) {{ [Console]::Error.Write($step.stderr) }}
  if ($step.exit) {{ exit 0 }}
  if ($step.raw) {{ [Console]::WriteLine($step.raw); continue }}
  [Console]::WriteLine('{{"method":"notice","params":{{}}}}')
  if ($step.error) {{ $response = @{{ id=$request.id; error=$step.error }} }}
  elseif ($step.echo) {{ $response = @{{ id=$request.id; result=$request.params }} }}
  else {{ $response = @{{ id=$request.id; result=$step.result }} }}
  [Console]::WriteLine(($response | ConvertTo-Json -Depth 40 -Compress))
}}
while ([Console]::ReadLine()) {{ }}
Set-Content -LiteralPath (Join-Path $env:CODEX_HOME 'closed') -Value 'yes'
"#
        );
        std::fs::write(&script, format!("\u{feff}{script_text}")).unwrap();
        let mut command = tokio::process::Command::new("powershell.exe");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ]);
        command.arg(script);
        let server = AppServer::from_command(
            command,
            dir.path(),
            std::time::Duration::from_secs(5),
            timeout,
        )
        .unwrap();
        (dir, server)
    }

    fn steps(mut requests: Vec<Value>) -> Value {
        let mut steps = vec![
            json!({"method":"initialize", "result":{}}),
            json!({"method":"initialized"}),
        ];
        steps.append(&mut requests);
        Value::Array(steps)
    }

    fn thread(id: &str) -> Value {
        json!({"id":id,"parentThreadId":null,"agentNickname":null,"cwd":"C:/project", "path":null,
          "cliVersion":"0.166.0","createdAt":1,"updatedAt":2,"source":"cli","ephemeral":false})
    }

    #[test]
    #[cfg(windows)]
    fn handshake_request_ids_notifications_and_home_are_preserved() {
        let (dir, mut server) = fixture(
            steps(vec![json!({"method":"echo", "echo":true})]),
            std::time::Duration::from_secs(5),
        );
        assert_eq!(
            server.request("echo", json!({"hello":"한글"})).unwrap(),
            json!({"hello":"한글"})
        );
        drop(server);
        let log = std::fs::read_to_string(dir.path().join("requests.jsonl")).unwrap();
        let requests: Vec<Value> = log
            .lines()
            .map(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).unwrap())
            .collect();
        assert_eq!(requests[0]["method"], "initialize");
        assert_eq!(requests[1]["method"], "initialized");
        assert_ne!(requests[0]["id"], requests[2]["id"]);
        assert!(dir.path().join("closed").exists());
    }

    #[test]
    #[cfg(windows)]
    fn lists_all_pages_and_archives_with_explicit_db_only_sources() {
        let (dir, mut server) = fixture(
            steps(vec![
                json!({"method":"thread/list","result":{"data":[thread("a")],"nextCursor":"one"}}),
                json!({"method":"thread/list","result":{"data":[],"nextCursor":"two"}}),
                json!({"method":"thread/list","result":{"data":[thread("b")],"nextCursor":null}}),
                json!({"method":"thread/list","result":{"data":[thread("a"),thread("c")],"nextCursor":null}}),
            ]),
            std::time::Duration::from_secs(5),
        );
        let threads = server.list_threads().unwrap();
        assert_eq!(
            threads.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
        assert!(!threads[0].archived);
        assert!(threads[2].archived);
        assert_eq!(threads[0].history_mode, "legacy");
        drop(server);
        let log = std::fs::read_to_string(dir.path().join("requests.jsonl")).unwrap();
        let requests: Vec<Value> = log
            .lines()
            .skip(2)
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        for request in &requests {
            assert_eq!(request["params"]["useStateDbOnly"], true);
            assert_eq!(
                request["params"]["sourceKinds"].as_array().unwrap().len(),
                10
            );
            assert_eq!(request["params"]["limit"], 100);
            assert_eq!(request["params"]["sortKey"], "created_at");
        }
        assert_eq!(requests[2]["params"]["cursor"], "two");
        assert_eq!(requests[3]["params"]["archived"], true);
    }

    #[test]
    #[cfg(windows)]
    fn mid_page_rpc_failure_is_not_partial_success() {
        let (_dir, mut server) = fixture(
            steps(vec![
                json!({"method":"thread/list","result":{"data":[thread("a")],"nextCursor":"one"}}),
                json!({"method":"thread/list","error":{"code":-32000,"message":"database unavailable"}}),
            ]),
            std::time::Duration::from_secs(5),
        );
        let error = server.list_threads().unwrap_err();
        assert_eq!(error.downcast_ref::<RpcError>().unwrap().code, -32000);
    }

    #[test]
    #[cfg(windows)]
    fn repeated_cursor_is_an_error() {
        let (_dir, mut server) = fixture(
            steps(vec![
                json!({"method":"thread/list","result":{"data":[],"nextCursor":"one"}}),
                json!({"method":"thread/list","result":{"data":[],"nextCursor":"one"}}),
            ]),
            std::time::Duration::from_secs(5),
        );
        assert!(server
            .list_threads()
            .unwrap_err()
            .to_string()
            .contains("cursor"));
    }

    #[test]
    #[cfg(windows)]
    fn malformed_response_and_timeout_poison_the_transport() {
        let (_dir, mut server) = fixture(
            steps(vec![json!({"method":"bad","raw":"not json"})]),
            std::time::Duration::from_secs(5),
        );
        assert!(server.request("bad", json!({})).is_err());
        assert!(server.request("again", json!({})).is_err());
        let (_dir, mut server) = fixture(
            steps(vec![json!({"method":"slow","delay":1000,"result":{}})]),
            std::time::Duration::from_millis(50),
        );
        assert!(server
            .request("slow", json!({}))
            .unwrap_err()
            .to_string()
            .contains("timeout"));
    }

    #[test]
    #[cfg(windows)]
    fn stderr_tail_is_bounded_and_preserves_the_rpc_failure_message() {
        let (_dir, mut server) = fixture(
            steps(vec![json!({
                "method":"bad", "stderr":"x".repeat(20000),
                "error":{"code":-32000,"message":"database unavailable","data":{"reason":"busy"}}
            })]),
            std::time::Duration::from_secs(5),
        );
        let error = server.request("bad", json!({})).unwrap_err();
        assert_eq!(
            error
                .downcast_ref::<RpcError>()
                .unwrap()
                .data
                .as_ref()
                .unwrap()["reason"],
            "busy"
        );
        assert!(error.to_string().contains("database unavailable"));
        assert!(server.stderr.lock().unwrap().len() <= 8192);
    }

    #[test]
    #[cfg(windows)]
    fn eof_is_reported_without_waiting_for_the_request_deadline() {
        let (_dir, mut server) = fixture(
            steps(vec![json!({"method":"gone", "exit":true})]),
            std::time::Duration::from_secs(5),
        );
        let start = std::time::Instant::now();
        assert!(server
            .request("gone", json!({}))
            .unwrap_err()
            .to_string()
            .contains("EOF"));
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
    }

    #[test]
    #[cfg(windows)]
    fn initialization_timeout_reaps_a_child_that_ignores_stdin() {
        let dir = tempfile::tempdir().unwrap();
        let mut command = Command::new("powershell.exe");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 30",
        ]);
        let start = std::time::Instant::now();
        let error = match AppServer::from_command(
            command,
            dir.path(),
            Duration::from_millis(50),
            Duration::from_secs(5),
        ) {
            Ok(_) => panic!("unresponsive child unexpectedly initialized"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("timeout"));
        assert!(start.elapsed() < Duration::from_secs(3));
    }

    #[test]
    #[cfg(windows)]
    fn cmd_shim_timeout_stops_the_spawned_grandchild() {
        use std::os::windows::process::CommandExt;
        let dir = tempfile::Builder::new()
            .prefix("csm shim ")
            .tempdir()
            .unwrap();
        std::fs::write(dir.path().join("shim.cmd"), "@echo off\r\npowershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File \"%~dp0worker.ps1\"\r\n").unwrap();
        std::fs::write(
            dir.path().join("worker.ps1"),
            r#"
$ErrorActionPreference = 'Stop'
[IO.File]::WriteAllText((Join-Path $env:CODEX_HOME 'worker.pid'), [string]$PID)
for ($i = 0; $i -lt 120; $i++) {
    [IO.File]::WriteAllText((Join-Path $env:CODEX_HOME 'heartbeat'), [string]$i)
    Start-Sleep -Milliseconds 50
}
"#,
        )
        .unwrap();
        let start = std::time::Instant::now();
        let result = AppServer::from_command(
            Command::new(dir.path().join("shim.cmd")),
            dir.path(),
            Duration::from_millis(1500),
            Duration::from_secs(5),
        );
        assert!(result.is_err());
        let elapsed = start.elapsed();
        let pid: u32 = std::fs::read_to_string(dir.path().join("worker.pid"))
            .unwrap()
            .parse()
            .unwrap();
        let heartbeat = std::fs::read(dir.path().join("heartbeat")).unwrap();
        std::thread::sleep(Duration::from_millis(250));
        let still_writing = heartbeat != std::fs::read(dir.path().join("heartbeat")).unwrap();
        let system32 = Path::new(&std::env::var_os("SystemRoot").unwrap()).join("System32");
        let output = std::process::Command::new(system32.join("tasklist.exe"))
            .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
            .creation_flags(0x0800_0000)
            .output()
            .unwrap();
        let still_alive = String::from_utf8_lossy(&output.stdout).contains(&format!("\"{pid}\""));
        if still_alive {
            std::process::Command::new(system32.join("taskkill.exe"))
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .creation_flags(0x0800_0000)
                .output()
                .unwrap();
        }
        assert!(
            !still_alive && !still_writing,
            "timeout left the self-created shim grandchild running"
        );
        assert!(
            elapsed < Duration::from_secs(5),
            "shim grandchild kept the timed-out transport alive for {elapsed:?}"
        );
    }

    #[test]
    #[cfg(windows)]
    fn single_page_long_excerpt_preserves_conversation_start_and_end() {
        let (_dir, mut server) = fixture(
            steps(vec![json!({"method":"thread/items/list","result":{"data":[
                {"turnId":"t","item":{"id":"u","type":"userMessage","content":[{"type":"text","text":"시작 질문"}]}},
                {"turnId":"t","item":{"id":"a","type":"agentMessage","text":format!("{}대화 끝 표시", "답".repeat(15000))}}
            ],"nextCursor":null}})]),
            Duration::from_secs(5),
        );
        let excerpt = server.read_excerpt("thread").unwrap();
        assert!(excerpt.starts_with("시작 질문"));
        assert!(excerpt.ends_with("대화 끝 표시"));
        assert!(excerpt.chars().count() <= 12000);
    }

    #[test]
    #[cfg(windows)]
    fn excerpt_reads_only_conversation_text_and_bounds_unicode() {
        let (_dir, mut server) = fixture(
            steps(vec![
                json!({"method":"thread/items/list","result":{"data":[
              {"turnId":"t","item":{"id":"u0","type":"userMessage","content":[{"type":"text","text":"# AGENTS.md instructions\n<INSTRUCTIONS>hidden"}]}},
              {"turnId":"t","item":{"id":"u1","type":"userMessage","content":[{"type":"image","url":"hidden"},{"type":"text","text":"question"}]}},
              {"turnId":"t","item":{"id":"cmd","type":"commandExecution","output":"secret"}},
              {"turnId":"t","item":{"id":"a1","type":"agentMessage","text":"answer"}}
            ],"nextCursor":"more"}}),
                json!({"method":"thread/items/list","result":{"data":[
              {"turnId":"t","item":{"id":"a2","type":"agentMessage","text":"끝".repeat(15000)}},
              {"turnId":"t","item":{"id":"a1","type":"agentMessage","text":"answer"}}
            ],"nextCursor":null}}),
            ]),
            std::time::Duration::from_secs(5),
        );
        let excerpt = server.read_excerpt("thread").unwrap();
        assert!(excerpt.contains("question"));
        assert_eq!(excerpt.matches("answer").count(), 1);
        assert!(!excerpt.contains("hidden") && !excerpt.contains("secret"));
        assert!(excerpt.chars().count() <= 12000);
        assert!(excerpt.ends_with('끝'));
    }
}
