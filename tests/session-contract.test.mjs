// 조회 실패 보존과 API 세션 삭제 사전 확인 계약을 실제 훅으로 검증한다.
import assert from "node:assert/strict";
import {readFileSync} from "node:fs";
import test from "node:test";
import ts from "typescript";

export function harness(file, gateway, params, modules={}, runEffects=false) {
  const slots=[]; let cursor=0;
  let effects=[];
  const react={useState(initial) { const i=cursor++; if(!(i in slots)) slots[i]=typeof initial==="function"?initial():initial;
    return [slots[i],value=>{slots[i]=typeof value==="function"?value(slots[i]):value;}]; },
    useRef(initial) {return this.useState(()=>({current:initial}))[0];},useCallback:fn=>fn,useMemo:fn=>fn(),useEffect:fn=>{if(runEffects)effects.push(fn);}};
  react.useRef=react.useRef.bind(react);
  const {outputText}=ts.transpileModule(readFileSync(new URL(file,import.meta.url),"utf8"),{
    compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}});
  const exports={};
  new Function("require","exports","console","alert",outputText)(name=>{
    if(name in modules)return modules[name];
    if(name==="react")return react;
    if(name==="@/adapters/tauriGateway")return {tauriGateway:gateway};
    if(name==="@/i18n")return {detectLocale:()=>"ko"};
    throw new Error(name);
  },exports,{error(){}},()=>assert.fail("unexpected blocking alert"));
  return (...args)=>{cursor=0;effects=[];const value=Object.values(exports)[0](...args.length?args:[params]);for(const effect of effects)effect();return value;};
}

test("refresh failure keeps the old list and clears its banner only after success",async()=>{
  let fail=false;
  const render=harness("../src/application/useSessionData.ts",{
    listSessions:async()=>{if(fail)throw Error("API unavailable");return [{sessionId:"kept"}];},
    getConfig:async()=>({sessions:{},settings:{}}),
  });
  await render().refresh(); fail=true; await render().refresh();
  assert.deepEqual(render().sessions,[{sessionId:"kept"}]);
  assert.match(render().refreshError,/API unavailable/);
  fail=false;await render().refresh();assert.equal(render().refreshError,null);
});

test("hidden registered records remain raw and reachable through the visibility toggle",()=>{
  const sessions=[{sessionId:"visible",registered:true,hidden:false,storageType:"local-only"},
    {sessionId:"hidden",registered:true,hidden:true,storageType:"local-only"},
    {sessionId:"extra",registered:false,hidden:false,storageType:"local-only"}];
  const render=harness("../src/application/useSessionManager.ts",{},undefined,{
    "@/i18n":{createT:()=>key=>key},
    "@/application/useSessionData":{useSessionData:()=>({sessions,refreshError:"failed"})},
    "@/application/useSessionSelection":{useSessionSelection:()=>({})},
    "@/application/useSessionCommands":{useSessionCommands:()=>({})},
    "@/application/useSettingsActions":{useSettingsActions:()=>({})},
    "@/lib/sessionTableState":{filterSessionsWithAncestors:items=>items},
  });
  assert.equal(render().localCount,2);assert.equal(render().refreshError,"failed");
  assert.deepEqual(render().filtered.map(s=>s.sessionId),["visible","extra"]);
  render().setShowHidden(true);
  assert.equal(render().filtered.length,3);assert.equal(sessions.length,3);
});

const record=sessionId=>({sessionId,registered:true,storageType:"local-only"});
function commands(gateway, refresh=async()=>{}) {
  let selectedId="parent";let selected=new Set(["parent","child","uncertain"]);
  const render=harness("../src/application/useSessionCommands.ts",gateway,{
    refresh,setSessions(){},setSelectedId(fn){selectedId=fn(selectedId);},
    setSelectedForDeleteIds(fn){selected=fn(selected);},selectedForDelete:[record("parent")],
  });
  return {render,selection:()=>({selectedId,selected})};
}
test("confirmation uses exact affected IDs and unconfirmed results remain selected",async()=>{
  const calls=[];
  const app=commands({prepareSessionDeletion:async targets=>({targets,affectedIds:["parent","child","uncertain"]}),
    deleteSessions:async(...args)=>{calls.push(args);return [
      {sessionId:"parent",status:"deleted",error:null,warning:"cleanup warning"},
      {sessionId:"child",status:"alreadyMissing",error:null,warning:null},
      {sessionId:"uncertain",status:"unconfirmed",error:"still registered",warning:null}];}});
  await app.render().handleBulkDelete();
  assert.deepEqual(app.render().pendingDelete.affectedIds,["parent","child","uncertain"]);
  await app.render().confirmDelete();
  assert.deepEqual(calls,[[[{sessionId:"parent"}],["parent","child","uncertain"]]]);
  assert.deepEqual([...app.selection().selected],["uncertain"]);
  assert.equal(app.render().deleteReport.results[0].warning,"cleanup warning");
});
test("preflight failures never show confirmation or send delete",async()=>{
  let deletes=0;
  const app=commands({prepareSessionDeletion:async()=>{throw Error("scope unavailable");},deleteSessions:async()=>{deletes++;}});
  await app.render().handleBulkDelete();await app.render().confirmDelete();
  assert.equal(app.render().pendingDelete,null);assert.equal(deletes,0);
  assert.match(app.render().deleteReport.error,/scope unavailable/);
  assert.equal(app.render().deleteReport.phase,"prepare");
});
test("duplicate preparations share a lock and retry refreshes then prepares again",async()=>{
  let finish;const events=[];
  const app=commands({prepareSessionDeletion:async targets=>{events.push("prepare");return new Promise(resolve=>{finish=()=>resolve({targets,affectedIds:["parent"]});});},
    deleteSessions:async()=>[{sessionId:"parent",status:"unconfirmed",error:"unknown",warning:null}]},async()=>{events.push("refresh");});
  const first=app.render().handleBulkDelete();const duplicate=app.render().handleBulkDelete();
  assert.deepEqual(events,["prepare"]);finish();await Promise.all([first,duplicate]);
  await app.render().confirmDelete();const retry=app.render().handleRetryFailedDelete();
  await new Promise(setImmediate);assert.deepEqual(events,["prepare","refresh","refresh","prepare"]);
  finish();await retry;assert.deepEqual(app.render().pendingDelete.sessions,[{sessionId:"parent"}]);
});
test("cloud-only resume checks out first and extra files cannot use API commands",async()=>{
  const calls=[];const app=commands({checkoutSession:async()=>calls.push("checkout"),resumeSession:async()=>calls.push("resume"),
    archiveSession:async()=>calls.push("archive"),generateSummary:async()=>calls.push("summary")});
  await app.render().handleResume({...record("cloud"),registered:false,storageType:"cloud-only",historyMode:"legacy"});
  const extra={...record("extra"),registered:false};
  await app.render().handleResume(extra);await app.render().handleToggleArchive(extra);await app.render().handleGenerateSummary(extra);
  await app.render().handleDelete(extra);
  assert.deepEqual(calls,["checkout","resume"]);assert.equal(app.render().pendingDelete,null);
});

test("refreshing a missing failed row does not silently clear its selected ID",()=>{
  const render=harness("../src/application/useSessionSelection.ts",{},undefined,{},true);
  render([record("uncertain")]).handleToggleSelected(record("uncertain"));
  render([]);
  assert.deepEqual([...render([]).selectedForDeleteIds],["uncertain"]);
});

test("registered database-only summary uses only its session ID",async()=>{
  const calls=[];const app=commands({generateSummary:async(...args)=>calls.push(args)});
  await app.render().handleGenerateSummary({...record("db-only"),filePath:null,historyMode:"paginated"});
  assert.deepEqual(calls,[["db-only"]]);
});

test("successful deletion with a cleanup warning can be rechecked after its row disappears",async()=>{
  const calls=[];let selected=[record("parent")];
  const render=harness("../src/application/useSessionCommands.ts",{
    prepareSessionDeletion:async targets=>{calls.push(["prepare",targets]);return {targets,affectedIds:["parent"]};},
    deleteSessions:async(targets,approvedIds)=>{
      calls.push(["delete",targets,approvedIds]);
      return [{sessionId:"parent",status:calls.filter(call=>call[0]==="delete").length===1?"deleted":"alreadyMissing",error:null,
        warning:calls.filter(call=>call[0]==="delete").length===1?"metadata cleanup failed":null}];
    },
  },{refresh:async()=>{selected=[];calls.push(["refresh"]);},setSessions(){},setSelectedId(){},setSelectedForDeleteIds(){},
    get selectedForDelete(){return selected;}});
  await render().handleBulkDelete();await render().confirmDelete();
  assert.equal(selected.length,0,"the deleted row no longer exists in the refreshed list");
  await render().handleRetryFailedDelete();
  assert.deepEqual(render().pendingDelete?.sessions,[{sessionId:"parent"}]);
  assert.deepEqual(calls.slice(-2),[["refresh"],["prepare",[{sessionId:"parent"}]]]);
  assert.equal(calls.filter(call=>call[0]==="delete").length,1,"cleanup recheck still needs fresh confirmation");
  await render().confirmDelete();
  assert.equal(render().deleteReport.results[0].status,"alreadyMissing");
  assert.equal(render().deleteReport.results[0].warning,null);
});
