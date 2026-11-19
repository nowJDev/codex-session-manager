// 초기 전체 트리와 검색·접기·선택 상태가 갱신에도 일관되는지 검증한다.
import assert from 'node:assert/strict';
import test from 'node:test';
import {renderer,session,nodes,textOf,tableProps} from './helpers/frontendHarness.mjs';
const visible=out=>nodes(out).filter(n=>n.type==='TableRow'&&'data-depth' in n.props);
const id=n=>n.props['data-session-id'] || nodes(n).find(x=>x.type==='TableCell'&&x.props.title)?.props.title;
const ids=out=>visible(out).map(id);
const click=n=>n.props.onClick({stopPropagation(){},currentTarget:{focus(){}}});
const disclosure=(out,id)=>nodes(visible(out).find(n=>(n.props['data-session-id'] || nodes(n).find(x=>x.type==='TableCell'&&x.props.title)?.props.title)===id)).find(n=>n.type==='button'&&'aria-expanded' in n.props);

test('async initial rows are collapsed; refresh preserves manual expansion and collapses new branches',()=>{
  const view=renderer('src/components/SessionTable.tsx'); const props=tableProps([]);
  view.render(props); props.sessions=[session('p'),session('c','p'),session('g','c'),session('guardian',null,{sessionKind:'guardian'})];
  let out=view.render(props);assert.deepEqual(ids(out),['p','guardian']);
  click(disclosure(out,'p'));out=view.render(props);assert.deepEqual(ids(out),['p','c','guardian']);
  props.sessions=[...props.sessions.map(s=>({...s})),session('new'),session('new-child','new')];
  out=view.render(props);assert.deepEqual(ids(out),['p','c','guardian','new']);
});

test('search restores prior collapse state and does not reopen a manually collapsed match on refresh',()=>{
  const view=renderer('src/components/SessionTable.tsx');const all=[session('p'),session('c','p'),session('leaf','c'),session('other')];
  const props=tableProps(all);let out=view.render(props);assert.deepEqual(ids(out),['p','other']);click(disclosure(out,'p'));view.render(props);
  props.query='leaf';props.sessions=all.slice(0,3);out=view.render(props);assert.deepEqual(ids(out),['p','c','leaf']);
  click(disclosure(out,'p'));out=view.render(props);assert.deepEqual(ids(out),['p']);
  props.sessions=props.sessions.map(s=>({...s}));out=view.render(props);assert.deepEqual(ids(out),['p']);
  props.query='';props.sessions=all;out=view.render(props);assert.deepEqual(ids(out),['p','c','other']);
});

test('expand controls report all logical rows and select only currently visible registered rows',()=>{
  const view=renderer('src/components/SessionTable.tsx');const props=tableProps([session('p'),session('c','p'),session('extra',null,{registered:false}),session('guardian',null,{sessionKind:'guardian'})]);
  let report;let selected;props.onVisibleRowsChange=(rows)=>report=rows.map(s=>s.sessionId);props.onToggleVisibleSelection=(rows)=>selected=rows.map(s=>s.sessionId);
  let out=view.render(props);assert.deepEqual(report,['p','extra','guardian']);
  click(nodes(out).find(n=>n.props['aria-label']==='action.expandAll'));out=view.render(props);
  assert.deepEqual(report,['p','c','extra','guardian']);
  click(nodes(out).find(n=>n.props['aria-label']==='action.collapseAll'));out=view.render(props);
  assert.deepEqual(report,['p','extra','guardian']);nodes(out).find(n=>n.type==='input').props.onChange({currentTarget:{checked:true}});
  assert.deepEqual(selected,['p','guardian']);
  click(nodes(out).find(n=>n.props['aria-label']==='action.expandAll'));out=view.render(props);assert.equal(visible(out).length,4);
  props.sessions=Array.from({length:2300},(_,i)=>session(String(i)));out=view.render(props);
  nodes(out).find(n=>n.type==='input').props.onChange({currentTarget:{checked:true}});assert.equal(selected.length,2300);assert.equal(report.length,2300);
});

test('kind badges, summary badge, orphan explanation and deletion gate reach real row output',()=>{
  const view=renderer('src/components/SessionTable.tsx');const props=tableProps([session('guardian',null,{sessionKind:'guardian'}),session('orphan','missing',{isSummaryRun:true})]);props.canDelete=false;
  const out=view.render(props);const text=textOf(out);assert.match(text,/kind.guardian/);assert.match(text,/kind.summary/);assert.match(text,/list.parentMissing/);
  const deletes=nodes(out).filter(n=>n.type==='DropdownMenuItem'&&n.props.children==='action.delete');assert.ok(deletes.every(n=>n.props.disabled));
});

test('actual parent IDs connect every kind and default sorting promotes the latest descendant branch',()=>{
  const view=renderer('src/components/SessionTable.tsx');const state=view.load('src/lib/sessionTableState.ts');
  const all=[session('recent',null,{lastTimestamp:'2026-10-01'}),session('old',null,{lastTimestamp:'2026-01-01'}),session('child','old',{isSubagent:false,sessionKind:'unknown',lastTimestamp:'2026-11-17'})];
  const rows=state.buildSessionTreeRows(all,new Set(['old']),{key:'lastActive',dir:'desc'});
  assert.deepEqual(rows.map(r=>[r.session.sessionId,r.depth]),[['old',0],['child',1],['recent',0]]);
  assert.deepEqual(state.filterSessionsWithAncestors(all,'unknown').map(s=>s.sessionId),['old','child']);
});


test('collapse moves focus to its parent disclosure while retaining detail selection',()=>{
  const view=renderer('src/components/SessionTable.tsx');const props=tableProps([session('p'),session('c','p')]);props.selectedId='c';
  click(disclosure(view.render(props),'p'));
  const out=view.render(props);let focused=0;disclosure(out,'p').props.onClick({stopPropagation(){},currentTarget:{focus(){focused++;}}});
  assert.equal(focused,1);assert.equal(props.selectedId,'c');assert.deepEqual(ids(view.render(props)),['p']);
});

test('detail exposes kind, direct parent and actual depth with the same fallback title as the table',()=>{
  const view=renderer('src/components/SessionDetail.tsx',{'@/components/CodexStatusPanel':{CodexStatusPanel:'CodexStatusPanel'}});
  const props={session:session('abcdefgh-123','missing',{name:null,sessionKind:'guardian',isSummaryRun:true}),depth:7,parentMissing:true,locale:'ko',t:key=>key,onResume(){}};
  const out=view.render(props);const fields=nodes(out).filter(n=>typeof n.type==='function'&&n.props.label);
  assert.ok(fields.some(n=>n.props.label==='detail.parent'&&n.props.value==='missing'));
  assert.ok(fields.some(n=>n.props.label==='detail.depth'&&n.props.value==='7'));
  assert.match(textOf(out),/kind.guardian/);assert.match(textOf(out),/kind.summary/);assert.match(textOf(out),/list.parentMissing/);
  assert.equal(textOf(nodes(out).find(n=>n.type==='h2')),'kind.summary · abcdefgh');
});

test('saving other settings preserves historical exclusions while exclusion editing is absent',async()=>{
  const view=renderer('src/components/SettingsDialog.tsx');let saved;
  const props={open:true,current:{excludedScanPaths:['preserved'],extraProjectDirs:['extra'],wslAutoDetect:true},locale:'ko',t:key=>key,onSaved(){},onClose(){},onSaveSettings:async patch=>{saved=patch;}};
  const out=view.render(props);assert.doesNotMatch(textOf(out),/스캔 제외 경로/);
  const save=nodes(out).find(n=>n.type==='Button'&&n.props.children==='Save');await save.props.onClick();assert.deepEqual(saved.excludedScanPaths,['preserved']);assert.deepEqual(saved.extraProjectDirs,['extra']);
});


test('app uses table counts, removes visibility toggles and blocks destructive controls for stale data',()=>{
  const item=session('p');const manager={t:(key,p)=>key==='list.counts'?JSON.stringify(p):key,query:'',setQuery(){},loading:false,refreshError:'offline',snapshotStale:true,missingCount:2,canDelete:false,
    locale:'ko',config:{settings:{}},selectedId:'p',selected:item,selectedDepth:4,selectedParentMissing:false,editMode:null,editTarget:null,settingsOpen:false,
    codexCliMissing:false,codexStatus:null,codexStatusLoading:false,selectedForDeleteIds:new Set(['p']),pendingDelete:{sessions:[{sessionId:'p'}],affectedIds:['p']},
    deleting:false,preparingDelete:false,deleteReport:null,filtered:[item,session('c','p')],selectedForDeleteCount:1,visibleSelectedCount:0,displayedCount:1,total:2,workerCount:1,localCount:2,cloudCount:0,
    settingsActions:{},handleVisibleRowsChange(){},guardianCount:2,handleDeleteGuardians(){}};
  const view=renderer('src/App.tsx',{'@/application/useSessionManager':{useSessionManager:()=>manager}});const out=view.render();
  assert.doesNotMatch(textOf(out),/list.showHidden/);assert.match(textOf(out),/"displayed":1/);assert.match(textOf(out),/warning.staleSnapshot/);
  const bulk=nodes(out).find(n=>n.type==='Button'&&n.props.variant==='destructive');assert.equal(bulk.props.disabled,true);
  const guardianBulk=nodes(out).find(n=>n.type==='Button'&&n.props.title==='action.deleteGuardians');
  assert.equal(guardianBulk?.props.disabled,true);assert.equal(guardianBulk.props.onClick,manager.handleDeleteGuardians);
  const table=nodes(out).find(n=>n.props.sessions===manager.filtered);assert.equal(table.props.onVisibleRowsChange,manager.handleVisibleRowsChange);assert.equal(table.props.canDelete,false);
  const detail=nodes(out).find(n=>n.props.session===item);assert.equal(detail.props.depth,4);
});


test('kind search accepts the displayed English labels as well as Korean labels',()=>{
  const view=renderer('src/components/SessionTable.tsx');const state=view.load('src/lib/sessionTableState.ts');
  const all=[session('p'),session('internal','p',{sessionKind:'otherInternal'}),session('summary',null,{isSummaryRun:true})];
  assert.deepEqual(state.filterSessionsWithAncestors(all,'Other internal').map(s=>s.sessionId),['p','internal']);
  assert.deepEqual(state.filterSessionsWithAncestors(all,'Summary run').map(s=>s.sessionId),['summary']);
  assert.deepEqual(state.filterSessionsWithAncestors(all,'기타 내부').map(s=>s.sessionId),['p','internal']);
});

test('the deletion dialog estimates from completed requests and shows elapsed time during scope checks',()=>{
  const dialog=renderer('src/components/DeleteConfirmDialog.tsx',{}, {effects:false});
  let emit;
  const props={open:true,count:100,pending:false,canDelete:true,title:'title',description:'description',confirmLabel:'confirm',
    cancelLabel:'cancel',t:(key,values)=>key+(values?' '+Object.values(values).join('/'):''),onConfirm:onProgress=>{emit=onProgress;}};
  nodes(dialog.render(props)).find(node=>node.type==='Button'&&node.props.variant==='destructive').props.onClick();
  const renderProgress=update=>{emit(update);return dialog.render({...props,pending:true});};
  const status=out=>textOf(nodes(out).find(node=>node.props.role==='status'));
  assert.equal(status(renderProgress({phase:'checkingScope'})),'delete.preparing');
  const single=renderProgress({phase:'deleting',processed:0,total:1,elapsedMs:0});
  assert.match(status(single),/delete.requestProgress 0\/1/);
  assert.doesNotMatch(status(single),/delete.remaining/);
  assert.match(textOf(single),/delete.elapsed 0/);
  const partial=renderProgress({phase:'deleting',processed:2,total:5,elapsedMs:2400});
  assert.match(status(partial),/delete.requestProgress 2\/5/);
  assert.match(status(partial),/delete.remaining 4/);
  assert.equal(status(renderProgress({phase:'verifying'})),'delete.verifying');
});
