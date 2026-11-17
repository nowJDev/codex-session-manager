// 완전성 보존과 오래된 렌더·사전 조회 응답의 삭제 우회를 실제 훅으로 검증한다.
import assert from 'node:assert/strict';
import test from 'node:test';
import {renderer,session,snapshot} from './helpers/frontendHarness.mjs';
const config=async()=>({sessions:{},settings:{}});
function dataView(gateway) {return renderer('src/application/useSessionData.ts',{'@/adapters/tauriGateway':{tauriGateway:gateway},'@/i18n':{detectLocale:()=> 'ko'}},{effects:false});}

test('first incomplete snapshot shows partial rows; failed refresh never replaces a complete list',async()=>{
  let response=snapshot([session('partial')],{complete:false,missingCount:2,errors:['DB unavailable']});
  const view=dataView({listSessions:async()=>response,getConfig:config});
  await view.render().refresh();let state=view.render();assert.deepEqual(state.sessions,[session('partial')]);assert.equal(state.missingCount,2);assert.equal(state.canDeleteNow(),false);
  response=snapshot([session('complete')]);await state.refresh();state=view.render();assert.equal(state.canDeleteNow(),true);
  response=snapshot([session('wrong')],{complete:false,missingCount:1,errors:['timeout']});await state.refresh();state=view.render();
  assert.deepEqual(state.sessions.map(s=>s.sessionId),['complete']);assert.equal(state.snapshotStale,true);assert.equal(state.canDeleteNow(),false);assert.match(state.refreshError,/timeout/);
  response=snapshot([session('fresh')]);await state.refresh();assert.equal(view.render().canDeleteNow(),true);assert.equal(view.render().refreshError,null);
});

test('a rejected refresh blocks deletion synchronously through an already retained callback',async()=>{
  let fail=false;const view=dataView({listSessions:async()=>{if(fail)throw Error('offline');return snapshot([session('p')]);},getConfig:config});
  await view.render().refresh();const old=view.render();assert.equal(typeof old.canDeleteNow,'function','data exposes the current deletion safety gate');assert.equal(old.canDeleteNow(),true);fail=true;const pending=old.refresh();assert.equal(old.canDeleteNow(),false);await pending;assert.equal(old.canDeleteNow(),false);assert.equal(view.render().snapshotStale,true);
});

function commandView(gateway, extra={}) {const params={refresh:async()=>{},setSessions(){},setSelectedId(){},setSelectedForDeleteIds(){},selectedForDelete:[session('p')],canDeleteNow:()=>true,...extra};
  const view=renderer('src/application/useSessionCommands.ts',{'@/adapters/tauriGateway':{tauriGateway:gateway}},{effects:false});return {params,render:()=>view.render(params)};}

test('single and bulk prepare are blocked by an incomplete snapshot',async()=>{
  let prepares=0;const app=commandView({prepareSessionDeletion:async targets=>{prepares++;return {targets,affectedIds:['p']};}},{canDeleteNow:()=>false});
  await app.render().handleDelete(session('p'));await app.render().handleBulkDelete();assert.equal(prepares,0);assert.equal(app.render().pendingDelete,null);
});

test('late prepare response after refresh failure cannot open confirmation',async()=>{
  let ready=true;let finish;const app=commandView({prepareSessionDeletion:targets=>new Promise(resolve=>{finish=()=>resolve({targets,affectedIds:['p']});})},{canDeleteNow:()=>ready});
  const request=app.render().handleBulkDelete();ready=false;finish();await request;assert.equal(app.render().pendingDelete,null);
});

test('an old confirmation closure reads the latest deletion gate',async()=>{
  let deletes=0;const app=commandView({prepareSessionDeletion:async targets=>({targets,affectedIds:['p']}),deleteSessions:async()=>{deletes++;return [];}});
  await app.render().handleBulkDelete();const previous=app.render();app.params.canDeleteNow=()=>false;app.render();await previous.confirmDelete();assert.equal(deletes,0);
});

test('retry refresh that resolves after failure cannot prepare or delete again',async()=>{
  let ready=true;let prepares=0;const app=commandView({prepareSessionDeletion:async targets=>{prepares++;return {targets,affectedIds:['p']};},deleteSessions:async()=>[{sessionId:'p',status:'unconfirmed',error:'uncertain',warning:null}]},
    {canDeleteNow:()=>ready,refresh:async()=>{ready=false;}});
  await app.render().handleBulkDelete();await app.render().confirmDelete();ready=true;await app.render().handleRetryFailedDelete();assert.equal(prepares,1);assert.equal(app.render().pendingDelete,null);
});


test('selection survives incomplete absence but complete absence clears it without propagating parent checks',()=>{
  const view=renderer('src/application/useSessionSelection.ts');const all=[session('p'),session('c','p')];
  view.render(all,true).handleToggleSelected(all[0]);let state=view.render(all,true);assert.deepEqual([...state.selectedForDeleteIds],['p']);
  state=view.render([],false);assert.deepEqual([...state.selectedForDeleteIds],['p']);
  state=view.render([],true);assert.deepEqual([...state.selectedForDeleteIds],[]);
});


test('manager counts the logical displayed rows and retains hidden selections through search',async()=>{
  const all=[session('p'),session('c','p'),session('guardian',null,{sessionKind:'guardian'}),session('extra',null,{registered:false})];
  const gateway={listSessions:async()=>snapshot(all),getConfig:config};
  const view=renderer('src/application/useSessionManager.ts',{'@/adapters/tauriGateway':{tauriGateway:gateway},'@/i18n':{detectLocale:()=> 'ko',createT:()=>key=>key}},{effects:false});
  await view.render().refresh();let state=view.render();state.handleToggleSelected(all[1]);state=view.render();
  assert.equal(typeof state.handleVisibleRowsChange,'function','the manager accepts actual table rows');state.handleVisibleRowsChange([all[0],all[2],all[3]]);state=view.render();
  assert.equal(state.displayedCount,3);assert.equal(state.selectedForDeleteCount,1);assert.equal(state.visibleSelectedCount,0);assert.equal(state.workerCount,1);
  state.setQuery('guardian');state=view.render();assert.deepEqual(state.filtered.map(s=>s.sessionId),['guardian']);assert.equal(state.selectedForDeleteCount,1);
});
