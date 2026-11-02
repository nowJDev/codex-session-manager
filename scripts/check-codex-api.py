# 격리된 Codex 홈에서 실제 API의 DB 전용 조회와 검증된 삭제를 확인한다.
import argparse
import collections
import datetime
import json
import os
from pathlib import Path
import queue
import shutil
import sqlite3
import subprocess
import tempfile
import threading
import uuid


SOURCES = ["cli", "vscode", "exec", "appServer", "subAgent", "subAgentReview",
           "subAgentCompact", "subAgentThreadSpawn", "subAgentOther", "unknown"]


class RpcError(RuntimeError):
    pass


class AppServer:
    def __init__(self, codex, root, env):
        self.root = root
        self.process = subprocess.Popen(
            [str(codex), "app-server", "--stdio"], cwd=root, env=env,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, encoding="utf-8", creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        self.responses = queue.Queue()
        self.stderr = collections.deque(maxlen=20)
        self.next_id = 0

        def read_stdout():
            try:
                for line in self.process.stdout:
                    self.responses.put(line)
            finally:
                self.responses.put(None)

        def read_stderr():
            for line in self.process.stderr:
                self.stderr.append(line[-1000:])

        self.readers = [threading.Thread(target=read_stdout, daemon=True),
                        threading.Thread(target=read_stderr, daemon=True)]
        for reader in self.readers:
            reader.start()

    def __enter__(self):
        try:
            result = self.request("initialize", {
                "clientInfo": {"name": "csm-api-check", "version": "1"},
                "capabilities": {"experimentalApi": True},
            })
            assert Path(result["codexHome"]).resolve() == (self.root / ".codex").resolve()
            self.process.stdin.write('{"method":"initialized"}\n')
            self.process.stdin.flush()
            return self
        except BaseException:
            self.close()
            raise

    def request(self, method, params):
        assert method != "turn/start", "This check must never request model inference"
        self.next_id += 1
        self.process.stdin.write(json.dumps({"id": self.next_id, "method": method, "params": params}) + "\n")
        self.process.stdin.flush()
        while True:
            try:
                line = self.responses.get(timeout=30)
            except queue.Empty as error:
                raise RuntimeError(f"{method} timeout; {''.join(self.stderr)}") from error
            assert line is not None, f"{method} EOF; {''.join(self.stderr)}"
            response = json.loads(line)
            if "id" not in response and "method" in response:
                continue
            assert response.get("id") == self.next_id, response
            if "error" in response:
                raise RpcError(f"{method}: {response['error']}")
            return response["result"]

    def list_threads(self, db_only=True, limit=100):
        threads = {}
        for archived in (False, True):
            cursor = None
            seen = set()
            while True:
                page = self.request("thread/list", {
                    "archived": archived, "cursor": cursor, "limit": limit,
                    "sortKey": "created_at", "sortDirection": "desc",
                    "modelProviders": [], "sourceKinds": SOURCES, "useStateDbOnly": db_only,
                })
                for thread in page["data"]:
                    threads[thread["id"]] = {**thread, "archived": archived}
                cursor = page.get("nextCursor")
                if cursor is None:
                    break
                assert cursor not in seen, "Repeated thread/list cursor"
                seen.add(cursor)
        return threads

    def close(self):
        self.process.stdin.close()
        try:
            self.process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=5)
        for reader in self.readers:
            reader.join(timeout=2)
        self.process.stdout.close()
        self.process.stderr.close()

    def __exit__(self, *args):
        self.close()


def run(command, root, env):
    result = subprocess.run([str(part) for part in command], cwd=root, env=env,
                            capture_output=True, text=True, encoding="utf-8", timeout=90)
    assert result.returncode == 0, f"{command[1:]} failed: {result.stdout}\n{result.stderr}"
    return result.stdout


def fixture(root, thread_id, parent=None, offset=0):
    timestamp = (datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(seconds=100-offset)).isoformat(timespec="milliseconds").replace("+00:00", "Z")
    turn_id = str(uuid.uuid4())
    source = {"subagent": {"thread_spawn": {"parent_thread_id": parent, "depth": 1,
                                          "agent_nickname": "fixture child"}}} if parent else "cli"
    metadata = {"id": thread_id, "timestamp": timestamp, "cwd": str(root),
                "originator": "codex_cli_rs", "cli_version": "0.166.0",
                "source": source, "model_provider": "openai"}
    if parent:
        metadata["parent_thread_id"] = parent
    records = [
        {"type": "session_meta", "payload": metadata},
        {"type": "event_msg", "payload": {"type": "task_started", "turn_id": turn_id, "model_context_window": None}},
        {"type": "event_msg", "payload": {"type": "user_message", "message": "CSM API fixture question",
                                          "images": [], "local_images": [], "text_elements": []}},
        {"type": "response_item", "payload": {"type": "message", "role": "user",
                                              "content": [{"type": "input_text", "text": "CSM API fixture question"}]}},
        {"type": "response_item", "payload": {"type": "message", "role": "assistant",
                                              "content": [{"type": "output_text", "text": "CSM API fixture answer"}], "phase": "final_answer"}},
        {"type": "event_msg", "payload": {"type": "agent_message", "message": "CSM API fixture answer", "phase": "final_answer"}},
        {"type": "event_msg", "payload": {"type": "task_complete", "turn_id": turn_id,
                                          "last_agent_message": "CSM API fixture answer"}},
    ]
    folder = root / ".codex" / "sessions" / timestamp[:10].replace("-", "/")
    folder.mkdir(parents=True, exist_ok=True)
    path = folder / f"rollout-{timestamp[:19].replace(':', '-')}-{thread_id}.jsonl"
    path.write_text("".join(json.dumps({"timestamp": timestamp, **record}) + "\n" for record in records), encoding="utf-8")
    return path


def owned_path(root, path):
    resolved = path.resolve()
    if not root.name.startswith("csm-api-") or root.resolve() not in resolved.parents or path.is_symlink():
        raise ValueError(f"Refusing to change a path outside the generated fixture: {path}")
    return resolved


def clean(root):
    for path in sorted(root.rglob("*"), key=lambda item: len(item.parts), reverse=True):
        owned_path(root, path)
        if path.is_dir():
            path.rmdir()
        else:
            path.unlink()
    root.rmdir()


def migration_metadata(home, thread_id):
    database = sqlite3.connect(f"file:{(home / 'state_5.sqlite').as_posix()}?mode=ro", uri=True)
    try:
        row = database.execute(
            "SELECT history_mode, has_user_event, model_provider FROM threads WHERE id=?", (thread_id,)
        ).fetchone()
        assert row is not None and row[0] == "paginated", row
        metadata = {"history_mode": row[0], "has_user_event": row[1], "model_provider": row[2]}
    finally:
        database.close()
    history = sqlite3.connect(f"file:{(home / 'thread_history_1.sqlite').as_posix()}?mode=ro", uri=True)
    try:
        metadata["turns"] = history.execute("SELECT COUNT(*) FROM thread_turns WHERE thread_id=?", (thread_id,)).fetchone()[0]
        metadata["items"] = history.execute("SELECT COUNT(*) FROM thread_items WHERE thread_id=?", (thread_id,)).fetchone()[0]
        return metadata
    finally:
        history.close()


def main():
    if not __debug__:
        raise RuntimeError("Run this integration check without Python -O so assertions remain enabled")
    parser = argparse.ArgumentParser(description="Check real Codex APIs using disposable sessions without model inference")
    parser.add_argument("--codex", type=Path)
    parser.add_argument("--manager", type=Path, default=Path(__file__).resolve().parents[1] / "src-tauri/target/debug" / ("session-cli.exe" if os.name == "nt" else "session-cli"))
    args = parser.parse_args()
    codex = args.codex
    if codex is None and os.name == "nt":
        package = Path(os.environ["APPDATA"]) / "npm/node_modules/@openai/codex"
        codex = next(package.rglob("codex.exe"), None)
    codex = codex or Path(shutil.which("codex") or "codex")
    assert args.manager.is_file(), "Build session-cli or provide --manager"
    root = Path(tempfile.mkdtemp(prefix="csm-api-")).resolve()
    home = root / ".codex"
    home.mkdir()
    (home / "config.toml").write_text('model="gpt-5"\n[features]\nbackground_paginated_rollout_migration=false\n', encoding="utf-8")
    env = {**os.environ, "CODEX_HOME": str(home), "CODEX_SESSION_HOME": str(root), "CODEX_CLI": str(codex)}
    created = {name: str(uuid.uuid4()) for name in ("parent", "child", "archived", "legacy")}
    print(f"Isolated home: {home}", flush=True)
    try:
        print(run([codex, "--version"], root, env).strip(), flush=True)
        with AppServer(codex, root, env) as api:
            started = api.request("thread/start", {"cwd": str(root), "baseInstructions": "", "developerInstructions": "", "ephemeral": False})["thread"]
            assert Path(started["cwd"]).resolve() == root
            print(f"NOTE metadata-only thread/start historyMode={started['historyMode']}; registered={started['id'] in api.list_threads()}", flush=True)
        paths = {name: fixture(root, thread_id, created["parent"] if name == "child" else None, offset)
                 for offset, (name, thread_id) in enumerate(created.items())}
        with AppServer(codex, root, env) as api:
            registered = api.list_threads(db_only=False)
            assert set(created.values()).issubset(registered), registered
        command = [codex, "migrate-rollouts", "--apply", "--json"]
        for name in ("parent", "child", "archived"):
            command.extend(["--thread", created[name]])
        migration = json.loads(run(command, root, env))
        assert all(outcome["status"] == "migrated" for outcome in migration["outcomes"]), migration
        assert len(migration["outcomes"]) == 3, migration
        print("PASS official migration of three generated JSONL fixtures", flush=True)
        with AppServer(codex, root, env) as api:
            registered = api.list_threads()
            assert set(created.values()).issubset(registered), registered
            assert api.list_threads(limit=1).keys() == registered.keys(), "Real thread/list pagination lost a fixture"
            for name in ("parent", "archived"):
                items = api.request("thread/items/list", {"threadId": created[name], "limit": 100, "sortDirection": "asc"})
                text = json.dumps(items)
                assert "CSM API fixture question" in text and "CSM API fixture answer" in text, (name, items)
            migration_details = migration_metadata(home, created["parent"])
            assert migration_details["turns"] >= 1 and migration_details["items"] >= 2, migration_details
            api.request("thread/archive", {"threadId": created["archived"]})
            registered = api.list_threads()
            paths["archived"] = Path(registered[created["archived"]]["path"])
        parent_rollout = paths["parent"].read_bytes()
        owned_path(root, paths["parent"]).unlink()
        with AppServer(codex, root, env) as api:
            registered = api.list_threads()
            if created["parent"] not in registered:
                try:
                    api.request("thread/items/list", {"threadId": created["parent"], "limit": 100})
                    raise AssertionError("Unlisted migrated fixture unexpectedly has readable history")
                except RpcError as error:
                    assert "missing source rollout" in str(error), error
                print(f"LIMIT imported fixture needs its source rollout: {migration_details}", flush=True)
                print("LIMIT native JSONL-free manager deletion not exercised: imported history retains rollout lineage; model turns are disabled", flush=True)
                owned_path(root, paths["parent"]).write_bytes(parent_rollout)
                registered = api.list_threads()
            assert set(created.values()).issubset(registered), registered
            assert registered[created["archived"]]["archived"]
            assert registered[created["legacy"]]["historyMode"] == "legacy"
            for name in ("parent", "child", "archived"):
                thread_id = created[name]
                assert registered[thread_id]["historyMode"] == "paginated"
                metadata = api.request("thread/read", {"threadId": thread_id, "includeTurns": False})["thread"]
                assert metadata["id"] == thread_id
            legacy = api.request("thread/read", {"threadId": created["legacy"], "includeTurns": True})
            assert "CSM API fixture question" in json.dumps(legacy)
            print("PASS actual API pagination, archived listing, metadata, paginated items and legacy read", flush=True)
            listed = json.loads(run([args.manager, "list"], root, env))
            sessions = {session["sessionId"]: session for session in listed}
            for name in ("parent", "child", "archived"):
                session = sessions[created[name]]
                assert session["registered"] and session["historyMode"] == "paginated", session
                if not paths[name].exists():
                    assert session["filePath"] is None and session["size"] is None, session
            assert sessions[created["child"]]["parentId"] == created["parent"]
            print("PASS manager lists paginated sessions and child relationship", flush=True)
            for name, expected in (("archived", {created["archived"]}),
                                   ("parent", {created["parent"], created["child"]}),
                                   ("legacy", {created["legacy"]})):
                result = json.loads(run([args.manager, "delete", created[name]], root, env))
                assert {item["sessionId"] for item in result} == expected, result
                assert all(item["status"] == "deleted" for item in result), result
                assert not expected.intersection(api.list_threads()), "API still lists deleted fixture"
                retry = json.loads(run([args.manager, "delete", created[name]], root, env))
                assert all(item["status"] == "alreadyMissing" for item in retry), retry
                print(f"PASS manager verified delete + alreadyMissing retry: {name}", flush=True)
            assert not set(created.values()).intersection(api.list_threads())
        print("PASS isolated real-CLI API integration (no turn/start/model inference)", flush=True)
    finally:
        clean(root)
        print(f"Removed generated home: {root}", flush=True)


if __name__ == "__main__":
    main()
