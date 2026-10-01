// 새로고침 요청 병합과 최신 결과 반영을 실제 훅 코드로 검증한다.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import ts from "typescript";

test("overlapping refreshes share work and discard outdated results", async () => {
  const states = [];
  const requests = [];
  const gateway = {
    listSessions: () => new Promise((resolve) => requests.push(resolve)),
    getConfig: async () => ({ sessions: {}, settings: {} }),
  };
  const react = {
    useState: (initial) => {
      const index = states.length;
      states.push(initial);
      return [initial, (value) => { states[index] = value; }];
    },
    useRef: (current) => ({ current }),
    useCallback: (callback) => callback,
    useEffect: () => {},
  };
  const source = readFileSync(new URL("../src/application/useSessionData.ts", import.meta.url), "utf8");
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  });
  const exports = {};
  new Function("require", "exports", outputText)((name) => {
    if (name === "react") return react;
    if (name === "@/adapters/tauriGateway") return { tauriGateway: gateway };
    if (name === "@/i18n") return { detectLocale: () => "ko" };
    throw new Error(`Unexpected import: ${name}`);
  }, exports);

  const { refresh } = exports.useSessionData();
  const first = refresh();
  await Promise.resolve();
  const second = refresh();
  const third = refresh();
  assert.equal(requests.length, 1, "concurrent requests must not start concurrent scans");
  requests[0]([{ sessionId: "outdated" }]);
  await new Promise(setImmediate);
  assert.deepEqual(states[0], [], "a superseded scan must not replace displayed sessions");
  assert.equal(states[1], true, "loading must remain active for the queued scan");
  assert.equal(requests.length, 2, "overlapping requests must coalesce into one follow-up scan");
  requests[1]([{ sessionId: "latest" }]);
  await Promise.all([first, second, third]);
  assert.deepEqual(states[0], [{ sessionId: "latest" }]);
  assert.equal(states[1], false);

  const later = refresh();
  await Promise.resolve();
  assert.equal(requests.length, 3, "a completed refresh must allow new scans");
  requests[2]([]);
  await later;
});
