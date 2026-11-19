// 삭제 확인창의 영향 목록, 페이지 이동, 포커스와 닫기 동작을 검증한다.
import assert from "node:assert/strict";
import test from "node:test";
import { nodes, renderer, textOf } from "./helpers/frontendHarness.mjs";

function renderDialog(overrides = {}) {
  const view = renderer("src/components/DeleteConfirmDialog.tsx", {}, { effects: false });
  const props = {
    open: true, count: 26, pending: false, canDelete: true,
    title: "Delete sessions", description: "Deletion scope", confirmLabel: "Confirm", cancelLabel: "Cancel",
    t: (key, params = {}) => [key, ...Object.values(params)].join(" "),
    onConfirm() {}, onCancel() {},
    ...overrides,
  };
  return { props, render: () => view.render(props) };
}

test("the deletion preview displays titles, projects and reasons in pages of 25", () => {
  const previewRows = Array.from({ length: 26 }, (_, i) => ({
    id: `id-${i}`, title: `Title ${i}!`, project: `Project ${i}!`, reason: `Reason ${i}!`,
  }));
  const dialog = renderDialog({ previewRows });
  const firstPage = dialog.render();
  assert.match(textOf(firstPage), /Title 24!.*Project 24!.*Reason 24!/);
  assert.doesNotMatch(textOf(firstPage), /Title 25!/);
  const nextPage = nodes(firstPage).find(node => textOf(node) === "delete.nextPage" && node.type === "Button");
  assert.equal(nextPage?.props.disabled, false);
  nextPage.props.onClick();
  const secondPage = dialog.render();
  assert.match(textOf(secondPage), /Title 25!.*Project 25!.*Reason 25!/);
  assert.doesNotMatch(textOf(secondPage), /Title 24!/);
});

test("the confirmation permits removing hidden selections before deletion", () => {
  let removed = false;
  const dialog = renderDialog({ hiddenSelectedCount: 3, onUnselectHidden: () => { removed = true; } });
  const output = dialog.render();
  assert.match(textOf(output), /delete.hiddenSelected 3/);
  nodes(output).find(node => node.type === "Button" && textOf(node) === "delete.unselectHidden").props.onClick();
  assert.equal(removed, true);
});

test("the confirmation focuses Cancel on opening and prevents dismissal during deletion", () => {
  let cancelled = 0;
  let focused = false;
  const dialog = renderDialog({ onCancel: () => { cancelled++; } });
  const output = dialog.render();
  const cancel = nodes(output).find(node => node.type === "Button" && textOf(node) === "Cancel");
  const content = nodes(output).find(node => node.type === "DialogContent");
  assert.ok(cancel.props.ref);
  cancel.props.ref.current = { focus: () => { focused = true; } };
  content.props.onOpenAutoFocus({ preventDefault() {} });
  assert.equal(focused, true);

  dialog.props.pending = true;
  const pending = dialog.render();
  const pendingContent = nodes(pending).find(node => node.type === "DialogContent");
  assert.equal(pendingContent.props.showClose, false);
  let prevented = 0;
  const event = { preventDefault: () => { prevented++; } };
  pendingContent.props.onEscapeKeyDown(event);
  pendingContent.props.onInteractOutside(event);
  assert.equal(prevented, 2);
  pending.props.onOpenChange(false);
  assert.equal(cancelled, 0);
});

test("a zero-target preview offers only dismissal", () => {
  const { render } = renderDialog({ count:0, cancelLabel:"Close" });
  assert.deepEqual(nodes(render()).filter((node) => node.type === "Button").map(textOf), ["Close"]);
});
