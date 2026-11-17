// Host-independent regression checks for startup and panel-size rendering.
const assert=require('node:assert/strict');
const fs=require('node:fs');
const vm=require('node:vm');
const zlib=require('node:zlib');
const png=require('./png.js');
const {unfilter}=require('./png-test-utils.cjs');
function element(){return {style:{},children:[],value:'',checked:false,textContent:'',clientWidth:320,clientHeight:220,
    focus(){this.focusCount=(this.focusCount||0)+1;},addEventListener(event,fn,options){this[event]=fn;this[event+"Options"]=options;},appendChild(child){this.children.push(child);},replaceChild(next,old){this.children[this.children.indexOf(old)]=next;},removeChild(child){this.children.splice(this.children.indexOf(child),1);},
    getBoundingClientRect(){return {width:this.clientWidth,height:this.clientHeight};},
    set innerHTML(value){this.children=[];}};}
const nodes=Object.fromEntries(['wrap-last-run','auto-wrap-document','bind-auto-wrap','wrap-status','wrap-layer','auto-wrap','preview-stage','zoom-reset','stagger-mode','stagger-settings','stagger-columns','stagger-rows','convert-single-document','zoom-slider','preview-background','show-grid','background-mode','background-color','background-color-row','background-swatch','background-error','version','preview','status','width','height','width-label','height-label','zoom','elevated','live','refresh','mode','columns','rows','preset-2','preset-3','preset-4','slots','slot-controls','bind','unbind','selected-slot','document-picker','document-options','reload-documents','show-slots','multi-document-controls','atlas-actions','grid-size','operation-status','new-grid-document','export-cell','export-grid'].map(k=>[k,element()]));
nodes['stagger-mode'].selectedIndex=0;nodes['stagger-columns'].value='2';nodes['stagger-rows'].value='2';nodes.mode.selectedIndex=0;nodes['background-mode'].selectedIndex=0;nodes['background-color'].value='#555555';
nodes.width.value='128';nodes.height.value='64';nodes.zoom.value='1';nodes.live.checked=true;
nodes.columns.value='2';nodes.rows.value='2';nodes['show-slots'].checked=true;
let interval,hooks,reads=0,disposals=0,guideDisposals=0,failRead=false;const readIds=[],readOptions=[],exportFolders=[];
let modalDepth=0,rollbackChangesHistoryID=false;
const ps={app:{documents:[]},imaging:{async getPixels(options){
    assert(modalDepth>0, "pixel reads must execute in modal scope even without helpers");
    reads++;readIds.push(options.documentID);readOptions.push(options);
    const doc=ps.app.documents.find(d=>d.id===options.documentID);
    for(const layer of doc.layers||[])if((layer.name||'').startsWith('辅助网格 [UYTile]'))assert.equal(layer.visible,false,'helper must be hidden during capture');
    if(failRead)throw new Error('Simulated pixel read failure');
    const bounds=options.sourceBounds,w=options.targetSize?options.targetSize.width:bounds.right-bounds.left,h=options.targetSize?options.targetSize.height:bounds.bottom-bounds.top;
    return {level:0,sourceBounds:bounds,imageData:{width:w,height:h,components:4,async getData(){return new Uint8Array(w*h*4).fill(options.documentID===1?255:128);},dispose(){disposals++;}}};
}}};
ps.constants={LayerKind:{GROUP:'group'},NewDocumentMode:{RGB:'RGBColorMode'},DocumentFill:{TRANSPARENT:'transparent'},BlendMode:{MULTIPLY:'multiply'},ElementPlacement:{PLACEBEFORE:'placeBefore',PLACEAFTER:'placeAfter'}};
ps.core={async executeAsModal(fn){assert.equal(modalDepth,0);modalDepth++;try{return await fn({isCancelled:false,hostControl:{async suspendHistory(){return ps.app.documents.flatMap(d=>(d.layers||[]).map(layer=>({layer,visible:layer.visible,locked:layer.allLocked})));},async resumeHistory(states,commit){assert.equal(commit,false);if(rollbackChangesHistoryID)ps.app.activeDocument.activeHistoryState.id++;for(const s of states){s.layer.visible=s.visible;s.layer.allLocked=s.locked;}}}});}finally{modalDepth--;}}};
ps.imaging.createImageDataFromBuffer=async(raw,options)=>({raw,...options,dispose(){guideDisposals++;}});
ps.imaging.putPixels=async({documentID,layerID,imageData,targetBounds})=>{const doc=ps.app.documents.find(d=>d.id===documentID),layer=doc.layers.find(l=>l.id===layerID);layer.gridPixels=imageData.raw;layer.pixelSize=[imageData.width,imageData.height];layer.targetBounds=targetBounds;};
ps.action={async batchPlay(commands){ps.app.activeDocument.selectedLayerID=commands[0]._target[0]._id;}};
const uxp={entrypoints:{setup(v){hooks=v;}},storage:{formats:{binary:'binary'},localFileSystem:{async getFolder(){return {async createFolder(name){const folder={name,nativePath:'mock/'+name,files:new Map(),async createFile(filename,options){assert.equal(options.overwrite,false);assert(!this.files.has(filename));return {write:async bytes=>this.files.set(filename,Buffer.from(bytes))};}};exportFolders.push(folder);return folder;}};}}}};
const host={
    require(name){if(name==='photoshop')return ps;if(name==='uxp')return uxp;if(name==='./png.js')return png;if(name==='./manifest.json')return require('./manifest.json');throw Error(name);},
    document:{getElementById:id=>nodes[id],createElement:()=>element()},
    setInterval(fn){interval=fn;return 1;},clearInterval(){},console,
};
vm.runInNewContext(fs.readFileSync(__dirname+'/main.js','utf8'),host);
const tick=()=>new Promise(resolve=>setImmediate(resolve));
function decodedPNG(bytes){const width=bytes.readUInt32BE(16),height=bytes.readUInt32BE(20),chunks=[];for(let p=8;p<bytes.length;){const n=bytes.readUInt32BE(p);if(bytes.toString('ascii',p+4,p+8)==='IDAT')chunks.push(bytes.subarray(p+8,p+8+n));p+=n+12;}return {width,height,raw:unfilter(zlib.inflateSync(Buffer.concat(chunks)),width,height)};}
// Offset selection is evaluated in lattice cells, including negative repeat groups.
const plain=value=>JSON.parse(JSON.stringify(value));
assert.deepEqual(plain(host.staggerSlot(4,3,4,3,{columns:1,rows:2})),{column:3,row:1});
assert.deepEqual(plain(host.staggerSlot(4,3,4,3,{columns:2,rows:1})),{column:2,row:2});
assert.deepEqual(plain(host.staggerSlot(-1,-1,4,3,{columns:1,rows:2})),{column:2,row:0});
for(const [columns,rows]of [[4,3],[1,2],[2,1]])for(const shift of [{columns:0,rows:0},{columns:1,rows:2},{columns:2,rows:1}])for(let j=-8;j<=8;j++)for(let i=-8;i<=8;i++){
    const slot=host.staggerSlot(i,j,columns,rows,shift),repeat=host.staggerSlot(i+2*columns,j-2*rows,columns,rows,shift);
    assert(slot.column>=0&&slot.column<columns&&slot.row>=0&&slot.row<rows);assert.deepEqual(plain(slot),plain(repeat));
}
// Coverage includes every visible guide pixel, excluding the rectangle corners.
for(const [width,height]of [[128,64],[64,128]]){
    const mask=host.diamondMask(width,height),guide=host.gridRaster(width,height,{columns:1,rows:1,tw:width,th:height},true);
    assert.equal(mask[0],0);assert.equal(mask[width*height-1],0);
    for(let index=0;index<mask.length;index++)if(guide[index*4+3])assert.equal(mask[index],1,'no guide-covered pixel may be clipped');
}
// Independent pixel-center expectations: symmetric tips, no downward raster bias.
for(const [width,height]of [[128,64],[64,128]]){
    const image=host.gridRaster(width,height,{columns:1,rows:1,tw:width,th:height},true);
    const alpha=(x,y)=>image[(y*width+x)*4+3];
    for(let y=0;y<height;y++)for(let x=0;x<width;x++){
        assert.equal(alpha(x,y),alpha(width-1-x,height-1-y),'diamond must be symmetric about its pixel-center origin');
        assert.equal(alpha(x,y),alpha(width-1-x,y),'left and right slopes must align');
    }
    assert(alpha(width/2-1,0)>0);assert(alpha(width/2,0)>0);
    assert(alpha(width/2-1,height-1)>0);assert(alpha(width/2,height-1)>0);
}
// Compare the actual PSD guide raster with preview overlay pixels, across zoom levels.
for(const spec of [{columns:2,rows:2,tw:128,th:64},{columns:3,rows:5,tw:32,th:16}]){
    const width=(spec.columns+spec.rows)*spec.tw/2,height=(spec.columns+spec.rows)*spec.th/2;
    const guide=host.gridRaster(width,height,spec);let previousURL;
    for(const zoom of [.5,1,2,4]){
        const image=host.previewGridImage(width*zoom,height*zoom,spec.columns,spec.rows,0,0,width,height);
        if(previousURL)assert.equal(image.src,previousURL,'zoom must not move rasterized grid lines');previousURL=image.src;
        const decoded=decodedPNG(Buffer.from(image.src.split(',')[1],'base64'));
        assert.equal(decoded.width,width);assert.equal(decoded.height,height);
        for(let y=0;y<height;y++)for(let x=0;x<width;x++)assert.equal(decoded.raw[y*(width*4+1)+1+x*4+3]>0,guide[(y*width+x)*4]===70,'preview line must match PSD guide pixel');
    }
}
(async()=>{
    await tick();assert.match(nodes.status.textContent,/请打开/);assert.equal(typeof interval,'function');
    const background=nodes['preview-background'],append=background.appendChild;
    nodes.preview.clientWidth=641;nodes.preview.clientHeight=579;
    background.appendChild=function(){throw new Error('Simulated background layout failure');};interval();await tick();
    assert.match(nodes.status.textContent,/Simulated background layout failure/);
    background.appendChild=append;interval();await tick();assert.match(nodes.status.textContent,/请打开/);
    nodes.preview.clientWidth=320;nodes.preview.clientHeight=220;interval();await tick();
    nodes.mode.selectedIndex=2;nodes.columns.value='1';nodes.rows.value='1';interval();await tick();assert.equal(nodes.slots.children.length,1);assert.equal(nodes['slot-controls'].style.display,'block');
    nodes.mode.selectedIndex=-1;assert.equal(host.mode(),2,'transient dropdown state must preserve selected mode');
    nodes.mode.selectedIndex=0;interval();await tick();assert.equal(nodes['slot-controls'].style.display,'none');
    nodes.columns.value='2';nodes.rows.value='2';interval();await tick();
    assert.equal(nodes.version.textContent,'版本 '+require('./manifest.json').version);
    const docA={id:1,width:128,height:128,title:'Test',activeHistoryState:{id:1}};
    ps.app.documents=[docA];ps.app.activeDocument=docA;
    nodes.refresh.click();await tick();assert.equal(reads,1);assert.match(nodes.status.textContent,/Test/);
    assert(nodes.preview.children.length>0);assert.equal(disposals,1);
    const stableImage=nodes.preview.children[0];
    nodes['preview-stage'].mouseenter();nodes['preview-stage'].mousedown();assert.equal(nodes['preview-stage'].focusCount,2);assert.equal(nodes['preview-stage'].wheelOptions.capture,true);assert.equal(nodes['preview-stage'].wheelOptions.passive,false);
    let prevented=0,stopped=0;const wheel=(deltaY,deltaMode=0)=>nodes['preview-stage'].wheel({deltaY,deltaMode,preventDefault(){prevented++;},stopPropagation(){stopped++;}});
    wheel(-120);await tick();assert(Number(nodes.zoom.value)>1);assert.equal(nodes.zoom.value,nodes['zoom-slider'].value);assert.equal(reads,1);assert.equal(nodes.preview.children[0],stableImage);
    const enlarged=Number(nodes.zoom.value);wheel(3,1);await tick();assert(Number(nodes.zoom.value)<enlarged);
    wheel(1,2);await tick();assert.equal(reads,1);assert.equal(prevented,3);assert.equal(stopped,3);
    wheel(0);assert.equal(prevented,3);
    nodes.zoom.value='10';nodes.zoom.change();await tick();wheel(-120);await tick();assert.equal(Number(nodes.zoom.value),10);
    nodes.zoom.value='0.1';nodes.zoom.change();await tick();wheel(120);await tick();assert.equal(Number(nodes.zoom.value),0.1);
    nodes['zoom-reset'].click();await tick();assert.equal(Number(nodes.zoom.value),1);assert.equal(reads,1);
    for(const zoom of [0.1,10,1]){nodes['zoom-slider'].value=String(zoom);nodes['zoom-slider'].input();await tick();assert.equal(Number(nodes.zoom.value),zoom);assert.equal(reads,1);}
    nodes.zoom.value='2.5';nodes.zoom.change();await tick();assert.equal(Number(nodes['zoom-slider'].value),2.5);
    nodes.zoom.value='';nodes.zoom.change();assert.equal(Number(nodes.zoom.value),2.5);
    nodes.zoom.value='99';nodes.zoom.change();await tick();assert.equal(Number(nodes.zoom.value),10);
    nodes['zoom-reset'].click();await tick();assert.equal(Number(nodes.zoom.value),1);assert.equal(Number(nodes['zoom-slider'].value),1);assert.equal(reads,1);
    nodes.zoom.value='1';nodes.zoom.change();await tick();
    const checker=nodes['preview-background'].children[0],checkerURL=checker.src;
    const checkerPNG=decodedPNG(Buffer.from(checkerURL.split(',')[1],'base64'));assert.equal(checkerPNG.width,256);assert.equal(checkerPNG.height,256);
    const shade=(x,y)=>checkerPNG.raw[y*(256*4+1)+1+x*4];assert.equal(shade(0,0),shade(7,7));assert.notEqual(shade(0,0),shade(8,0));assert.equal(shade(0,0),shade(8,8));
    nodes.preview.clientWidth=641;nodes.preview.clientHeight=579;host.previewBackground();
    assert.equal(nodes['preview-background'].children.length,9);assert.equal(nodes['preview-background'].children[0],checker);
    for(const image of nodes['preview-background'].children){assert.equal(image.src,checkerURL);assert.equal(image.style.width,'256px');assert.equal(image.style.height,'256px');assert.equal(parseFloat(image.style.left)%256,0);assert.equal(parseFloat(image.style.top)%256,0);}
    nodes.preview.clientWidth=320;nodes.preview.clientHeight=220;host.previewBackground();assert.equal(nodes['preview-background'].children.length,2);
    nodes['show-grid'].checked=true;nodes['show-grid'].change();await tick();
    const gridImage=nodes.preview.children.find(c=>c.className==='preview-grid');assert(gridImage);assert.equal(gridImage.style.pointerEvents,'none');
    const overlay=decodedPNG(Buffer.from(gridImage.src.split(',')[1],'base64'));assert.equal(overlay.raw[4],0);assert(overlay.raw.some(v=>v===220));
    assert.equal(reads,1);assert.equal(nodes.preview.children[0],stableImage);
    nodes['show-grid'].checked=false;nodes['show-grid'].change();await tick();assert(!nodes.preview.children.some(c=>c.className==='preview-grid'));
    assert.match(nodes['preview-background'].children[0].src,/data:image\/png;base64/);
    nodes['background-mode'].selectedIndex=1;nodes['background-mode'].change();
    nodes['background-color'].value='#123456';nodes['background-color'].change();
    assert.equal(nodes['preview-background'].style.backgroundColor,'#123456');assert(nodes['preview-background'].children.every(c=>c.style.display==='none'));
    nodes['background-color'].value='bad';nodes['background-color'].change();assert.match(nodes['background-error'].textContent,/#RRGGBB/);assert.equal(nodes['preview-background'].style.backgroundColor,'#123456');
    nodes['background-mode'].selectedIndex=0;nodes['background-mode'].change();assert.match(nodes['preview-background'].children[0].src,/data:image\/png;base64/);
    assert.equal(reads,1);assert.equal(nodes.preview.children[0],stableImage);
    interval();await tick();assert.equal(nodes.preview.children[0],stableImage);assert.equal(reads,1);
    nodes.zoom.value='1.1';nodes.zoom.change();await tick();assert.equal(nodes.preview.children[0],stableImage);assert.equal(reads,1);
    nodes.zoom.value='1';nodes.zoom.change();await tick();
    for(const child of nodes.preview.children){assert(parseFloat(child.style.left)<=320);assert(parseFloat(child.style.top)<=220);}
    const left=nodes.preview.children.map(c=>c.style.left).join(',');
    nodes.preview.clientWidth=640;nodes.live.checked=false;interval();await tick();
    assert.equal(reads,1);assert.notEqual(nodes.preview.children.map(c=>c.style.left).join(','),left);
    hooks.panels.tilePreview.hide();nodes.refresh.click();await tick();assert.equal(reads,2);
    nodes.mode.selectedIndex=2;nodes.mode.change();await tick();assert.match(nodes.status.textContent,/绑定/);
    nodes.bind.click();await tick();assert.match(nodes.status.textContent,/1\/4/);
    const docB={id:2,width:128,height:128,title:'Second.psd',activeHistoryState:{id:1}};
    ps.app.documents.push(docB);ps.app.activeDocument=docB;
    nodes.slots.children[1].click();nodes.bind.click();await tick();assert.match(nodes.status.textContent,/2\/4/);
    assert.equal(new Set(nodes.preview.children.filter(c=>c.src).map(c=>c.src)).size,2);
    const offsetReads=reads,offsetBindings=JSON.stringify(host.activeBindings());
    for(const [mode,columns,rows]of [[1,1,2],[2,2,1],[3,1,2],[3,2,1]]){
        nodes['stagger-mode'].selectedIndex=mode;nodes['stagger-columns'].value=String(columns);nodes['stagger-rows'].value=String(rows);nodes['stagger-mode'].change();await tick();
        assert.match(nodes.status.textContent,/错位预览/);assert.equal(reads,offsetReads);assert.equal(JSON.stringify(host.activeBindings()),offsetBindings);
        assert.equal(nodes['stagger-columns'].disabled,mode===2);assert.equal(nodes['stagger-rows'].disabled,mode===1);
    }
    nodes['stagger-mode'].selectedIndex=0;nodes['stagger-mode'].change();await tick();assert.equal(nodes['stagger-settings'].style.display,'none');
    const previous=reads;docB.activeHistoryState.id++;nodes.live.checked=true;interval();await tick();
    assert.equal(reads,previous+1);assert.equal(readIds[readIds.length-1],2);
    ps.app.activeDocument=docA;nodes.bind.click();await tick();
    assert.equal(new Set(nodes.preview.children.filter(c=>c.src).map(c=>c.src)).size,1);
    ps.app.documents=[docB];interval();await tick();assert.match(nodes.status.textContent,/关闭.*重新绑定/);
    assert(nodes.preview.children.some(c=>c.className==='slot-marker'));
    // Bind without activating the document: dropdown selects among all open docs.
    ps.app.documents=[docA,docB];ps.app.activeDocument=docA;
    nodes['reload-documents'].click();await tick();nodes.slots.children[0].click();await tick();
    nodes['document-picker'].selectedIndex=2;nodes['document-picker'].change();await tick();
    assert.match(nodes['document-options'].children[2].textContent,/Second.psd/);
    assert.equal(ps.app.activeDocument,docA);
    for(const size of [2,3,4]){nodes['preset-'+size].click();await tick();assert.equal(nodes.slots.children.length,size*size);assert.equal(Number(nodes.columns.value),size);assert.equal(Number(nodes.rows.value),size);}
    nodes.columns.value='8';nodes.rows.value='8';nodes.columns.change();await tick();assert.equal(nodes.slots.children.length,64);
    nodes.slots.children[63].click();await tick();nodes['document-picker'].selectedIndex=2;nodes['document-picker'].change();await tick();
    assert.match(nodes['selected-slot'].textContent,/行 8，列 8/);
    const beforeRepeat=reads;
    // Shrinking and expanding keeps the hidden (8,8) binding.
    nodes.columns.value='3';nodes.rows.value='5';nodes.columns.change();await tick();assert.equal(nodes.slots.children.length,15);
    nodes.columns.value='8';nodes.rows.value='8';nodes.columns.change();await tick();assert.match(nodes.slots.children[63].title,/Second.psd/);
    assert.equal(reads,beforeRepeat);
    const marker=nodes.preview.children.find(c=>c.className==='slot-marker');assert(marker);marker.click();await tick();
    nodes['show-slots'].checked=false;nodes['show-slots'].change();await tick();assert(!nodes.preview.children.some(c=>c.className==='slot-marker'));
    // Exercise 64 distinct documents, not just 64 aliases of two documents.
    ps.app.documents=Array.from({length:64},(_,i)=>({id:i+10,width:512,height:512,title:`Tile-${i}.png`,activeHistoryState:{id:1}}));
    ps.app.activeDocument=ps.app.documents[0];nodes['reload-documents'].click();await tick();
    for(let i=0;i<64;i++){nodes.slots.children[i].click();nodes['document-picker'].selectedIndex=i+1;nodes['document-picker'].change();}
    await tick();assert.match(nodes.status.textContent,/64\/64.*64 个文档.*256px/);
    const last64=readOptions.slice(-64);assert.equal(new Set(last64.map(o=>o.documentID)).size,64);
    assert(last64.every(o=>o.targetSize.width===256&&o.targetSize.height===256));
    assert.equal(reads,disposals);
    // Create a single-document 3x3 atlas, with a top locked Multiply helper.
    let nextDocID=100;
    ps.app.documents.add=async options=>{
        assert.equal(options.fill,'transparent');
        const id=nextDocID;nextDocID+=2;let nextLayer=id*100+2;
        const doc={...options,id,width:options.width,height:options.height,title:options.name,activeHistoryState:{id:1},layers:[{id:id*100+1,name:'Layer 1',visible:true,allLocked:false}],async createPixelLayer(options){const layer={...options,id:nextLayer++,visible:true,allLocked:false,move(reference,placement){doc.layers.splice(doc.layers.indexOf(this),1);doc.layers.splice(doc.layers.indexOf(reference)+(placement==='placeAfter'?1:0),0,this);}};this.layers.unshift(layer);return layer;}};
        ps.app.documents.push(doc);ps.app.activeDocument=doc;return doc;
    };
    nodes.mode.selectedIndex=1;nodes.columns.value='3';nodes.rows.value='3';nodes.mode.change();await tick();
    await nodes['new-grid-document'].click();await tick();
    rollbackChangesHistoryID=true;nodes.refresh.click();await tick();
    const afterRollbackReads=reads;for(let n=0;n<5;n++){interval();await tick();}assert.equal(reads,afterRollbackReads,'helper history rollback must not trigger an endless capture loop');
    const slotButton=nodes.slots.children[1];slotButton.click();await tick();assert.equal(nodes.slots.children[1],slotButton);assert.equal(slotButton.style.backgroundColor,'#315e8c');
    assert.equal(slotButton.style.border,'2px solid #ddd');assert.equal(slotButton.style.borderRadius,'13px');
    nodes.slots.children[0].click();await tick();assert.equal(slotButton.style.backgroundColor,'#292929');assert.equal(slotButton.style.border,'2px solid #ddd');assert.equal(slotButton.style.borderRadius,'13px');
    const atlasReads=reads;nodes['show-slots'].checked=false;nodes['show-grid'].checked=true;nodes['show-grid'].change();await tick();
    assert(nodes.preview.children.some(c=>c.className==='preview-grid'));assert(!nodes.preview.children.some(c=>c.className==='slot-marker'));assert.equal(reads,atlasReads);
    nodes['show-grid'].checked=false;nodes['show-grid'].change();await tick();
    const atlas=ps.app.activeDocument,guide=atlas.layers[0];
    assert.equal(atlas.width,384);assert.equal(atlas.height,192);assert.equal(guide.blendMode,'multiply');assert.equal(guide.allLocked,true);assert.equal(guide.visible,true);assert.equal(atlas.selectedLayerID,10001);assert.equal(guideDisposals,1);
    // Every logical cell center maps back to its original (column,row).
    const spec=host.atlasSpec(atlas);
    for(let r=0;r<3;r++)for(let c=0;c<3;c++){const p=host.atlasPoint(c+.5,r+.5,spec),uv=host.atlasUV(p.x/atlas.width,p.y/atlas.height,3,3);assert(Math.abs(uv.u-c-.5)<1e-9);assert(Math.abs(uv.v-r-.5)<1e-9);}
    await nodes['export-grid'].click();await tick();assert.match(nodes['operation-status'].textContent,/已导出 9 张/);
    const nine=exportFolders[exportFolders.length-1];assert.equal(nine.files.size,9);
    assert(nine.files.has('tile_r03_c03.png'));
    const exportedMask=decodedPNG(nine.files.get('tile_r01_c01.png'));
    for(const [x,y]of [[0,31],[127,31],[62,0],[65,0],[62,63],[65,63]])assert.equal(exportedMask.raw[y*(128*4+1)+1+x*4+3],128,'export must preserve guide-covered tips');
    for(const bytes of nine.files.values()){const decoded=decodedPNG(bytes);assert.equal(decoded.width,128);assert.equal(decoded.height,64);assert.equal(decoded.raw[4],0);assert.equal(decoded.raw[32*(128*4+1)+1+64*4+3],128);}
    assert.equal(guide.visible,true);assert.equal(guide.allLocked,true);
    // Existing 256x128 artwork is a 2x2 lattice of 128x64 diamond cells.
    const existing={id:101,title:'existing.png',width:256,height:128,activeHistoryState:{id:1},layers:[]};ps.app.documents.push(existing);ps.app.activeDocument=existing;
    nodes.columns.value='2';nodes.rows.value='2';nodes.columns.change();await tick();
    await nodes['export-grid'].click();await tick();assert.match(nodes['operation-status'].textContent,/已导出 4 张/);assert.equal(exportFolders[exportFolders.length-1].files.size,4);
    const rawReads=readOptions.filter(o=>o.documentID===101&&!o.targetSize);assert.equal(rawReads.length,4);assert(rawReads.every(o=>o.sourceBounds.right-o.sourceBounds.left===128&&o.sourceBounds.bottom-o.sourceBounds.top===64));
    // A capture failure still rolls visibility and lock state back.
    ps.app.activeDocument=atlas;nodes.columns.value='3';nodes.rows.value='3';nodes.columns.change();await tick();failRead=true;
    await nodes['export-grid'].click();await tick();assert.match(nodes['operation-status'].textContent,/0\/9/);assert.equal(guide.visible,true);assert.equal(guide.allLocked,true);failRead=false;
    // Convert a custom 3x2 Slot arrangement: duplicates, blanks and a source guide.
    rollbackChangesHistoryID=false;const addDocument=ps.app.documents.add;ps.app.documents=[docA,docB,atlas];ps.app.documents.add=addDocument;ps.app.activeDocument=docA;
    nodes.mode.selectedIndex=2;nodes.columns.value='3';nodes.rows.value='2';nodes.elevated.checked=false;nodes.mode.change();await tick();
    for(let i=0;i<6;i++){nodes.slots.children[i].click();nodes.unbind.click();await tick();}
    for(const [cell,doc]of [[0,docA],[1,docB],[3,docA],[5,atlas]]){nodes.slots.children[cell].click();ps.app.activeDocument=doc;nodes.bind.click();await tick();}
    const beforeConvert=readIds.length;await nodes['convert-single-document'].click();await tick();
    assert.match(nodes['operation-status'].textContent,/已转换为单文档/);assert.equal(nodes.mode.selectedIndex,1);assert.equal(nodes['atlas-actions'].style.display,'block');assert.equal(nodes['multi-document-controls'].style.display,'none');
    const converted=ps.app.activeDocument;assert.equal(converted.width,320);assert.equal(converted.height,160);assert.equal(converted.layers.length,7);
    assert.match(converted.layers[0].name,/辅助网格/);assert.equal(converted.layers[0].allLocked,true);assert.equal(converted.layers[0].blendMode,'multiply');
    assert.deepEqual(converted.layers.slice(1).map(l=>l.name.match(/Slot (\d+,\d+)/)[1]),['1,1','1,2','1,3','2,1','2,2','2,3']);
    assert.equal(converted.selectedLayerID,converted.layers[1].id);
    assert.deepEqual(JSON.parse(JSON.stringify(converted.layers[1].targetBounds)),{left:64,top:0});assert.deepEqual(JSON.parse(JSON.stringify(converted.layers[2].targetBounds)),{left:128,top:32});assert.deepEqual(JSON.parse(JSON.stringify(converted.layers[4].targetBounds)),{left:0,top:32});
    assert.equal(converted.layers[1].gridPixels[3],0);assert.equal(converted.layers[1].gridPixels[(32*128+64)*4],255);assert.equal(converted.layers[2].gridPixels[(32*128+64)*4],128);
    const conversionMask=host.diamondMask(128,64);for(let y=0;y<64;y++)for(let x=0;x<128;x++){const offset=(y*128+x)*4;for(let channel=0;channel<4;channel++)assert.equal(converted.layers[1].gridPixels[offset+channel],conversionMask[y*128+x]?255:0);}
    for(const [x,y]of [[0,31],[127,31],[62,0],[65,0],[62,63],[65,63]]){
        const offset=(y*128+x)*4;assert.equal(converted.layers[1].gridPixels[offset+3],255,'conversion must preserve each tip');
    }
    assert.deepEqual(converted.layers[1].gridPixels,converted.layers[4].gridPixels);assert.equal(converted.layers[3].gridPixels,undefined);assert.equal(converted.layers[5].gridPixels,undefined);
    const convertedReads=readIds.slice(beforeConvert);for(const source of [docA,docB,atlas])assert.equal(convertedReads.filter(id=>id===source.id).length,1,'read each unique source once');
    assert.equal(guide.visible,true);assert.equal(guide.allLocked,true);assert(ps.app.documents.includes(docA));assert(ps.app.documents.includes(docB));
    // Closed bindings fail before creating a destination and keep multi-document mode.
    nodes.mode.selectedIndex=2;nodes.mode.change();await tick();ps.app.documents=ps.app.documents.filter(d=>d!==docB);const beforeFailure=ps.app.documents.length;
    await nodes['convert-single-document'].click();await tick();assert.match(nodes['operation-status'].textContent,/已关闭/);assert.equal(ps.app.documents.length,beforeFailure);assert.equal(nodes.mode.selectedIndex,2);
    ps.app.documents.push(docB);ps.app.documents.add=addDocument;
    failRead=true;await nodes['convert-single-document'].click();await tick();assert.equal(ps.app.documents.length,beforeFailure+1);assert.match(nodes['operation-status'].textContent,/未创建目标文档/);assert.equal(guide.visible,true);assert.equal(guide.allLocked,true);failRead=false;
    const putPixels=ps.imaging.putPixels;ps.imaging.putPixels=async options=>{if(options.targetBounds)throw new Error('Simulated destination write failure');return putPixels(options);};
    const beforePartial=ps.app.documents.length;await nodes['convert-single-document'].click();await tick();assert.equal(ps.app.documents.length,beforePartial+1);assert.match(nodes['operation-status'].textContent,/保留未完成的新文档/);assert.equal(nodes.mode.selectedIndex,2);assert(ps.app.documents.includes(docA));assert(ps.app.documents.includes(docB));ps.imaging.putPixels=putPixels;
    console.log('Conversion checks passed: 3x2 ordered layers, duplicate/blank Slots, original-size reads, top locked Multiply grid, mode refresh, closed/read/write failures and source preservation.');
    console.log('Mock-host checks passed: startup/resize, document dropdown, 2x2/3x3/4x4 presets and manual 8x8/3x5, hidden binding retention, clickable markers, deduplicated cache, 64 distinct documents at 256px and buffer disposal.');
    console.log('Atlas checks passed: locked Multiply guide, 3x3 creates/exports 9 transparent PNGs, existing 256x128 splits to four 128x64 cells, cell-coordinate mapping and helper restoration after failure.');
})().catch(e=>{console.error(e);process.exitCode=1;});
