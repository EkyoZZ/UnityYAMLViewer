const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const html=fs.readFileSync(__dirname+'/index.html','utf8'),nodes={};
for(const [,id]of html.matchAll(/id="([^"]+)"/g))nodes[id]={value:'',checked:false,selectedIndex:0,style:{},children:[],textContent:'',addEventListener(e,fn){this[e]=fn;},appendChild(n){this.children.push(n);},removeChild(n){this.children.splice(this.children.indexOf(n),1);},set innerHTML(_){this.children=[];}};
nodes.mode.selectedIndex=1;nodes.width.value='16';nodes.height.value='8';nodes.columns.value=nodes.rows.value='1';
let doc,modal=false,commits=0,rollbacks=0,disposed=0,failWrite=false,failResize=false;
function editableTarget(){
    assert.equal(doc.activeLayers.length,1);assert.equal(doc.activeLayers[0].id,3,'only the helper is selected while expanding');
    const guide=doc.activeLayers[0];
    if(guide.allLocked||guide.pixelsLocked||guide.positionLocked||guide.transparentPixelsLocked)throw Error('target sheet is protected and cannot be modified');
    assert.equal(doc.layers.find(l=>l.id===4).allLocked,true,'reference lock must never be removed');
}
const ps={app:{get documents(){return doc?[doc]:[];},get activeDocument(){return doc;}},constants:{LayerKind:{NORMAL:'normal',GROUP:'group'},BlendMode:{NORMAL:'normal',MULTIPLY:'multiply'},DocumentMode:{RGB:'RGB'},BitsPerChannelType:{EIGHT:8},AnchorPosition:{TOPRIGHT:'topRight',TOPLEFT:'topLeft'},ElementPlacement:{PLACEBEFORE:'before'}},
    core:{async executeAsModal(fn){assert(!modal);modal=true;try{return await fn({isCancelled:false,hostControl:{async suspendHistory(){return {width:doc.width,height:doc.height,active:doc.activeLayers.slice(),order:doc.layers.slice(),layers:doc.layers.map(l=>({layer:l,name:l.name,allLocked:l.allLocked,pixelsLocked:l.pixelsLocked,positionLocked:l.positionLocked,transparentPixelsLocked:l.transparentPixelsLocked,blendMode:l.blendMode,bounds:{...l.boundsNoEffects},rgba:new Uint8Array(l.rgba)}))};},async resumeHistory(s,commit){if(commit){commits++;doc.activeHistoryState.id++;return;}rollbacks++;doc.width=s.width;doc.height=s.height;doc.layers=s.order;doc.activeLayers=s.active;for(const l of s.layers){Object.assign(l.layer,{name:l.name,allLocked:l.allLocked,pixelsLocked:l.pixelsLocked,positionLocked:l.positionLocked,transparentPixelsLocked:l.transparentPixelsLocked,blendMode:l.blendMode,boundsNoEffects:l.bounds,rgba:l.rgba});}}}});}finally{modal=false;}}},
    imaging:{async createImageDataFromBuffer(raw,options){assert(modal);return {...options,raw,dispose(){disposed++;}};},async putPixels({layerID,imageData,targetBounds,replace}){assert(modal);editableTarget();assert(replace);assert.equal(layerID,3,'expansion must write only the guide layer');const guide=doc.layers.find(l=>l.id===layerID);guide.rgba=new Uint8Array(imageData.raw);guide.boundsNoEffects={...targetBounds,right:targetBounds.left+imageData.width,bottom:targetBounds.top+imageData.height};if(failWrite)throw Error('simulated write failure');}},
    action:{async addNotificationListener(){},async batchPlay(commands){for(const command of commands){if(command._obj==='get')return [{}];const layer=doc.layers.find(l=>l.id===command._target[0]._id);doc.activeLayers=command.selectionModifier?[...doc.activeLayers,layer]:[layer];}return commands.map(()=>({}));}}};
const host={require(name){if(name==='photoshop')return ps;if(name==='uxp')return {entrypoints:{setup(){}}};return require(name.startsWith('./')?__dirname+'/'+name.slice(2):name);},document:{getElementById:id=>nodes[id],querySelectorAll:()=>Object.entries(nodes).map(([id,node])=>Object.assign(node,{id})),createElement:()=>({style:{},addEventListener(){}})},setInterval(){return 1;},clearInterval(){},console};
vm.runInNewContext(fs.readFileSync(__dirname+'/main.js','utf8'),host);
function makeDoc(columns,rows,width=(columns+rows)*8,height=(columns+rows)*4,tw=16,th=8){
    nodes.width.value=String(tw);nodes.height.value=String(th);
    nodes.columns.value=String(columns);nodes.rows.value=String(rows);
    const rw=(columns+rows)*tw/2,rh=(columns+rows)*th/2,left=Math.floor((width-rw)/2),top=Math.floor((height-rh)/2);
    const rgba=new Uint8Array(rw*rh*4),mask=host.diamondMask(rw,rh,columns,rows);
    for(let i=0;i<mask.length;i++)if(mask[i])rgba.set([i%255,110,90,255],i*4);
    const paint={id:2,name:'Paint',kind:'normal',visible:true,opacity:100,fillOpacity:100,blendMode:'normal',rgba,boundsNoEffects:{left,top,right:left+rw,bottom:top+rh}};
    const guide={...paint,id:3,name:`辅助网格 [UYTile] ${columns}x${rows} ${tw}x${th}`,allLocked:true,pixelsLocked:true,positionLocked:true,transparentPixelsLocked:true,blendMode:'multiply',rgba:Uint8Array.from([5,6,7,255]),move(reference){editableTarget();doc.layers.splice(doc.layers.indexOf(this),1);doc.layers.splice(doc.layers.indexOf(reference),0,this);}};
    for(const key of ['allLocked','pixelsLocked','positionLocked','transparentPixelsLocked']){
        let locked=guide[key];
        Object.defineProperty(guide,key,{enumerable:true,configurable:true,get(){return locked;},set(value){
            if(value===false&&!doc.activeLayers.some(layer=>layer.id===guide.id))return;
            locked=value;
        }});
    }
    const reference={...paint,id:4,name:'Reference',allLocked:true,rgba:Uint8Array.from([10,20,30,255]),boundsNoEffects:{left:0,top:0,right:1,bottom:1}};
    doc={id:1,title:'Tile.psd',path:'C:/Tile.psd',width,height,mode:'RGB',bitsPerChannel:8,layers:[reference,guide,paint],activeLayers:[paint],activeHistoryState:{id:1},historyStates:[{id:1}],async resizeCanvas(w,h,anchor){assert(modal);editableTarget();assert(w>=this.width&&h>=this.height,'never crop existing pixels');const dx=anchor==='topRight'?w-this.width:0;for(const l of this.layers){l.boundsNoEffects={...l.boundsNoEffects,left:l.boundsNoEffects.left+dx,right:l.boundsNoEffects.right+dx};}this.width=w;this.height=h;if(failResize)throw Error('simulated canvas failure');}};
    return {paint,guide,reference};
}
(async()=>{
    nodes['document-settings-toggle'].click();assert.equal(nodes['document-settings'].style.display,'none');
    nodes['document-settings-toggle'].click();assert.equal(nodes['document-settings'].style.display,'flex');assert.equal(nodes.width.value,'16');
    for(const [columns,rows,nextColumns,nextRows,w,h]of [[1,1,2,2,16,8],[1,1,2,1,16,8],[1,1,1,2,16,8],[2,3,4,4,103,75]]){
        const {paint,guide,reference}=makeDoc(columns,rows,w,h),before=new Uint8Array(paint.rgba),refBefore=new Uint8Array(reference.rgba);
        await nodes['bind-auto-wrap'].click();
        if(!host.isWrapLayerBound(doc.id,paint.id))await nodes['bind-auto-wrap'].click();
        const old=host.atlasSpec(doc),bounds=[];
        for(let r=0;r<rows;r++)for(let c=0;c<columns;c++)bounds.push({r,c,b:host.cellBounds(r,c,old)});
        nodes.columns.value=String(nextColumns);nodes.rows.value=String(nextRows);
        const plan=host.gridExpansion(doc),beforeCommits=commits;await host.expandGridDocument();
        assert.equal(commits,beforeCommits+1);assert.equal(doc.width,w+(nextColumns+nextRows-columns-rows)*8);assert.equal(doc.height,h+(nextColumns+nextRows-columns-rows)*4);
        const current=host.atlasSpec(doc);assert.equal(current.left,old.left);assert.equal(current.top,old.top);
        for(const {r,c,b}of bounds){const after=host.cellBounds(r,c,current);assert.equal(after.left,b.left+plan.leftGrowth);assert.equal(after.top,b.top);}
        assert.deepEqual(paint.rgba,before,'paint pixels are never resampled');assert.deepEqual(reference.rgba,refBefore,'references remain intact');assert(reference.allLocked);
        assert.equal(paint.boundsNoEffects.left,old.left+plan.leftGrowth);assert.equal(paint.boundsNoEffects.top,old.top);
        assert.equal(guide.name,`辅助网格 [UYTile] ${nextColumns}x${nextRows} 16x8`);assert(guide.allLocked);assert.equal(guide.blendMode,'multiply');assert.equal(doc.layers[0].id,guide.id);
        assert.equal(doc.activeLayers[0].id,paint.id);assert(host.isWrapLayerBound(doc.id,paint.id),'expansion preserves layer IDs and their bindings');
        const again=commits;await host.expandGridDocument();assert.equal(commits,again,'same-size expansion is a no-op');
    }
    // Match the reported 256x128 / 2x2 case, including a locked current selection.
    for(const selection of ['guide','reference','both']){
        const {guide,paint,reference}=makeDoc(2,2,256,128,128,64),original=new Uint8Array(paint.rgba);
        doc.activeLayers=selection==='guide'?[guide]:selection==='reference'?[reference]:[guide,reference];const originalSelection=doc.activeLayers.slice();
        nodes.columns.value=nodes.rows.value='3';await host.expandGridDocument();
        assert.equal(doc.width,384);assert.equal(doc.height,192);assert.match(guide.name,/3x3 128x64$/);
        assert.equal(guide.rgba.length,384*192*4);assert.deepEqual(paint.rgba,original);assert.deepEqual(doc.activeLayers,originalSelection);
        assert(guide.allLocked&&guide.positionLocked&&guide.pixelsLocked&&guide.transparentPixelsLocked);assert(reference.allLocked);
    }
    const {guide,paint}=makeDoc(1,1,73,51),oldBounds={...paint.boundsNoEffects},oldName=guide.name,oldPixels=new Uint8Array(guide.rgba);
    nodes.columns.value=nodes.rows.value='2';failWrite=true;
    await assert.rejects(host.expandGridDocument(),/simulated/);failWrite=false;
    assert.equal(rollbacks,1);assert.equal(doc.width,73);assert.equal(doc.height,51);assert.equal(guide.name,oldName);assert.deepEqual(guide.rgba,oldPixels);assert.deepEqual(paint.boundsNoEffects,oldBounds);assert(guide.allLocked);
    assert(guide.positionLocked&&guide.pixelsLocked&&guide.transparentPixelsLocked);assert.deepEqual(doc.activeLayers,[paint]);
    failResize=true;await assert.rejects(host.expandGridDocument(),/扩大画布.*simulated canvas failure/);failResize=false;
    assert.equal(rollbacks,2);assert.equal(doc.width,73);assert.equal(doc.height,51);assert.deepEqual(paint.boundsNoEffects,oldBounds);assert.deepEqual(doc.activeLayers,[paint]);assert(guide.allLocked&&guide.positionLocked&&guide.pixelsLocked&&guide.transparentPixelsLocked);
    nodes.width.value='32';assert.throws(()=>host.gridExpansion(doc),/单格尺寸/);nodes.width.value='16';
    makeDoc(2,2);nodes.columns.value='1';assert.throws(()=>host.gridExpansion(doc),/减少/);
    assert(disposed>=5);
    console.log('Expansion checks passed: 256x128 2x2 to 384x192 3x3 with locked selections, helper-only unlocking, canvas/write failure rollback, 1x1 and oversized canvas expansion, original pixels/locks/bindings and selection restoration.');
})().catch(error=>{console.error(error);process.exitCode=1;});
