// 세션 표의 제목·정렬·행별 이름과 가지 활동값을 검증한다.
import assert from "node:assert/strict";
import test from "node:test";
import { nodes, renderer, session, tableProps, textOf } from "./helpers/frontendHarness.mjs";

test("a native title button opens details, and only that button resumes on double-click", () => {
  const view = renderer("src/components/SessionTable.tsx");
  const props = tableProps([session("first"), session("second", null, { registered: false })]);
  props.t = (key, params) => [key, params?.name].filter(Boolean).join(":");
  const selected = [];
  const resumed = [];
  props.onSelect = (item) => selected.push(item.sessionId);
  props.onResume = (item) => resumed.push(item.sessionId);
  const rendered = nodes(view.render(props));
  const row = rendered.find((node) => node.props["data-session-id"] === "first");
  const other = rendered.find((node) => node.props["data-session-id"] === "second");
  const title = (element) => nodes(element).find((node) => node.type === "button" && "data-session-title" in node.props);
  assert.ok(title(row));
  assert.equal(row.props.onDoubleClick, undefined);
  assert.equal(row.props.onClick, undefined);
  title(row).props.onClick();
  title(row).props.onDoubleClick();
  title(other).props.onDoubleClick();
  assert.deepEqual(selected, ["first"]);
  assert.deepEqual(resumed, ["first"]);
  for (const label of ["action.selectSession:first", "action.favoriteSession:first", "action.sessionActions:first"]) {
    assert.ok(nodes(row).some((node) => node.props["aria-label"] === label), label);
  }
});

test("column sorting uses native buttons and exposes the current sort to assistive technology", () => {
  const view = renderer("src/components/SessionTable.tsx");
  const props = tableProps([session("z"), session("a")]);
  const header = (key) => {
    const node = nodes(view.render(props)).find((part) => part.props.sortKey === key);
    return node.type(node.props);
  };
  assert.equal(header("lastActive").props["aria-sort"], "descending");
  assert.equal(header("name").props["aria-sort"], undefined);
  const sort = nodes(header("name")).find((node) => node.type === "button");
  assert.ok(sort);
  sort.props.onClick();
  assert.equal(header("name").props["aria-sort"], "ascending");
  assert.equal(header("lastActive").props["aria-sort"], undefined);
});

test("collapsed branch displays its newest activity even under a different sort", () => {
  const view = renderer("src/components/SessionTable.tsx");
  const state = view.load("src/lib/sessionTableState.ts");
  const all = [
    session("parent", null, { lastTimestamp: "2026-01-01" }),
    session("child", "parent", { lastTimestamp: "2026-10-01" }),
    session("grandchild", "child", { lastTimestamp: "2026-11-01" }),
    session("other", null, { lastTimestamp: "2026-05-01" }),
  ];
  const rows = state.buildSessionTreeRows(all, new Set(), { key: "name", dir: "asc" });
  assert.equal(rows.find(({ session }) => session.sessionId === "parent").branchLastTimestamp, "2026-11-01");

  const parent = nodes(view.render(tableProps(all))).find((node) => node.props["data-session-id"] === "parent");
  assert.match(textOf(parent), /2026-11-01/);
  assert.match(textOf(parent), /list.branchActivityValue/);
  assert.doesNotMatch(textOf(parent), /2026-01-01/);

  const detail = renderer("src/components/SessionDetail.tsx");
  const detailNodes = nodes(detail.render({
    session: all[0], branchLastTimestamp: "2026-11-01", depth: 0,
    locale: "ko", t: (key) => key,
  }));
  assert.equal(detailNodes.find((node) => node.props.label === "detail.ownActivity").props.value, "2026-01-01");
  assert.equal(detailNodes.find((node) => node.props.label === "detail.branchActivity").props.value, "2026-11-01");
});

test("filtering keeps the full branch activity and sort value without revealing excluded descendants", () => {
  const view = renderer("src/components/SessionTable.tsx");
  const state = view.load("src/lib/sessionTableState.ts");
  const parent = session("parent", null, { lastTimestamp: "2026-01-01" });
  const peer = session("peer", null, { lastTimestamp: "2026-06-01" });
  const child = session("child", "parent", { lastTimestamp: "2026-11-01" });
  const allRows = state.buildSessionTreeRows([parent, peer, child], new Set(["parent"]));
  const props = tableProps([parent, peer]);
  props.query = "fixture";
  props.branchActivities = new Map(allRows.map((row) => [row.session.sessionId, row]));

  const displayed = nodes(view.render(props)).filter((node) => node.props["data-session-id"]);
  assert.deepEqual(displayed.map((node) => node.props["data-session-id"]), ["parent", "peer"]);
  assert.match(textOf(displayed[0]), /2026-11-01/);
  assert.match(textOf(displayed[0]), /list.branchActivityValue/);
  assert.ok(nodes(displayed[0]).every((node) => node.type !== "button" || !("aria-expanded" in node.props)));
});
