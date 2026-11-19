// Exercise event-free pen history detection without Photoshop or a tablet.
const assert=require('node:assert/strict');
const fs=require('node:fs');
const vm=require('node:vm');
const source=fs.readFileSync(__dirname+'/main.js','utf8');
const status={textContent:'',style:{}},enabled={checked:true},bindingLabel={textContent:''},bindingButton={textContent:'',style:{}};
const layerA={id:10,name:'Paint'},layerB={id:20,name:'Paint'},reference={id:30,name:'Reference'};
const doc={id:1,path:'C:/Tiles/A.psd',activeLayers:[layerA],activeHistoryState:{id:1,name:'打开'},historyStates:[{id:1}]};
let now=1000,nextTimer=1;const timers=new Map();
const host={ps:{app:{activeDocument:doc},action:{async addNotificationListener(){throw Error('not supported');}}},
    visible:true,selected:0,operationBusy:false,busy:false,autoWrapPending:null,paintListenerRegistered:false,wrapObserved:null,autoWrapDocument:null,documents:()=>[doc],cache:new Map(),lastKey:"",atlasSpec:()=>({}),autoWrapRuns:0,timer:null,wrapTimer:null,generation:0,pending:false,refreshes:0,refresh(){host.refreshes++;},viewport:()=>({}),setInterval(fn){const id=nextTimer++;timers.set(id,fn);return id;},clearInterval(id){timers.delete(id);},
    $:id=>id==='auto-wrap'?enabled:id==='auto-wrap-document'?bindingLabel:id==='bind-auto-wrap'?bindingButton:status,mode:()=>1,wrapTarget:()=>({doc:host.ps.app.activeDocument,layer:host.ps.app.activeDocument.activeLayers[0]}),Date:{now:()=>now},
    reconcileControls(){},operation:task=>task(),async wrapCurrentLayer(expected){host.executed.push(expected);return 1;},executed:[]};
Object.defineProperty(host,'autoWrapEnabled',{get(){return enabled.checked;},set(value){enabled.checked=value;},configurable:true});
vm.runInNewContext(source.slice(source.indexOf('function setStatus('),source.indexOf('function previewBackground(')),host);
vm.runInNewContext(source.slice(source.indexOf('function wrapObservation()'),source.indexOf('async function newGridDocument')),host);
vm.runInNewContext(source.slice(source.indexOf('function startWrapMonitor()'),source.indexOf('$("refresh").addEventListener')),host);
host.panelReady=true;
host.panels={tilePreview:{content:'controls',root:null,shown:true},tilePreviewView:{content:'view-area',root:null,shown:true}};
vm.runInNewContext(source.slice(source.indexOf('function panelVisibility('),source.indexOf('function reportStartupFailure(')),host);
function history(id,name){doc.activeHistoryState={id,name};if(!doc.historyStates.some(h=>h.id===id))doc.historyStates.push({id});}
(async()=>{
    await host.enableAutoWrap(true);
    assert(enabled.checked,'history fallback must work even if paint listener registration fails');
    assert.equal(host.autoWrapPending,null,'enabling must not wrap old content');
    history(2,'画笔工具');host.pollAutoWrap();
    assert.equal(host.autoWrapPending.historyID,2,'Wacom-style history-only stroke must queue');
    assert.equal(host.autoWrapPending.beforeHistoryID,1,'use the pre-stroke baseline, not an intervening helper history');
    host.tryAutoWrap();assert.equal(host.executed.length,0);
    now+=199;host.tryAutoWrap();assert.equal(host.executed.length,0);
    now++;host.tryAutoWrap();assert.equal(host.executed.length,1);
    history(1,'打开');host.pollAutoWrap();assert.equal(host.autoWrapPending,null);
    history(2,'画笔工具');host.pollAutoWrap();assert.equal(host.autoWrapPending,null,'redo must not wrap again');
    history(3,'移动');host.pollAutoWrap();assert.equal(host.autoWrapPending,null);
    history(4,'菱形回落（当前图层）');host.pollAutoWrap();assert.equal(host.autoWrapPending,null);
    history(5,'Brush Tool');host.pollAutoWrap();assert(host.autoWrapPending);
    doc.activeLayers=[layerB];host.pollAutoWrap();assert.equal(host.autoWrapPending,null,'switching to an unbound layer cancels pending work');
    host.onPaint();assert.equal(host.autoWrapPending,null,'same layer name must not imply binding');
    assert.equal(bindingButton.textContent,'绑定当前图层');
    await host.toggleWrapLayerBinding();
    assert.deepEqual(Array.from(host.autoWrapDocument.layers.keys()),[10,20],'binding B must retain A');
    assert.equal(bindingButton.textContent,'解绑当前图层');
    history(6,'画笔工具');host.operationBusy=true;host.pollAutoWrap();assert.equal(host.autoWrapPending,null);
    host.observeWrapBaseline();host.operationBusy=false;host.pollAutoWrap();assert.equal(host.autoWrapPending,null,'own operation baseline prevents feedback');
    host.onPaint();assert.equal(host.autoWrapPending.delay,200,'paint notifications retain fast path');
    history(7,'画笔工具');host.pollAutoWrap();
    assert.equal(host.autoWrapPending.beforeHistoryID,6);
    const firstQueuedAt=host.autoWrapPending.at;now+=100;
    history(8,'画笔工具');host.pollAutoWrap();
    assert.equal(host.autoWrapPending.at,firstQueuedAt,'successive strokes must not postpone the execution deadline indefinitely');
    assert.equal(host.autoWrapPending.beforeHistoryID,6,'rapid successive strokes retain the earliest unprocessed baseline');
    const other={...doc,id:22,path:'C:/Other/Other.psd',title:'Other.psd',activeHistoryState:{id:99,name:'画笔工具'},historyStates:[{id:99}]};
    host.documents=()=>[doc,other];host.ps.app.activeDocument=other;
    host.pollAutoWrap();host.onPaint();assert.equal(host.autoWrapPending,null,'a different document must never auto-wrap, even with matching layers/history');
    assert.equal(host.autoWrapDocument.id,1,'switching documents must not change the binding');
    host.ps.app.activeDocument=doc;history(9,'画笔工具');host.pollAutoWrap();
    assert.equal(host.autoWrapPending,null,'returning from another document starts a fresh baseline');
    history(10,'画笔工具');host.pollAutoWrap();assert.equal(host.autoWrapPending.documentID,1);
    // Pause preserves both bindings, and an unbound reference is never processed.
    enabled.checked=false;await host.enableAutoWrap(false);
    assert.deepEqual(Array.from(host.autoWrapDocument.layers.keys()),[10,20]);assert.equal(host.autoWrapPending,null);
    doc.activeLayers=[reference];enabled.checked=true;await host.enableAutoWrap(false);
    history(11,'画笔工具');host.pollAutoWrap();host.onPaint();assert.equal(host.autoWrapPending,null);
    // Switching between both bound layers continues without rebinding.
    for(const [layer,id]of [[layerA,12],[layerB,13],[layerA,14]]){
        doc.activeLayers=[layer];host.pollAutoWrap();assert.equal(bindingButton.textContent,'解绑当前图层');
        history(id,'画笔工具');host.pollAutoWrap();assert.equal(host.autoWrapPending.layerID,layer.id);
        now+=201;host.tryAutoWrap();await new Promise(resolve=>setImmediate(resolve));
        assert.equal(host.executed.at(-1).layerID,layer.id);
    }
    // Unbinding A cancels its pending task but leaves B and the enabled state intact.
    history(15,'画笔工具');host.pollAutoWrap();assert(host.autoWrapPending);
    await host.toggleWrapLayerBinding();assert.equal(bindingButton.textContent,'绑定当前图层');
    assert.deepEqual(Array.from(host.autoWrapDocument.layers.keys()),[20]);assert(enabled.checked);assert.equal(host.autoWrapPending,null);
    host.onPaint();assert.equal(host.autoWrapPending,null);
    doc.activeLayers=[layerB];host.pollAutoWrap();history(16,'画笔工具');host.pollAutoWrap();assert.equal(host.autoWrapPending.layerID,20);
    host.ps.app.activeDocument=other;await host.enableAutoWrap(true);
    assert.equal(host.autoWrapDocument.id,22,'explicit rebind changes the target');
    assert.equal(host.autoWrapPending,null,'rebind drops previous queued work');
    host.documents=()=>[doc];host.ps.app.activeDocument=doc;host.pollAutoWrap();host.onPaint();
    assert.equal(host.autoWrapPending,null,'closed bound document cannot fall through to another document');
    const sameName={...other,id:23,path:'C:/Different/Other.psd'};
    host.documents=()=>[doc,sameName];host.ps.app.activeDocument=sameName;host.pollAutoWrap();
    assert.equal(host.autoWrapDocument.id,22,'matching title is insufficient for automatic restoration');
    const reopened={...other,id:24,path:'c:\\OTHER\\Other.psd',activeHistoryState:{id:1,name:'打开'},historyStates:[{id:1}],activeLayers:[{id:240,name:'Paint'}]};
    host.documents=()=>[doc,reopened];host.ps.app.activeDocument=reopened;
    host.pollAutoWrap();assert.equal(host.autoWrapDocument.id,24,'the same saved file can reopen with a new document ID');
    assert.equal(host.autoWrapPending,null,'restoration resets history and does not replay old work');
    reopened.activeHistoryState={id:2,name:'画笔工具'};reopened.historyStates.push({id:2});host.pollAutoWrap();
    assert.equal(host.autoWrapPending,null,'a new layer ID must not inherit a same-named binding after reopening');
    await host.toggleWrapLayerBinding();reopened.activeHistoryState={id:3,name:'画笔工具'};reopened.historyStates.push({id:3});host.pollAutoWrap();
    assert.equal(host.autoWrapPending.documentID,24);assert.equal(host.autoWrapPending.layerID,240);
    assert.equal(host.autoWrapPending.beforeHistoryID,2,'explicit binding establishes a fresh baseline');
    // Reopen while a task is pending must clear old layer/history IDs before execution.
    const again={...reopened,id:25,activeHistoryState:{id:10,name:'打开'},historyStates:[{id:10}]};
    host.documents=()=>[again];host.ps.app.activeDocument=again;now+=1000;host.tryAutoWrap();
    assert.equal(host.autoWrapDocument.id,25);assert.equal(host.autoWrapPending,null);
    const beforeHiddenRuns=host.executed.length;
    host.start();
    host.panelVisibility('tilePreview',null,false);
    assert.equal(host.visible,true,'collapsing settings must leave the preview running');assert(enabled.checked);assert(host.wrapTimer);
    const beforeHiddenRefresh=host.refreshes;
    host.panelVisibility('tilePreviewView',null,false);
    assert.equal(host.visible,false);assert.equal(host.timer,null);assert(host.wrapTimer,'hiding must retain the independent auto-wrap monitor');
    again.activeHistoryState={id:11,name:'画笔工具'};again.historyStates.push({id:11});timers.get(host.wrapTimer)();
    now+=201;timers.get(host.wrapTimer)();await new Promise(resolve=>setImmediate(resolve));
    assert.equal(host.executed.length,beforeHiddenRuns+1,'hidden panel must still process the bound document');
    host.request(true);assert.equal(host.visible,false);assert.equal(host.refreshes,beforeHiddenRefresh,'background completion must not reopen or redraw the panel');
    const originalWrap=host.wrapCurrentLayer;host.wrapCurrentLayer=async()=>{const error=Error('host is in a modal state');error.number=9;throw error;};
    again.activeHistoryState={id:12,name:'画笔工具'};again.historyStates.push({id:12});timers.get(host.wrapTimer)();now+=201;timers.get(host.wrapTimer)();await new Promise(resolve=>setImmediate(resolve));
    assert.equal(host.autoWrapPending.retries,1,'temporary modal rejection must retain the task');
    host.wrapCurrentLayer=originalWrap;now+=301;timers.get(host.wrapTimer)();await new Promise(resolve=>setImmediate(resolve));
    assert.equal(host.autoWrapPending,null);assert.equal(host.executed.length,beforeHiddenRuns+2);
    // Unbinding during a modal retry must prevent the queued write from returning.
    host.wrapCurrentLayer=async()=>{const error=Error('busy');error.number=9;throw error;};
    again.activeHistoryState={id:13,name:'画笔工具'};again.historyStates.push({id:13});host.pollAutoWrap();now+=201;host.tryAutoWrap();await new Promise(resolve=>setImmediate(resolve));
    assert(host.autoWrapPending);await host.toggleWrapLayerBinding();assert.equal(host.autoWrapPending,null);
    host.wrapCurrentLayer=originalWrap;const beforeUnbound=host.executed.length;now+=500;host.tryAutoWrap();assert.equal(host.executed.length,beforeUnbound);
    host.destroyPanel();assert.equal(host.wrapTimer,null);assert.equal(host.autoWrapDocument,null);assert.equal(enabled.checked,false);
    assert.equal(timers.size,0,'destroy must release both timers');
    enabled.checked=true;await host.enableAutoWrap(false);assert.equal(enabled.checked,false,'checkbox alone must not silently bind the active document');
    enabled.checked=false;await host.enableAutoWrap(true);host.tryAutoWrap();assert.equal(host.autoWrapPending,null);assert.equal(host.autoWrapDocument,null);
    enabled.checked=true;host.wrapTarget=()=>{throw Error('网格不匹配');};await host.enableAutoWrap(true);
    assert.equal(status.style.color,'#ff6e6e','enable failures must be red');
    enabled.checked=false;await host.enableAutoWrap(true);assert.equal(status.style.display,'none','normal background status remains hidden');
    console.log('Auto-wrap checks passed: cumulative layer bindings, bound/unbound switches, per-layer unbind, pause/resume, reopening IDs, pending/retry cancellation, hidden monitoring and history/event fallback.');
})().catch(error=>{console.error(error);process.exitCode=1;});
