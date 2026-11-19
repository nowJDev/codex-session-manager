// 삭제 범위 재검토와 편집 오류의 실제 프런트 동작을 검증한다.
import assert from "node:assert/strict";
import test from "node:test";
import { nodes, noop, renderer, session, textOf } from "./helpers/frontendHarness.mjs";

function commands(gateway) {
  const view = renderer("src/application/useSessionCommands.ts", {
    "@/adapters/tauriGateway": { tauriGateway: gateway },
  });
  let selected = new Set(["parent", "child", "hidden"]);
  const props = {
    refresh: async () => {}, canDeleteNow: () => true, setSessions: noop, setSelectedId: noop,
    setSelectedForDeleteIds: (update) => { selected = typeof update === "function" ? update(selected) : update; },
    selectedForDelete: [session("parent"), session("child", "parent"), session("hidden")],
  };
  return { render: () => view.render(props), selection: () => [...selected] };
}

test("removing hidden direct choices replans and preserves descendants still covered by a parent", async () => {
  const sent = [];
  const view = commands({
    prepareSessionDeletion: async (targets) => {
      sent.push(targets.map((target) => target.sessionId));
      return { targets, affectedIds: [...new Set([...sent.at(-1), "child"])] };
    },
  });
  await view.render().handleBulkDelete();
  await view.render().handleUnselectHiddenDelete(["child", "hidden"]);
  assert.deepEqual(sent, [["parent", "child", "hidden"], ["parent"]]);
  assert.deepEqual(view.selection(), ["parent"]);
  assert.deepEqual(view.render().pendingDelete.affectedIds, ["parent", "child"]);
});

test("a rejected scope can be reviewed again without sending another deletion request", async () => {
  let prepares = 0;
  let deletes = 0;
  const view = commands({
    prepareSessionDeletion: async (targets) => ({ targets, affectedIds: ++prepares === 1 ? ["parent"] : ["parent", "child"] }),
    deleteSessions: async () => { deletes++; throw Error("scope changed"); },
  });
  await view.render().handleDelete(session("parent"));
  await view.render().confirmDelete();
  assert.equal(view.render().deleteReport.error, "Error: scope changed");
  await view.render().handleRecheckDelete();
  assert.equal(deletes, 1);
  assert.deepEqual(view.render().pendingDelete.affectedIds, ["parent", "child"]);
});

test("edit keeps the draft and labels the input after a failure, and IME Enter does not save", async () => {
  const dialog = renderer("src/components/EditDialog.tsx");
  const props = { open:true, title:"Name", label:"Session name", initialValue:"old",
    t:(key) => key, onClose:noop, onSubmit:async () => { attempts++; throw Error("write"); } };
  let attempts = 0;
  const input = (out) => nodes(out).find((node) => node.type === "Input");
  input(dialog.render(props)).props.onChange({ target:{value:"draft"} });
  input(dialog.render(props)).props.onKeyDown({ key:"Enter", nativeEvent:{ isComposing:true }, preventDefault:noop });
  assert.equal(attempts, 0);
  const edited = dialog.render(props);
  const field = input(edited);
  assert.ok(field.props.id);
  assert.equal(nodes(edited).find((node) => node.type === "Label").props.htmlFor, field.props.id);
  await nodes(edited).find((node) => node.type === "Button" && node.props.children === "settings.save").props.onClick();
  const failed = dialog.render(props);
  assert.equal(input(failed).props.value, "draft");
  assert.match(textOf(failed), /edit.saveError/);
});

test("a stale confirmation cannot submit the scope before hidden selections were removed", async () => {
  const deletions = [];
  const view = commands({
    prepareSessionDeletion: async (targets) => ({ targets, affectedIds: targets.map((target) => target.sessionId) }),
    deleteSessions: async (targets) => { deletions.push(targets); return []; },
  });
  await view.render().handleBulkDelete();
  const old = view.render();
  await old.handleUnselectHiddenDelete(["hidden"]);
  await old.confirmDelete();
  assert.deepEqual(deletions, []);
  const current = view.render();
  current.cancelDelete();
  await current.confirmDelete();
  assert.deepEqual(deletions, []);
});
