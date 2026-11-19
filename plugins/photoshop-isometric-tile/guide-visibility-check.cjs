const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const source=fs.readFileSync(__dirname+'/main.js','utf8');
(async()=>{for(const visible of [true,false])for(const fails of [false,true]){
const guide={id:2,visible,allLocked:true},doc={id:1,activeHistoryState:{id:3}};let rolledBack=false;
const host={wrapObserved:null,autoWrapPending:null,gridLayers:()=>[{layer:guide}],observeWrapBaseline(){}};
vm.runInNewContext(source.slice(source.indexOf('async function readWithoutGrid('),source.indexOf('function gridRaster(')),host);
const context={hostControl:{async suspendHistory(){return 1;},async resumeHistory(token,commit){assert.equal(commit,false);assert.equal(guide.visible,visible,'visibility restored before rollback');assert.equal(guide.allLocked,true,'lock restored before rollback');rolledBack=true;}}};
const task=host.readWithoutGrid(context,doc,async()=>{assert.equal(guide.visible,false);if(fails)throw Error('capture failed');return 7;},'test');
if(fails)await assert.rejects(task,/capture failed/);else assert.equal(await task,7);assert(rolledBack);
}console.log('Guide visibility checks passed: explicit restore before rollback, visible/hidden guide and capture failure.');})().catch(e=>{console.error(e);process.exitCode=1;});
