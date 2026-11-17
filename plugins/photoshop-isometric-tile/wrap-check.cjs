const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const nodes={};
for(const [,id]of fs.readFileSync(__dirname+'/index.html','utf8').matchAll(/id="([^"]+)"/g))nodes[id]={value:'',checked:false,selectedIndex:0,style:{},children:[],clientWidth:320,clientHeight:240,textContent:'',addEventListener(name,fn){this[name]=fn;},appendChild(child){this.children.push(child);},replaceChild(next,old){this.children[this.children.indexOf(old)]=next;},removeChild(child){this.children.splice(this.children.indexOf(child),1);},set innerHTML(v){this.children=[];},getBoundingClientRect(){return {width:320,height:240};},focus(){}};
Object.assign(nodes.width,{value:'128'});nodes.height.value='64';nodes.columns.value=nodes.rows.value=nodes.zoom.value='1';nodes.mode.selectedIndex=1;nodes['background-mode'].selectedIndex=1;nodes['background-color'].value='#555555';
let doc=null,modal=false,commits=0,rollbacks=0,writes=0,failWrite=false,masked=false,paintCallback,clock=0,interval;
const ps={constants:{LayerKind:{NORMAL:'normal',GROUP:'group'},BlendMode:{NORMAL:'normal'},DocumentMode:{RGB:'RGB'},BitsPerChannelType:{EIGHT:8}},app:{documents:[],get activeDocument(){return doc;}},action:{async batchPlay(){return [{hasUserMask:masked}];},async addNotificationListener(events,fn){assert.deepEqual(Array.from(events),['paint']);paintCallback=fn;},async removeNotificationListener(){paintCallback=null;}},imaging:{async getPixels({documentID,layerID,sourceBounds:b,historyStateID}){
    assert(modal);assert.equal(documentID,doc.id);const layer=doc.layers.find(l=>l.id===layerID);assert(layer,'must read individual layer, never composite');
    const width=b.right-b.left,height=b.bottom-b.top,raw=new Uint8Array(width*height*4),s=layer.boundsNoEffects;
    for(let y=0;y<height;y++)for(let x=0;x<width;x++){const sx=x+b.left-s.left,sy=y+b.top-s.top;if(sx>=0&&sx<layer.width&&sy>=0&&sy<layer.height)raw.set((historyStateID!==undefined&&layer.previousRGBA?layer.previousRGBA:layer.rgba).subarray((sy*layer.width+sx)*4,(sy*layer.width+sx)*4+4),(y*width+x)*4);}
    return {level:0,sourceBounds:b,imageData:{width,height,components:4,colorProfile:'Test RGB',async getData(){return raw;},dispose(){}}};
},async createImageDataFromBuffer(raw,options){return {...options,raw:new Uint8Array(raw),dispose(){}};},async putPixels({documentID,layerID,imageData,targetBounds,replace}){
    assert(modal);assert.equal(documentID,doc.id);assert(replace);assert.equal(imageData.colorProfile,'Test RGB');const layer=doc.layers.find(l=>l.id===layerID);writes++;layer.rgba=new Uint8Array(imageData.raw);layer.width=imageData.width;layer.height=imageData.height;layer.boundsNoEffects={left:targetBounds.left,top:targetBounds.top,right:targetBounds.left+layer.width,bottom:targetBounds.top+layer.height};if(failWrite)throw Error('write failed');
}},core:{async executeAsModal(fn){assert(!modal);modal=true;try{return await fn({isCancelled:false,hostControl:{async suspendHistory(){return doc.layers.map(layer=>({layer,rgba:new Uint8Array(layer.rgba),bounds:{...layer.boundsNoEffects},width:layer.width,height:layer.height}));},async resumeHistory(snapshot,commit){if(commit){commits++;doc.activeHistoryState.id++;}else{rollbacks++;for(const s of snapshot){s.layer.rgba=s.rgba;s.layer.boundsNoEffects=s.bounds;s.layer.width=s.width;s.layer.height=s.height;}}}}});}finally{modal=false;}}}};
const host={require(name){if(name==='photoshop')return ps;if(name==='uxp')return {entrypoints:{setup(){}}};return require(name.startsWith('./')?__dirname+'/'+name.slice(2):name);},document:{getElementById:id=>nodes[id],createElement:()=>({style:{},children:[],addEventListener(){}})},setInterval(fn){interval=fn;return 1;},clearInterval(){},Date:{now:()=>clock},console};
vm.runInNewContext(fs.readFileSync(__dirname+'/main.js','utf8'),host);
const tick=()=>new Promise(resolve=>setImmediate(resolve));
function makeDocument(columns=1,rows=1,left=0,top=0){
    nodes.columns.value=String(columns);nodes.rows.value=String(rows);const width=(columns+rows)*64,height=(columns+rows)*32;
    const layer={id:2,name:'Paint',kind:'normal',blendMode:'normal',visible:true,opacity:100,fillOpacity:100,width:width-left,height:height-top,rgba:new Uint8Array((width-left)*(height-top)*4),boundsNoEffects:{left,top,right:width,bottom:height}};
    const guide={id:3,name:'辅助网格 [UYTile] '+columns+'x'+rows+' 128x64',kind:'normal',visible:true,allLocked:true,width:1,height:1,rgba:Uint8Array.from([7,8,9,255]),boundsNoEffects:{left:0,top:0,right:1,bottom:1}};
    doc={id:1,width,height,mode:'RGB',bitsPerChannel:8,colorProfileName:'Test RGB',activeHistoryState:{id:1},layers:[guide,layer],activeLayers:[layer]};ps.app.documents=[doc];return layer;
}
function setPixel(layer,x,y,color){const b=layer.boundsNoEffects;layer.rgba.set(color,((y-b.top)*layer.width+x-b.left)*4);}
function getPixel(layer,x,y){const b=layer.boundsNoEffects;return Array.from(layer.rgba.subarray(((y-b.top)*layer.width+x-b.left)*4,((y-b.top)*layer.width+x-b.left)*4+4));}
(async()=>{
    await tick();
    // A soft brush crossing the lower-left edge must leave no stale green pixels
    // in the inclusive guide aliases after outside samples have been composited.
    {
        const spec={columns:2,rows:2,tw:128,th:64},w=256,h=128,mask=host.diamondMask(w,h,2,2);
        const before=new Uint8Array(w*h*4);
        for(let i=0;i<w*h;i++)if(mask[i])before.set([20,180,40,255],i*4);
        const raw=new Uint8Array(before);
        for(let y=91;y<104;y++)for(let x=54;x<76;x++)raw.set([230,10,20,128],(y*w+x)*4);
        const result=host.wrapPixels(raw,w,h,0,0,spec,before);
        let pairs=0;
        for(let y=0;y<h;y++)for(let x=0;x<w;x++){
            if(!mask[y*w+x]||host.insideWrapUnit(x,y,spec))continue;
            const target=host.wrappedPixel(x+.5,y+.5,spec),i=(y*w+x)*4,j=(Math.floor(target.y)*w+Math.floor(target.x))*4;
            assert.deepEqual(Array.from(result.rgba.subarray(i,i+4)),Array.from(result.rgba.subarray(j,j+4)),`guide pair ${x},${y}`);pairs++;
        }
        assert(pairs>0);
    }
    for(const [columns,rows]of [[1,1],[3,3],[2,3]]){
        const layer=makeDocument(columns,rows),spec={columns,rows,tw:128,th:64},w=doc.width,h=doc.height,mask=host.diamondMask(w,h,columns,rows);
        // Every outside point lands at an integer-translated pixel inside the unit.
        for(let y=-32;y<h+32;y+=3)for(let x=-64;x<w+64;x+=3){const p=host.wrappedPixel(x+.5,y+.5,spec),tx=Math.floor(p.x),ty=Math.floor(p.y);assert(tx>=0&&tx<w&&ty>=0&&ty<h);assert(mask[ty*w+tx]);assert.equal(p.x%1,.5);assert.equal(p.y%1,.5);}
        setPixel(layer,0,0,[200,10,20,255]);const p=host.wrappedPixel(.5,.5,spec),tx=Math.floor(p.x),ty=Math.floor(p.y);const previous=commits;
        const moved=await host.wrapCurrentLayer();assert.equal(moved,1);assert.equal(commits,previous+1);assert.deepEqual(getPixel(layer,0,0),[0,0,0,0]);assert.deepEqual(getPixel(layer,tx,ty),[200,10,20,255]);assert.deepEqual(Array.from(doc.layers[0].rgba),[7,8,9,255]);
        const oldWrites=writes;assert.equal(await host.wrapCurrentLayer(),0);assert.equal(writes,oldWrites,'wrapping must be idempotent');
    }
    let layer=makeDocument(2,2);setPixel(layer,230,45,[222,3,4,255]);assert.equal(await host.wrapCurrentLayer(),1);assert.deepEqual(getPixel(layer,102,109),[222,3,4,255]);assert.deepEqual(getPixel(layer,102,110),[0,0,0,0]);
    // Regression: the exact lower-left/lower-right guide rows belong to the upper side.
    for(const [x,y,tx,ty]of [[64,97,192,33],[191,97,63,33],[64,100,192,36],[192,100,64,36]]){
        layer=makeDocument(2,2);setPixel(layer,x,y,[90,120,150,255]);
        assert(!host.insideWrapUnit(x,y,{columns:2,rows:2,tw:128,th:64}));
        assert.equal(await host.wrapCurrentLayer(),1);assert.deepEqual(getPixel(layer,x,y),[0,0,0,0]);assert.deepEqual(getPixel(layer,tx,ty),[90,120,150,255]);assert.deepEqual(getPixel(layer,tx,ty-1),[0,0,0,0]);assert.deepEqual(getPixel(layer,tx,ty+1),[0,0,0,0]);
        const visible=host.diamondMask(256,128,2,2);assert.equal(visible[y*256+x],0);assert.equal(visible[ty*256+tx],1);
    }
    layer=makeDocument();setPixel(layer,64,32,[0,0,255,255]);setPixel(layer,0,0,[255,0,0,128]);await host.wrapCurrentLayer();assert.deepEqual(getPixel(layer,64,32),[128,0,127,255]);
    // All four guide-covered boundaries remain intact, as requested.
    for(const [x,y]of [[0,31],[127,31],[62,0],[65,63]]){layer=makeDocument();setPixel(layer,x,y,[30,40,50,128]);assert.equal(await host.wrapCurrentLayer(),1);assert.deepEqual(getPixel(layer,x,y),[30,40,50,128]);const target=host.wrappedPixel(x+.5,y+.5,{columns:1,rows:1,tw:128,th:64});assert.deepEqual(getPixel(layer,Math.floor(target.x),Math.floor(target.y)),[30,40,50,128]);assert.equal(await host.wrapCurrentLayer(),0,'copying a translucent boundary twice must not compound alpha');}
    for(const [x,y,tx,ty]of [[64,96,192,32],[191,96,63,32]]){layer=makeDocument(2,2);setPixel(layer,x,y,[30,40,50,128]);assert.equal(await host.wrapCurrentLayer(),1);assert.deepEqual(getPixel(layer,x,y),[30,40,50,128]);assert.deepEqual(getPixel(layer,tx,ty),[30,40,50,128]);assert.match(nodes['operation-status'].textContent,/搬移 0.*复制 1/);}
    // If the interior edge changed since the previous history, synchronize outward instead.
    layer=makeDocument(2,2);setPixel(layer,64,96,[0,0,200,255]);setPixel(layer,192,32,[0,0,200,255]);layer.previousRGBA=new Uint8Array(layer.rgba);doc.historyStates=[{id:0},{id:1}];setPixel(layer,192,32,[200,0,0,128]);assert.equal(await host.wrapCurrentLayer(),1);assert.deepEqual(getPixel(layer,64,96),[200,0,0,128]);assert.deepEqual(getPixel(layer,192,32),[200,0,0,128]);
    layer=makeDocument(1,1,-64,-32);setPixel(layer,-64,-32,[20,30,40,255]);assert.equal(await host.wrapCurrentLayer(),1);assert.deepEqual(getPixel(layer,-64,-32),[0,0,0,0]);
    layer=makeDocument();setPixel(layer,0,0,[10,20,30,255]);const original=new Uint8Array(layer.rgba);failWrite=true;await assert.rejects(host.wrapCurrentLayer(),/write failed/);failWrite=false;assert.deepEqual(layer.rgba,original);assert.equal(rollbacks,1);
    const oldWrites=writes;layer.allLocked=true;await assert.rejects(host.wrapCurrentLayer(),/锁定/);layer.allLocked=false;masked=true;await assert.rejects(host.wrapCurrentLayer(),/蒙版/);masked=false;doc.bitsPerChannel=16;await assert.rejects(host.wrapCurrentLayer(),/8位/);doc.bitsPerChannel=8;doc.activeLayers=[doc.layers[0]];await assert.rejects(host.wrapCurrentLayer(),/辅助网格/);assert.equal(writes,oldWrites);
    layer=makeDocument();nodes['auto-wrap'].checked=true;await nodes['bind-auto-wrap'].click();assert(paintCallback);assert.match(nodes['wrap-status'].textContent,/等待新笔触/);setPixel(layer,0,0,[11,22,33,255]);doc.activeHistoryState.id++;paintCallback();assert.match(nodes['wrap-status'].textContent,/paint绘画事件/);const beforeDelay=writes;clock+=100;host.tryAutoWrap();assert.equal(writes,beforeDelay);clock+=200;
    // Call auto processing without preview rendering, to isolate history behavior.
    host.tryAutoWrap();await tick();assert.deepEqual(getPixel(layer,64,32),[11,22,33,255]);assert.match(nodes['wrap-status'].textContent,/自动回落完成/);const afterAuto=writes;host.tryAutoWrap();await tick();assert.equal(writes,afterAuto);
    setPixel(layer,0,0,[55,66,77,255]);doc.activeHistoryState.id++;paintCallback();doc.activeHistoryState.id--;clock+=300;host.tryAutoWrap();await tick();assert.equal(writes,afterAuto,'history changes after the paint event must cancel pending auto wrap');assert.match(nodes['wrap-status'].textContent,/已跳过/);
    console.log('Layer wrap checks passed: single/multi lattice mapping, alpha, four-side boundary retention, exact lower-to-upper 2x2 translation, negative bounds, idempotency, one-step history, rollback, layer guards and paint-only auto processing.');
})().catch(error=>{console.error(error);process.exitCode=1;});
