// 부모·서브 세션의 표시 순서와 접기·검색·잘못된 관계를 실제 계산 코드로 검증한다.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import ts from "typescript";

function loadModule(file) {
  const source = readFileSync(new URL(file, import.meta.url), "utf8");
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  });
  const exports = {};
  new Function("require", "exports", outputText)((name) => {
    if (name === "@/lib/sessionDisplay") return loadModule("../src/lib/sessionDisplay.ts");
    throw new Error("Unexpected import: " + name);
  }, exports);
  return exports;
}

const state = loadModule("../src/lib/sessionTableState.ts");
const session = (sessionId, parentId = null, extra = {}) => ({
  sessionId, parentId, isSubagent: !!parentId, agentNickname: null,
  name: sessionId, description: null, autoSummary: null, firstUserMessage: null,
  project: "project", storageType: "local-only", favorite: false, size: 1,
  lastTimestamp: "2026-10-08T00:00:00Z", ...extra,
});
const rows = (sessions, expanded = new Set()) => {
  assert.equal(typeof state.buildSessionTreeRows, "function", "session table must group children under parents");
  return state.buildSessionTreeRows(sessions, expanded);
};
const idsAndDepth = (result) => result.map((row) => [row.session.sessionId, row.depth]);

test("collapsed parents hide descendants while orphan children remain visible", () => {
  const sessions = [session("child", "parent"), session("parent"), session("orphan", "missing")];
  const result = rows(sessions);
  assert.deepEqual(idsAndDepth(result), [["parent", 0], ["orphan", 0]]);
  assert.equal(result[0].childCount, 1);
  assert.equal(result[1].childCount, 0);
});

test("expanded families keep parent first and sort only among siblings", () => {
  const sessions = [session("z-child", "z-parent"), session("a-child", "z-parent"),
    session("grandchild", "a-child"), session("z-parent"), session("a-parent")];
  sessions.sort((a, b) => state.compareSessions(a, b, {key: "name", dir: "asc"}));
  const result = rows(sessions, new Set(["z-parent", "a-child"]));
  assert.deepEqual(idsAndDepth(result), [["a-parent", 0], ["z-parent", 0],
    ["a-child", 1], ["grandchild", 2], ["z-child", 1]]);
  assert.equal(result[1].childCount, 2);
  assert.deepEqual(idsAndDepth(rows(sessions, new Set(["z-parent"]))),
    [["a-parent", 0], ["z-parent", 0], ["a-child", 1], ["z-child", 1]]);
});

test("favorite children stay in their family and retain sibling priority", () => {
  const sessions = [session("parent"), session("z", "parent", {favorite:true}), session("a", "parent")];
  sessions.sort((a,b) => state.compareSessions(a,b,{key:"name",dir:"asc"}));
  assert.deepEqual(idsAndDepth(rows(sessions,new Set(["parent"]))), [["parent",0],["z",1],["a",1]]);
});

test("search includes every existing ancestor of a matching agent", () => {
  assert.equal(typeof state.filterSessionsWithAncestors,"function", "search must retain a matching child's parents");
  const sessions = [session("parent"),session("child","parent"),
    session("leaf","child",{agentNickname:"Gauss"}),session("sibling","parent"),session("other")];
  const filtered = state.filterSessionsWithAncestors(sessions," gaUSS ");
  assert.deepEqual(filtered.map(s=>s.sessionId),["parent","child","leaf"]);
  assert.deepEqual(idsAndDepth(rows(filtered,new Set(filtered.map(s=>s.sessionId)))),
    [["parent",0],["child",1],["leaf",2]]);
  assert.equal(state.filterSessionsWithAncestors(sessions," "),sessions);
  assert.deepEqual(state.filterSessionsWithAncestors(sessions,"unmatched"),[]);
});

test("cycles and self references neither lose rows nor loop", () => {
  const sessions = [session("a","b"),session("b","a"),session("self","self")];
  const result = rows(sessions,new Set(sessions.map(s=>s.sessionId)));
  assert.deepEqual(result.map(r=>r.session.sessionId).sort(),["a","b","self"]);
  assert.equal(new Set(result.map(r=>r.session.sessionId)).size,3);
  assert.deepEqual(state.filterSessionsWithAncestors(sessions,"a").map(s=>s.sessionId),["a","b"]);
});

test("ordinary forked sessions stay independent and deep chains are iterative", () => {
  assert.deepEqual(idsAndDepth(rows([session("fork","parent",{isSubagent:false}),session("parent")],new Set(["parent"]))),
    [["fork",0],["parent",0]]);
  const chain = Array.from({length:12000},(_,i)=>session(String(i),i?String(i-1):null));
  const result = rows(chain,new Set(chain.map(s=>s.sessionId)));
  assert.equal(result.length,12000);
  assert.equal(result.at(-1).depth,11999);
});

// 기존 회귀 검사와 같은 작은 hook 하네스로 실제 테이블 이벤트와 표시 행을 실행한다.
test("table disclosure, search expansion and select-all use actual visible rows", () => {
  const slots = [];
  let cursor = 0;
  let effects = [];
  const useState = (initial) => {
    const index = cursor++;
    if (!(index in slots)) slots[index] = typeof initial === "function" ? initial() : initial;
    return [slots[index], (value) => { slots[index] = typeof value === "function" ? value(slots[index]) : value; }];
  };
  const react = {
    memo: (component) => component, useState,
    useCallback: (fn) => fn, useMemo: (fn) => fn(),
    useRef: (initial) => useState(() => ({current: initial}))[0],
    useEffect: (fn, deps) => {
      const index = cursor++;
      if (!slots[index] || deps.some((dep,i) => dep !== slots[index][i])) effects.push(fn);
      slots[index] = deps;
    },
  };
  const element = (type, props) => ({type, props});
  const source = readFileSync(new URL("../src/components/SessionTable.tsx", import.meta.url), "utf8");
  const { outputText } = ts.transpileModule(source, {compilerOptions:{
    module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022,jsx:ts.JsxEmit.ReactJSX,
  }});
  const exports = {};
  new Function("require","exports",outputText)((name) => {
    if (name === "react") return react;
    if (name === "react/jsx-runtime") return {jsx:element,jsxs:element,Fragment:"Fragment"};
    if (name === "@/lib/sessionTableState") return state;
    if (name === "@/lib/sessionDisplay") return loadModule("../src/lib/sessionDisplay.ts");
    if (name === "@/lib/utils") return {cn:(...values)=>values.filter(Boolean).join(" "),formatBytes:String,formatRelativeTime:String};
    if (name === "lucide-react" || name.startsWith("@/components/ui/")) return new Proxy({}, {get:(_,key)=>key});
    throw new Error("Unexpected import: "+name);
  },exports);
  const all = [session("parent"), session("child","parent"),
    session("leaf","child",{agentNickname:"Gauss"}),session("other")];
  let selected = [];
  let stopped = 0;
  let resumes = 0;
  const noop = () => {};
  const props = {sessions:all, selectedId:null, locale:"ko", t:(key)=>key,
    selectedSessionIds:new Set(["parent","other"]),onSelect:noop,onResume:()=>resumes++,
    onRename:noop,onDescribe:noop,onDelete:noop,onToggleArchive:noop,onToggleCloud:noop,
    onGenerateSummary:noop,onToggleFavorite:noop,onToggleSelected:noop,
    onToggleVisibleSelection:(sessions)=>{selected=sessions.map(s=>s.sessionId);},
  };
  function render() {
    cursor = 0; effects = [];
    let output = exports.SessionTable(props);
    if(effects.length) {
      for(const effect of effects) effect();
      cursor = 0; effects = [];
      output = exports.SessionTable(props);
    }
    const nodes=[];
    function visit(node) {
      if(Array.isArray(node)) return node.forEach(visit);
      if(node && typeof node === "object" && node.props) {nodes.push(node);visit(node.props.children);}
    }
    visit(output);
    return nodes;
  }
  const displayed = (nodes) => nodes.filter(node=>node.type==="TableRow" && "data-depth" in node.props);
  const selectAll = (nodes) => nodes.find(node=>node.type==="input");
  const disclosure = (nodes,expanded) => nodes.find(node=>node.type==="button" && node.props["aria-expanded"]===expanded);
  let nodes=render();
  assert.equal(displayed(nodes).length,2,"children must start collapsed");
  assert.equal(selectAll(nodes).props.checked,true,"hidden unchecked children must not affect select-all");
  selectAll(nodes).props.onChange({currentTarget:{checked:true}});
  assert.deepEqual(selected,["parent","other"]);
  const button=disclosure(nodes,false);
  assert.ok(button,"parent must expose an accessible disclosure button");
  button.props.onClick({stopPropagation:()=>stopped++});
  button.props.onDoubleClick({stopPropagation:()=>stopped++});
  nodes=render();
  assert.equal(displayed(nodes).length,3);
  assert.equal(selectAll(nodes).props.checked,false);
  assert.equal(stopped,2);
  assert.equal(resumes,0,"disclosure clicks must not resume sessions");
  selectAll(nodes).props.onChange({currentTarget:{checked:true}});
  assert.deepEqual(selected,["parent","child","other"]);
  props.sessions=state.filterSessionsWithAncestors(all,"Gauss");
  props.autoExpand=true;
  nodes=render();
  assert.deepEqual(displayed(nodes).map(node=>node.props["data-depth"]),[0,1,2]);
  disclosure(nodes,true).props.onClick({stopPropagation:noop});
  nodes=render();
  assert.equal(displayed(nodes).length,1,"search results can be collapsed again");
});
