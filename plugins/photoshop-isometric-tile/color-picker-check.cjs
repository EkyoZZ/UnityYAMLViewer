const assert=require('node:assert/strict');
const fs=require('node:fs');
const vm=require('node:vm');
const source=fs.readFileSync(__dirname+'/main.js','utf8');
const nodes={'background-color':{value:'#123456'},'background-error':{textContent:'',style:{}}};
let result={value:true,RGBFloatColor:{red:255,grain:127.5,blue:0}},calls=0,updates=0;
const host={operationBusy:false,busy:false,$:id=>nodes[id],previewBackground(){updates++;},ps:{
    core:{async executeAsModal(fn){return fn();}},
    action:{async batchPlay(commands){calls++;assert.equal(commands[0]._obj,'showColorPicker');assert.equal(commands[0].color.red,0x12);return [result];}}
}};
vm.runInNewContext(source.slice(source.indexOf('function setStatus('),source.indexOf('function previewBackground(')),host);
vm.runInNewContext(source.slice(source.indexOf('async function pickBackgroundColor()'),source.indexOf('function mode()')),host);
(async()=>{
    await host.pickBackgroundColor();assert.equal(nodes['background-color'].value,'#FF8000');assert.equal(updates,1);
    nodes['background-color'].value='#123456';result={value:false};await host.pickBackgroundColor();assert.equal(nodes['background-color'].value,'#123456');assert.equal(updates,1);
    result={color:{red:4,green:5,blue:6}};await host.pickBackgroundColor();assert.equal(nodes['background-color'].value,'#040506');
    nodes['background-color'].value='#123456';result={_obj:'error',result:-128};await host.pickBackgroundColor();assert.equal(nodes['background-color'].value,'#123456');
    result={color:{red:NaN,grain:2,blue:3}};await host.pickBackgroundColor();assert.equal(nodes['background-color'].value,'#123456');assert.match(nodes['background-error'].textContent,/RGB/);assert.equal(host.operationBusy,false);
    host.busy=true;const before=calls;await host.pickBackgroundColor();assert.equal(calls,before);
    console.log('Color picker checks passed: initial RGB, modal result, green/grain compatibility, HEX synchronization, cancel, invalid result and busy guard.');
})().catch(error=>{console.error(error);process.exitCode=1;});
