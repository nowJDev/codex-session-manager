// 실제 삭제 훅의 부분 실패, 통신 실패와 중복 실행 방지를 검증한다.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

const source = readFileSync(new URL("../src/application/useSessionCommands.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
});

function setup(deleteSessions, refresh = async () => {}) {
  const slots = [];
  let cursor = 0;
  let refreshCount = 0;
  let calls = 0;
  let selectedId = "deleted";
  let selectedIds = new Set(["deleted", "missing", "failed", "unrelated"]);
  const useState = (initial) => {
    const index = cursor++;
    if (!(index in slots)) slots[index] = initial;
    return [slots[index], (value) => { slots[index] = typeof value === "function" ? value(slots[index]) : value; }];
  };
  const exports = {};
  vm.runInNewContext(outputText, {
    exports,
    require: (name) => name === "react"
      ? { useState, useRef: (initial) => useState({ current: initial })[0] }
      : { tauriGateway: { deleteSessions: async (targets) => { calls++; return deleteSessions(targets); } } },
    console: { error() {} },
    alert: () => assert.fail("Deletion must not open a blocking alert"),
  });
  const params = {
    refresh: async () => { refreshCount++; await refresh(); },
    setSessions() {},
    setSelectedId: (update) => { selectedId = update(selectedId); },
    setSelectedForDeleteIds: (update) => { selectedIds = update(selectedIds); },
    selectedForDelete: ["deleted", "missing", "failed"].map((sessionId) => ({ sessionId, filePath: `${sessionId}.jsonl` })),
  };
  const render = () => { cursor = 0; return exports.useSessionCommands(params); };
  render().handleBulkDelete();
  return {
    render,
    clearSelection: () => { params.selectedForDelete = []; selectedIds = new Set(); },
    selection: () => ({ selectedId, selectedIds }),
    refreshCount: () => refreshCount,
    calls: () => calls,
  };
}

const results = [
  { sessionId: "deleted", filePath: "deleted.jsonl", status: "deleted", error: null },
  { sessionId: "missing", filePath: "missing.jsonl", status: "alreadyMissing", error: null },
  { sessionId: "failed", filePath: "failed.jsonl", status: "failed", error: "Access denied" },
];
const failures = [];
async function check(name, run) {
  try { await run(); console.log(`PASS ${name}`); }
  catch (error) { failures.push(name); console.error(`FAIL ${name}: ${error.message}`); }
}

await check("partial failure keeps only unsuccessful targets selected and refreshes", async () => {
  const app = setup(async () => results);
  await app.render().confirmDelete();
  assert.equal([...app.selection().selectedIds].sort().join(","), "failed,unrelated");
  assert.equal(app.selection().selectedId, null);
  assert.equal(app.refreshCount(), 1);
  assert.equal(app.render().pendingDelete, null);
  assert.equal(app.render().deleteReport.results[2].error, "Access denied");
});

await check("IPC rejection preserves selection, closes modal and refreshes", async () => {
  const app = setup(async () => { throw new Error("IPC disconnected"); });
  await app.render().confirmDelete();
  assert.equal(app.selection().selectedIds.size, 4);
  assert.equal(app.refreshCount(), 1);
  assert.equal(app.render().pendingDelete, null);
  assert.match(app.render().deleteReport.error, /IPC disconnected/);
});

await check("same-render double confirmation runs once and closes modal before refresh completes", async () => {
  let finishDelete;
  let finishRefresh;
  let beganRefresh;
  const refreshStarted = new Promise((resolve) => { beganRefresh = resolve; });
  const app = setup(() => new Promise((resolve) => { finishDelete = resolve; }), () => {
    beganRefresh();
    return new Promise((resolve) => { finishRefresh = resolve; });
  });
  const commands = app.render();
  const first = commands.confirmDelete();
  const second = commands.confirmDelete();
  assert.equal(app.calls(), 1);
  finishDelete(results);
  await refreshStarted;
  assert.equal(app.render().pendingDelete, null);
  finishRefresh();
  await Promise.all([first, second]);
  assert.equal(app.render().deleting, false);
});

await check("metadata failure can be retried after its row disappears", async () => {
  const attempts = [];
  const app = setup(async (targets) => {
    attempts.push(targets);
    return attempts.length === 1 ? results : [{ ...results[2], status: "alreadyMissing", error: null }];
  });
  await app.render().confirmDelete();
  app.clearSelection();
  assert.equal(typeof app.render().handleRetryFailedDelete, "function", "Failure results need a retry action independent of list selection");
  app.render().handleRetryFailedDelete();
  assert.equal(app.calls(), 1, "retry must show confirmation before deleting");
  assert.equal(app.render().pendingDelete.sessions.length, 1);
  await app.render().confirmDelete();
  assert.equal(attempts[1].length, 1);
  assert.equal(attempts[1][0].sessionId, "failed");
  assert.equal(attempts[1][0].filePath, "failed.jsonl");
  assert.equal(app.render().deleteReport.results[0].status, "alreadyMissing");
});

assert.equal(failures.length, 0, failures.join("; "));
