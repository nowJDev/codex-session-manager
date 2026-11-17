// 실제 프런트 모듈의 상태와 JSX 이벤트를 메모리에서 실행한다.
import { readFileSync } from "node:fs";
import ts from "typescript";

export const session = (sessionId, parentId = null, extra = {}) => ({
  sessionId, parentId, isSubagent: !!parentId, sessionKind: parentId ? "worker" : "main", isSummaryRun: false,
  agentNickname: null, name: sessionId, description: null, autoSummary: null,
  project: "fixture", projectDir: "fixture", filePath: "fixture.jsonl", size: 1, preview: null,
  historyMode: "legacy", registered: true, totalLines: 1, firstTimestamp: null,
  lastTimestamp: "2026-11-01T00:00:00Z", cwd: null, version: null, firstUserMessage: null,
  storageType: "local-only", archived: false, favorite: false, ...extra,
});
export const snapshot = (sessions, extra = {}) => ({sessions, complete:true, missingCount:0, errors:[], ...extra});
export const noop = () => {};

export function renderer(file, modules = {}, {effects = true} = {}) {
  const slots = []; let cursor = 0; let pending = []; let dirty = false;
  const same = (a,b) => a && b && a.length === b.length && b.every((v,i)=>Object.is(v,a[i]));
  const react = {
    memo: fn => fn,
    useState(initial) { const i=cursor++; if (!(i in slots)) slots[i]=typeof initial === "function" ? initial() : initial;
      return [slots[i], value => { const next = typeof value === "function" ? value(slots[i]) : value;
        if (!Object.is(next,slots[i])) { slots[i]=next; dirty=true; } }]; },
    useRef(initial) { const i=cursor++; if (!(i in slots)) slots[i]={current:initial}; return slots[i]; },
    useMemo(fn,deps) {const i=cursor++; if (!slots[i] || !same(slots[i].deps,deps)) slots[i]={deps,value:fn()}; return slots[i].value;},
    useCallback(fn,deps) {return react.useMemo(()=>fn,deps);},
    useEffect(fn,deps) {const i=cursor++; if (effects && (!slots[i] || !same(slots[i],deps))) pending.push(fn); slots[i]=deps;},
  };
  const element=(type,props)=>({type,props}); const cache=new Map();
  function load(path) {
    if(cache.has(path)) return cache.get(path);
    const exports={};cache.set(path,exports);
    const source=readFileSync(new URL('../../'+path,import.meta.url),'utf8');
    const {outputText}=ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022,jsx:ts.JsxEmit.ReactJSX}});
    new Function('require','exports','console',outputText)(name=>{
      if(name in modules)return modules[name];
      if(name==='react')return react;
      if(name==='react/jsx-runtime')return {jsx:element,jsxs:element,Fragment:'Fragment'};
      if(name==='lucide-react'||name.startsWith('@/components/ui/'))return new Proxy({},{get:(_,key)=>key});
      if(name==='@/lib/utils')return {cn:(...xs)=>xs.filter(Boolean).join(' '),formatBytes:String,formatRelativeTime:String};
      if(name.startsWith('@/'))return load('src/'+name.slice(2)+(name.includes('/components/')?'.tsx':'.ts'));
      if(name.endsWith('.json'))return {default:JSON.parse(readFileSync(new URL('../../src/i18n/'+name.slice(2),import.meta.url),'utf8'))};
      throw Error('Unexpected import '+name);
    },exports,{error(){}});
    return exports;
  }
  const exported=load(file); const component=Object.values(exported)[0];
  return {load,render(...args) {let result;for(let pass=0;pass<20;pass++) {cursor=0;pending=[];dirty=false;result=component(...args);for(const fn of pending)fn();if(!dirty)return result;}throw Error('render did not settle');}};
}
export function nodes(output) {const out=[];function visit(n){if(Array.isArray(n))return n.forEach(visit);if(n&&typeof n==='object'&&n.props){out.push(n);visit(n.props.children);}}visit(output);return out;}
export function textOf(n) {if(Array.isArray(n))return n.map(textOf).join(' ');if(n&&typeof n==='object')return textOf(n.props?.children);return n == null ? '' : String(n);}
export const tableProps = sessions => ({sessions,query:'',selectedId:null,locale:'ko',t:key=>key,
  selectedSessionIds:new Set(),onSelect:noop,onResume:noop,onRename:noop,onDescribe:noop,onDelete:noop,
  onToggleArchive:noop,onToggleCloud:noop,onGenerateSummary:noop,onToggleFavorite:noop,
  onToggleSelected:noop,onToggleVisibleSelection:noop,onVisibleRowsChange:noop,canDelete:true});
