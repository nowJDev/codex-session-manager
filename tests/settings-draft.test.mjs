// 설정 화면의 디렉터리 초안, 저장 실패, 진단 시 선택 보존을 검증한다.
import assert from "node:assert/strict";
import test from "node:test";
import { nodes, noop, renderer, textOf } from "./helpers/frontendHarness.mjs";

function button(tree, ...labels) {
  const match = nodes(tree).find((node) => node.type === "Button" && labels.includes(textOf(node)));
  assert.ok(match, `button ${labels[0]}`);
  return match.props;
}

function dialog(overrides = {}) {
  const props = {
    open: true, current: { locale: "ko", preferredTerminal: "custom", customTerminalProgram: "old.exe" },
    locale: "ko", t: (key) => ({
      "settings.cloudPick": "직접 선택", "settings.cloudDetect": "자동 감지",
      "settings.save": "저장", "settings.cancel": "취소",
    })[key] ?? key, onClose: noop, onSaved: noop, onSaveSettings: noop,
    onCheckEnvironment: async () => ({ terminals: [] }),
    ...overrides,
  };
  const component = renderer("src/components/SettingsDialog.tsx");
  return { props, render: (patch = {}) => component.render({ ...props, ...patch }) };
}

test("picking a cloud folder gives a preview without persisting it", async () => {
  const root = "C:\\QA\\My Drive";
  const path = root + "\\Codex Sessions";
  const persisted = [];
  const gateway = {
    pickDirectory: async () => root,
    previewCloudFolder: async () => path,
    setCloudFolder: async (value) => {
      persisted.push(value);
      return path;
    },
  };
  const app = renderer("src/application/useSettingsActions.ts", {
    "@/adapters/tauriGateway": { tauriGateway: gateway },
  });

  const candidate = await app.render().pickCloudFolder();
  assert.deepEqual(persisted, []);
  assert.deepEqual(candidate, { root, path });
});

test("cancel drops the folder draft, while Save submits the preview's root", async () => {
  const candidate = { root: "C:\\QA", path: "C:\\QA\\Codex Sessions" };
  const saves = [];
  const ui = dialog({
    onPickCloudFolder: async () => candidate,
    onSaveSettings: async (...args) => { saves.push(args); },
  });
  await button(ui.render(), "직접 선택").onClick();
  const tree = ui.render();
  assert.ok(nodes(tree).some((node) => node.type === "Input" && node.props.value === candidate.path));
  button(tree, "취소").onClick();
  ui.render({ open: false });
  assert.ok(!nodes(ui.render()).some((node) => node.type === "Input" && node.props.value === candidate.path));
  assert.deepEqual(saves, []);
  await button(ui.render(), "직접 선택").onClick();
  await button(ui.render(), "저장").onClick();
  assert.equal(saves.length, 1);
  assert.equal(saves[0][1], candidate.root);
});

test("a missing Google Drive is disclosed without clearing the configured folder", async () => {
  const savedPath = "C:\\QA\\Codex Sessions";
  const ui = dialog({
    current: { cloudPath: savedPath },
    onDetectGoogleDrive: async () => null,
  });

  await button(ui.render(), "자동 감지").onClick();

  const contents = nodes(ui.render());
  assert.ok(contents.some((node) => node.type === "Input" && node.props.value === savedPath));
  assert.ok(contents.some((node) => node.props.role === "alert" && textOf(node).includes("Google Drive")));
});

test("live config refresh does not replace edits in an open settings dialog", () => {
  const ui = dialog();
  const program = nodes(ui.render()).find((node) => node.type === "Input" && node.props.value === "old.exe");
  program.props.onChange({ target: { value: "draft.exe" } });

  const refreshed = ui.render({ current: { ...ui.props.current, customTerminalProgram: "refreshed.exe" } });
  assert.ok(nodes(refreshed).some((node) => node.type === "Input" && node.props.value === "draft.exe"));
  ui.render({ open: false });
  const reopened = ui.render({ current: { ...ui.props.current, customTerminalProgram: "refreshed.exe" } });
  assert.ok(nodes(reopened).some((node) => node.type === "Input" && node.props.value === "refreshed.exe"));
});

test("running terminal diagnostics keeps the selected terminal and the custom option", async () => {
  const ui = dialog({ current: { preferredTerminal: "wt" } });
  await button(ui.render(), "Run diagnostics").onClick();
  const contents = nodes(ui.render());

  assert.equal(contents.find((node) => node.type === "select").props.value, "wt");
  assert.ok(contents.some((node) => node.type === "option" && node.props.value === "wt"));
  assert.ok(contents.some((node) => node.type === "option" && node.props.value === "custom"));
});

test("a failed settings save leaves the draft open for retry", async () => {
  let closed = false;
  const ui = dialog({
    onClose: () => { closed = true; },
    onSaveSettings: async () => { throw new Error("Disk full"); },
  });
  await button(ui.render(), "저장").onClick();

  assert.equal(closed, false);
  assert.ok(nodes(ui.render()).some((node) => node.props.role === "alert" && textOf(node).includes("Disk full")));
});
