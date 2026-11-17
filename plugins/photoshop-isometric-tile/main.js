const ps=require("photoshop");
const {entrypoints}=require("uxp");
const {png,base64}=require("./png.js");
const $=id=>document.getElementById(id);
$("auto-wrap").checked=false;
$("version").textContent=`版本 ${require("./manifest.json").version}`;
let timer=null,busy=false,visible=false,generation=0,lastKey="",lastViewport="",pending=false;
let selected=0,bindings=Array(64).fill(null),documentListKey="",documentOptions=[];
let operationBusy=false,slotLayout="";
let selectedMode=1,controlsKey="";
let autoWrapPending=null,paintListenerRegistered=false;
let wrapObserved=null,autoWrapDocument=null,wrapTimer=null,autoWrapRuns=0;
const cache=new Map();
const diamondMasks=new Map();
let backgroundKey="",checkerURL="";
function setStatus(id,text,error=false){
    const node=$(id);node.textContent=text;node.style.color=error?"#ff6e6e":"#ddd";
}
function previewBackground(){
    const panel=$("preview-background"),custom=$("background-mode").selectedIndex===1;
    $("background-color-row").style.display=custom?"flex":"none";
    const color=String($("background-color").value||"").trim();
    if(custom&&!/^#[0-9a-f]{6}$/i.test(color)){setStatus("background-error","请输入 #RRGGBB，例如 #303030",true);return;}
    setStatus("background-error","");
    if(custom){
        panel.style.backgroundColor=color;
        for(const child of panel.children)child.style.display="none";
        $("background-swatch").style.backgroundColor=color;backgroundKey="";return;
    }
    const view=viewport(),key=JSON.stringify(view);if(key===backgroundKey)return;
    // Fixed-size image tiles avoid unsupported CSS repeat and scaled-raster moire.
    const side=256;
    if(!checkerURL){
        const rgba=new Uint8Array(side*side*4);
        for(let y=0;y<side;y++)for(let x=0;x<side;x++){
            const shade=(Math.floor(x/8)+Math.floor(y/8))%2?166:204;
            rgba.set([shade,shade,shade,255],(y*side+x)*4);
        }
        checkerURL="data:image/png;base64,"+base64(png(side,side,rgba));
    }
    const columns=Math.ceil(view.width/side),rows=Math.ceil(view.height/side);
    let count=0;
    for(let row=0;row<rows;row++)for(let col=0;col<columns;col++){
        let image=panel.children[count++];
        if(!image){image=document.createElement("img");image.src=checkerURL;panel.appendChild(image);}
        Object.assign(image.style,{position:"absolute",left:col*side+"px",top:row*side+"px",width:side+"px",height:side+"px",display:"block"});
    }
    while(panel.children.length>count)panel.removeChild(panel.children[panel.children.length-1]);
    panel.style.backgroundColor="#cccccc";backgroundKey=key;
}
async function pickBackgroundColor(){
    if(operationBusy||busy){setStatus("background-error","正在读取画面，请稍后再选色。");return;}
    const original=String($("background-color").value||"").trim();
    const hex=/^#[0-9a-f]{6}$/i.test(original)?original:"#555555";
    operationBusy=true;
    try{
        const response=await ps.core.executeAsModal(async()=>{
            const [result]=await ps.action.batchPlay([{_obj:"showColorPicker",_target:[{_ref:"application"}],context:"选择预览背景色",color:{_obj:"RGBColor",red:parseInt(hex.slice(1,3),16),grain:parseInt(hex.slice(3,5),16),blue:parseInt(hex.slice(5,7),16)},_options:{dialogOptions:"display"}}],{});
            return result;
        },{commandName:"选择预览背景色"});
        if(!response||response.value===false||response.result===-128)return;
        if(response._obj==="error")throw new Error(response.message||"无法打开Photoshop调色板");
        const color=response.RGBFloatColor||response.color;
        const channels=color&&[color.red,color.grain===undefined?color.green:color.grain,color.blue];
        if(!channels||!channels.every(Number.isFinite))throw new Error("调色板未返回RGB颜色，请使用HEX输入。");
        $("background-color").value="#"+channels.map(v=>Math.round(Math.max(0,Math.min(255,v))).toString(16).padStart(2,"0")).join("").toUpperCase();
        previewBackground();
    }catch(error){if(error.number!==-128&&error.result!==-128)setStatus("background-error",error.message||String(error),true);}
    finally{operationBusy=false;}
}
function mode(){const index=$("mode").selectedIndex;if(Number.isInteger(index)&&index>=0&&index<3)selectedMode=index;return selectedMode;}
function reconcileControls(){
    const key=JSON.stringify([mode(),$("columns").value,$("rows").value,$("width").value,$("height").value,selected,bindings]);
    if(key!==controlsKey){slotUI();controlsKey=key;}
}
function number(id,min,max){const v=Number($(id).value);if(!Number.isFinite(v)||v<min||v>max)throw new Error(`${id}: ${min}–${max}`);return v;}
function viewport(){const panel=$("preview");return {width:Math.max(1,panel.clientWidth||panel.getBoundingClientRect().width||300),height:Math.max(1,panel.clientHeight||panel.getBoundingClientRect().height||200)};}
function documents(){const result=[];for(let i=0;i<ps.app.documents.length;i++)result.push(ps.app.documents[i]);return result;}
function atlasUV(x,y,columns,rows){const k=columns+rows;return {u:(k*(x+y)-rows)/2,v:(k*(y-x)+rows)/2};}
function atlasPoint(u,v,spec){return {x:(spec.rows+u-v)*spec.tw/2,y:(u+v)*spec.th/2};}
function gridLayers(doc){
    const found=[];
    function walk(layers){if(!layers)return;for(let i=0;i<layers.length;i++){
        const layer=layers[i],m=/^辅助网格 \[UYTile\] (\d+)x(\d+) (\d+)x(\d+)$/.exec(layer.name||"");
        if(m)found.push({layer,columns:Number(m[1]),rows:Number(m[2]),tw:Number(m[3]),th:Number(m[4])});
        if(ps.constants&&layer.kind===ps.constants.LayerKind.GROUP)walk(layer.layers);
    }}walk(doc.layers);return found;
}
function atlasSpec(doc){
    const {columns,rows}=dimensions(),k=columns+rows,tw=2*Number(doc.width)/k,th=2*Number(doc.height)/k;
    const desiredW=number("width",2,8192),desiredH=number("height",2,8192);
    if(tw!==desiredW||th!==desiredH)throw new Error(`当前文档 ${doc.width}×${doc.height} 与网格不匹配；${columns}×${rows} 格、单格 ${desiredW}×${desiredH} 需要画布 ${k*desiredW/2}×${k*desiredH/2}。请调整设置或新建文档`);
    const guides=gridLayers(doc);
    if(guides.length>1)throw new Error("文档有多个辅助网格，请先保留一个再预览或导出");
    if(guides.length&&(guides[0].columns!==columns||guides[0].rows!==rows||guides[0].tw!==tw||guides[0].th!==th))throw new Error(`辅助网格与面板设置不同，请改回 ${guides[0].columns}×${guides[0].rows}，或按新设置新建文档`);
    return {columns,rows,tw,th};
}
async function withoutGrid(doc,read,commandName="读取 Tile 画面"){
    return ps.core.executeAsModal(context=>readWithoutGrid(context,doc,read,commandName),{commandName});
}
async function readWithoutGrid(context,doc,read,commandName){
    const observed=wrapObserved&&wrapObserved.documentID===doc.id&&wrapObserved.historyID===doc.activeHistoryState.id;
    const guides=gridLayers(doc);
        // PS 2024 also requires modal scope for composite pixel capture without guides.
        if(!guides.length)return await read(context);
        const token=await context.hostControl.suspendHistory({documentID:doc.id,name:commandName});
        const states=guides.map(g=>({layer:g.layer,visible:g.layer.visible,locked:g.layer.allLocked}));
        try{
            for(const s of states){s.layer.allLocked=false;s.layer.visible=false;}
            return await read(context);
        }finally{
            // Roll back the temporary visibility/lock edits, including on cancellation.
            // This keeps helper lines out of images and avoids adding capture history.
            await context.hostControl.resumeHistory(token,false);
            if(observed&&!autoWrapPending)observeWrapBaseline();
        }
}
function gridRaster(width,height,spec,overlay=false){
    const rgba=new Uint8Array(width*height*4);if(!overlay)rgba.fill(255);
    function line(a,b){
        const dx=b.x-a.x,dy=b.y-a.y;
        const put=(x,y)=>{if(x>=0&&x<width&&y>=0&&y<height)rgba.set(overlay?[255,216,70,220]:[70,110,150,255],(y*width+x)*4);};
        // Geometry describes pixel edges; choose samples at pixel centers, like capture/export.
        // Do not clamp out-of-canvas endpoints into the last row: that deforms lower tips.
        if(Math.abs(dx)>=Math.abs(dy)&&dx!==0){
            for(let x=Math.max(0,Math.floor(Math.min(a.x,b.x)));x<Math.min(width,Math.ceil(Math.max(a.x,b.x)));x++){
                const t=(x+.5-a.x)/dx;if(t>=0&&t<=1)put(x,Math.round(a.y+t*dy-.5));
            }
        }else if(dy!==0){
            for(let y=Math.max(0,Math.floor(Math.min(a.y,b.y)));y<Math.min(height,Math.ceil(Math.max(a.y,b.y)));y++){
                const t=(y+.5-a.y)/dy;if(t>=0&&t<=1)put(Math.round(a.x+t*dx-.5),y);
            }
        }
    }
    for(let c=0;c<=spec.columns;c++)line(atlasPoint(c,0,spec),atlasPoint(c,spec.rows,spec));
    for(let r=0;r<=spec.rows;r++)line(atlasPoint(0,r,spec),atlasPoint(spec.columns,r,spec));
    return rgba;
}
function diamondMask(width,height,columns=1,rows=1){
    const key=JSON.stringify([width,height,columns,rows]);
    if(diamondMasks.has(key))return diamondMasks.get(key);
    const k=columns+rows,lines=gridRaster(width,height,{columns,rows,tw:2*width/k,th:2*height/k},true),mask=new Uint8Array(width*height);
    for(let y=0;y<height;y++)for(let x=0;x<width;x++){
        const index=y*width+x,uv=atlasUV((x+.5)/width,(y+.5)/height,columns,rows);
        // Retain the guide-covered boundary pixels, but never bake the guide color in.
        mask[index]=lines[index*4+3]>0||(uv.u>=0&&uv.u<columns&&uv.v>=0&&uv.v<rows)?1:0;
    }
    if(diamondMasks.size>=4)diamondMasks.delete(diamondMasks.keys().next().value);
    diamondMasks.set(key,mask);return mask;
}
function wrappedPixel(x,y,spec){
    const u=x/spec.tw+y/spec.th-spec.rows/2,v=y/spec.th-x/spec.tw+spec.rows/2;
    const cu=Math.floor(u/spec.columns),cv=Math.floor(v/spec.rows);
    return {x:x-cu*spec.columns*spec.tw/2+cv*spec.rows*spec.tw/2,y:y-cu*spec.columns*spec.th/2-cv*spec.rows*spec.th/2};
}
function insideWrapUnit(x,y,spec){
    const u=(x+.5)/spec.tw+(y+.5)/spec.th-spec.rows/2,v=(y+.5)/spec.th-(x+.5)/spec.tw+spec.rows/2;
    return u>=0&&u<spec.columns&&v>=0&&v<spec.rows;
}
function wrapPixels(rgba,width,height,left,top,spec,before=null){
    const canvasWidth=(spec.columns+spec.rows)*spec.tw/2,canvasHeight=(spec.columns+spec.rows)*spec.th/2,mask=diamondMask(canvasWidth,canvasHeight,spec.columns,spec.rows);
    const keep=(x,y)=>x>=0&&x<canvasWidth&&y>=0&&y<canvasHeight&&mask[y*canvasWidth+x];
    const out=new Uint8Array(rgba),translations=new Map();let moved=0,copied=0;
    for(let y=0;y<height;y++)for(let x=0;x<width;x++){
        const from=(y*width+x)*4,gx=x+left,gy=y+top;
        if(!rgba[from+3]||keep(gx,gy))continue;
        const target=wrappedPixel(gx+.5,gy+.5,spec),tx=Math.floor(target.x)-left,ty=Math.floor(target.y)-top;
        if(tx<0||tx>=width||ty<0||ty>=height)throw new Error("回落目标超出读取范围，未写入图层");
        const shift=`${Math.floor(target.x)-gx},${Math.floor(target.y)-gy}`;translations.set(shift,(translations.get(shift)||0)+1);
        out.fill(0,from,from+4);moved++;
    }
    // The inclusive guide fringe and its periodic interior pixel both exist.
    // Copy the changed edge to its partner without clearing it or compounding alpha.
    const different=(a,ai,b,bi)=>a[ai]!==b[bi]||a[ai+1]!==b[bi+1]||a[ai+2]!==b[bi+2]||a[ai+3]!==b[bi+3];
    for(let y=0;y<height;y++)for(let x=0;x<width;x++){
        const gx=x+left,gy=y+top;
        if(!keep(gx,gy)||insideWrapUnit(gx,gy,spec))continue;
        const edge=(y*width+x)*4,target=wrappedPixel(gx+.5,gy+.5,spec),tx=Math.floor(target.x)-left,ty=Math.floor(target.y)-top,interior=(ty*width+tx)*4;
        let from=edge,to=interior;
        if(before){
            if(different(rgba,edge,before,edge)&&rgba[edge+3]){}
            else if(different(rgba,interior,before,interior)&&rgba[interior+3]){from=interior;to=edge;}
            else continue;
        }
        if(!rgba[from+3]||!different(out,to,rgba,from))continue;
        out.set(rgba.subarray(from,from+4),to);copied++;
    }
    if(!moved&&!copied)return {rgba:out,moved,copied,translations:[]};
    // Read only the immutable source; multiple outside samples composite in row-major order.
    for(let y=0;y<height;y++)for(let x=0;x<width;x++){
        const from=(y*width+x)*4,gx=x+left,gy=y+top;
        if(!rgba[from+3]||keep(gx,gy))continue;
        const target=wrappedPixel(gx+.5,gy+.5,spec),to=((Math.floor(target.y)-top)*width+Math.floor(target.x)-left)*4;
        const sa=rgba[from+3]/255,da=out[to+3]/255,alpha=sa+da*(1-sa);
        for(let c=0;c<3;c++)out[to+c]=Math.round((rgba[from+c]*sa+out[to+c]*da*(1-sa))/alpha);
        out[to+3]=Math.round(alpha*255);
    }
    // Final canonical edge colors include moved/alpha-composited outside samples.
    // Propagate those final colors to every retained guide fringe, without alpha-over again.
    for(let y=0;y<height;y++)for(let x=0;x<width;x++){
        const gx=x+left,gy=y+top;
        if(!keep(gx,gy)||insideWrapUnit(gx,gy,spec))continue;
        const edge=(y*width+x)*4,target=wrappedPixel(gx+.5,gy+.5,spec),tx=Math.floor(target.x)-left,ty=Math.floor(target.y)-top;
        if(tx<0||tx>=width||ty<0||ty>=height)throw new Error("边界同步目标超出读取范围，未写入图层");
        const interior=(ty*width+tx)*4;
        if(out[interior+3]&&different(out,interior,rgba,interior)&&different(out,edge,out,interior)){
            out.set(out.subarray(interior,interior+4),edge);copied++;
        }
    }
    return {rgba:out,moved,copied,translations:Array.from(translations,([offset,count])=>({offset,count}))};
}
function unpackLayerPixels(result,raw,bounds){
    const data=result.imageData,width=bounds.right-bounds.left,height=bounds.bottom-bounds.top,rgba=new Uint8Array(width*height*4);
    if(!data)return rgba;
    if(result.level)throw new Error("PS返回缩小像素，已停止回落，避免改动原始分辨率");
    if(data.components!==3&&data.components!==4)throw new Error("当前层像素格式不支持回落");
    const b=result.sourceBounds,bw=b.right-b.left,bh=b.bottom-b.top;
    if(bw>0&&bh>0)for(let y=0;y<height;y++)for(let x=0;x<width;x++){
        const sx=Math.floor((bounds.left+x+.5-b.left)*data.width/bw),sy=Math.floor((bounds.top+y+.5-b.top)*data.height/bh);
        if(sx<0||sy<0||sx>=data.width||sy>=data.height)continue;
        const from=(sy*data.width+sx)*data.components,to=(y*width+x)*4;
        rgba[to]=raw[from];rgba[to+1]=raw[from+1];rgba[to+2]=raw[from+2];rgba[to+3]=data.components===4?raw[from+3]:255;
    }return rgba;
}
async function previousLayerPixels(doc,layer,bounds,colorProfile,beforeHistoryID){
    const states=doc.historyStates;if(!states)return null;
    let index=-1;for(let i=0;i<states.length;i++)if(states[i].id===doc.activeHistoryState.id){index=i;break;}
    const historyID=beforeHistoryID!==undefined&&Array.from(states).some(s=>s.id===beforeHistoryID)?beforeHistoryID:index>0?states[index-1].id:undefined;
    if(historyID===undefined)return null;
    let data;
    try{
        const result=await ps.imaging.getPixels({documentID:doc.id,layerID:layer.id,historyStateID:historyID,sourceBounds:bounds,componentSize:8,colorSpace:"RGB",colorProfile,applyAlpha:false});
        data=result.imageData;return unpackLayerPixels(result,data?await data.getData({chunky:true}):null,bounds);
    }catch(error){return null;}finally{if(data)data.dispose();}
}
function wrapTarget(expected){
    if(mode()!==1)throw new Error("回落仅用于单文档菱形网格模式");
    if(!ps.app.documents.length)throw new Error("请先打开单文档网格");
    const doc=ps.app.activeDocument,layers=doc.activeLayers;
    if(!layers||layers.length!==1)throw new Error("请仅选中一个普通像素图层");
    const layer=layers[0];
    if(expected&&(!autoWrapDocument||doc.id!==autoWrapDocument.id))return null;
    if(expected&&(doc.id!==expected.documentID||layer.id!==expected.layerID||doc.activeHistoryState.id!==expected.historyID))return null;
    if(layer.kind!==ps.constants.LayerKind.NORMAL||layer.isBackgroundLayer||/^辅助网格 \[UYTile\]/.test(layer.name||""))throw new Error("请选择绘画用的普通像素层，不能回落辅助网格／组／智能对象／背景层");
    if(layer.allLocked||layer.pixelsLocked||layer.transparentPixelsLocked)throw new Error("当前图层像素或透明度已锁定，请先解锁");
    if(layer.visible===false||layer.opacity!==100||layer.fillOpacity!==100)throw new Error("回落暂要求图层可见，不透明度与填充均为100%");
    if(layer.blendMode!==undefined&&layer.blendMode!==ps.constants.BlendMode.NORMAL)throw new Error("回落暂要求图层混合模式为正常");
    if(doc.mode!==ps.constants.DocumentMode.RGB||doc.bitsPerChannel!==ps.constants.BitsPerChannelType.EIGHT)throw new Error("回落暂支持RGB 8位文档，不会自动降低其他文档位深");
    const spec=atlasSpec(doc);
    if(spec.tw%2||spec.th%2||spec.tw>1024||spec.th>1024)throw new Error("回落单格宽高须为2–1024的偶数");
    return {doc,layer,spec};
}
async function wrapCurrentLayer(expected=null){
    const automatic=Boolean(expected),label=automatic?"自动回落":"手动回落";
    setStatus("wrap-status",`${label}：正在核对并读取当前图层…`);
    let moved=0,copied=0,translations=[],usedHistory=false;
    await ps.core.executeAsModal(async context=>{
        const target=wrapTarget(expected);if(!target){setStatus("wrap-status","自动回落已跳过：文档、图层或历史状态在等待期间发生变化。");return;}
        const {doc,layer,spec}=target;
        const descriptions=await ps.action.batchPlay([{_obj:"get",_target:[{_ref:"layer",_id:layer.id},{_ref:"document",_id:doc.id}],_options:{dialogOptions:"dontDisplay"}}],{}),descriptor=descriptions[0];
        if(!descriptor||descriptor._obj==="error")throw new Error("无法核对图层属性，未执行回落");
        if(descriptor.hasUserMask||descriptor.hasVectorMask||descriptor.hasFilterMask||descriptor.layerEffects)throw new Error("回落暂不处理带蒙版或图层效果的图层，请使用独立普通绘画层");
        const bounds=layer.boundsNoEffects,left=Math.min(0,Math.floor(Number(bounds.left))),top=Math.min(0,Math.floor(Number(bounds.top))),right=Math.max(Number(doc.width),Math.ceil(Number(bounds.right))),bottom=Math.max(Number(doc.height),Math.ceil(Number(bounds.bottom)));
        const width=right-left,height=bottom-top;
        if(!Number.isFinite(width*height)||width*height>16*1024*1024)throw new Error("回落读取范围超过16777216像素，请缩小图层范围");
        let data,imageData,history;
        try{
            const result=await ps.imaging.getPixels({documentID:doc.id,layerID:layer.id,sourceBounds:{left,top,right,bottom},componentSize:8,colorSpace:"RGB",applyAlpha:false});
            data=result.imageData;if(!data){setStatus("operation-status","当前层没有可回落的像素");setStatus("wrap-status",`${label}：图层为空，无需处理。`);return;}
            const readBounds={left,top,right,bottom},rgba=unpackLayerPixels(result,await data.getData({chunky:true}),readBounds);
            const before=await previousLayerPixels(doc,layer,readBounds,data.colorProfile||doc.colorProfileName,expected&&expected.beforeHistoryID);usedHistory=Boolean(before);
            const wrapped=wrapPixels(rgba,width,height,left,top,spec,before);moved=wrapped.moved;copied=wrapped.copied;translations=wrapped.translations;
            if(!moved&&!copied){setStatus("operation-status","没有待搬移像素或边界差异");setStatus("wrap-status",`${label}已检查：没有越界像素或待同步边界。`);return;}
            if(context.isCancelled)throw new Error("用户取消回落");
            imageData=await ps.imaging.createImageDataFromBuffer(wrapped.rgba,{width,height,components:4,colorSpace:"RGB",colorProfile:data.colorProfile||doc.colorProfileName});
            history=await context.hostControl.suspendHistory({documentID:doc.id,name:"菱形回落（当前图层）"});
            await ps.imaging.putPixels({documentID:doc.id,layerID:layer.id,imageData,replace:true,targetBounds:{left,top}});
            if(context.isCancelled)throw new Error("用户取消回落");
            await context.hostControl.resumeHistory(history,true);history=null;
        }finally{
            try{if(history)await context.hostControl.resumeHistory(history,false);}
            finally{if(imageData)imageData.dispose();if(data)data.dispose();}
        }
    },{commandName:"菱形回落（当前图层）"});
    if(moved||copied){cache.clear();lastKey="";setStatus("operation-status",`已搬移 ${moved} 个越界像素，复制 ${copied} 个边界像素；仅修改当前层，可在PS撤销。`);setStatus("wrap-status",`${label}完成：搬移${moved}／边界复制${copied}，已请求刷新。${copied&&!usedHistory?"无前一历史像素，边界按外缘向内同步。":""}${translations.length?"位移（x,y）："+translations.slice(0,4).map(t=>`(${t.offset}) ${t.count}像素`).join("；"):""}`);}return moved+copied;
}
function wrapObservation(){
    const doc=ps.app.activeDocument;
    if(!doc||!doc.activeLayers||doc.activeLayers.length!==1)return null;
    if(autoWrapDocument&&doc.id!==autoWrapDocument.id)return null;
    const history=doc.activeHistoryState;
    if(!history)return null;
    return {documentID:doc.id,layerID:doc.activeLayers[0].id,historyID:history.id,name:String(history.name||""),historyIDs:Array.from(doc.historyStates||[],h=>h.id)};
}
function wrapDocumentPath(doc){
    try{
        const path=String(doc.path||"").trim().replace(/\\/g,"/");
        if(/^[a-z]:\//i.test(path)||path.startsWith("//"))return path.toLowerCase();
        return path.startsWith("/")||path.startsWith("file:")?path:"";
    }catch(error){return "";}
}
function updateWrapBinding(){
    const label=$("auto-wrap-document");
    if(!autoWrapDocument){label.textContent="自动回落对象：未绑定";return false;}
    const docs=documents(),doc=docs.length?ps.app.activeDocument:null;
    let bound=docs.find(d=>d.id===autoWrapDocument.id);
    if(!bound){
        autoWrapPending=null;wrapObserved=null;
        const matches=autoWrapDocument.path?docs.filter(d=>wrapDocumentPath(d)===autoWrapDocument.path):[];
        if(matches.length===1){
            const reopened=matches[0];
            try{atlasSpec(reopened);}catch(error){setStatus("wrap-status",`同一文件已重开，但网格校验失败：${error.message||error}`,true);return false;}
            cache.delete(autoWrapDocument.id);lastKey="";
            autoWrapDocument={id:reopened.id,title:reopened.title||String(reopened.id),path:autoWrapDocument.path};
            bound=reopened;
            if(doc&&doc.id===reopened.id)wrapObserved=wrapObservation();
            setStatus("wrap-status","已恢复同一文件的自动回落绑定，历史基线已重建，等待新笔触。");
        }else{
            label.textContent=`自动回落对象：${autoWrapDocument.title} [${autoWrapDocument.id}]${matches.length>1?"（同一路径有多个文档，请手动绑定）":autoWrapDocument.path?"（已关闭，等待同一文件重开）":"（已关闭；未保存文档需手动绑定）"}`;
            return false;
        }
    }
    autoWrapDocument.path=wrapDocumentPath(bound)||autoWrapDocument.path;
    autoWrapDocument.title=bound.title||autoWrapDocument.title;
    label.textContent=`自动回落对象：${autoWrapDocument.title} [${autoWrapDocument.id}]${doc&&doc.id===autoWrapDocument.id?"":"（已切换其他文档，暂停）"}`;
    return Boolean(doc&&doc.id===autoWrapDocument.id);
}
function observeWrapBaseline(){try{const state=wrapObservation();if(state)wrapObserved=state;}catch(error){}}
function queueAutoWrap(state,source,delay){
    const previous=autoWrapPending;
    const firstHistory=previous&&previous.documentID===state.documentID&&previous.layerID===state.layerID?previous.beforeHistoryID:undefined;
    autoWrapPending={...state,beforeHistoryID:firstHistory===undefined?state.beforeHistoryID:firstHistory,at:previous&&previous.documentID===state.documentID&&previous.layerID===state.layerID?previous.at:Date.now(),delay,source};
    setStatus("wrap-status",`已检测到${source}：${state.name||state.historyID}；等待至少${delay}ms稳定后自动回落。`);
}
function onPaint(){
    if(operationBusy||busy||!$("auto-wrap").checked||mode()!==1)return;
    try{if(!autoWrapDocument||!updateWrapBinding())return;const state=wrapObservation();if(!state)return;const previous=wrapObserved;wrapObserved=state;queueAutoWrap({...state,beforeHistoryID:previous&&previous.documentID===state.documentID&&previous.layerID===state.layerID&&previous.historyID!==state.historyID?previous.historyID:undefined},"paint绘画事件",200);}catch(error){setStatus("wrap-status",`绘画事件读取失败：${error.message||error}`,true);}
}
function pollAutoWrap(){
    if(operationBusy||busy||!$("auto-wrap").checked||mode()!==1)return;
    try{
        if(!autoWrapDocument||!updateWrapBinding()){autoWrapPending=null;return;}
        const state=wrapObservation(),previous=wrapObserved;if(state)wrapObserved=state;
        if(!state||!previous)return;
        if(state.documentID!==previous.documentID||state.layerID!==previous.layerID){autoWrapPending=null;return;}
        if(state.historyID===previous.historyID)return;
        // Existing history entries are undo/redo/navigation, not a newly finished stroke.
        if(previous.historyIDs.includes(state.historyID)){autoWrapPending=null;return;}
        const paintHistory=/brush|pencil|画笔|畫筆|铅笔|鉛筆|ブラシ|鉛筆|브러시|연필|pinsel|buntstift|pinceau|crayon/i.test(state.name);
        if(!paintHistory){autoWrapPending=null;setStatus("wrap-status",`历史检测：${state.name||state.historyID}，未识别为画笔笔触，未自动回落。`);return;}
        if(state.historyIDs.length&&state.historyIDs[state.historyIDs.length-1]!==state.historyID){autoWrapPending=null;return;}
        queueAutoWrap({...state,beforeHistoryID:previous.historyID},"画笔历史变化（事件兜底）",200);
    }catch(error){setStatus("wrap-status",`历史检测暂不可用：${error.message||error}`,true);}
}
async function enableAutoWrap(bindCurrent=false){
    autoWrapPending=null;if(!$("auto-wrap").checked){stopWrapMonitor();autoWrapDocument=null;wrapObserved=null;updateWrapBinding();setStatus("wrap-status","自动回落已关闭，可点击按钮手动执行。");return;}
    try{
        if(!bindCurrent)throw new Error("请点击“绑定当前文档”开启本次工作，避免误绑定其他文档。");
        wrapTarget();
        const doc=ps.app.activeDocument;autoWrapDocument={id:doc.id,title:doc.title||String(doc.id),path:wrapDocumentPath(doc)};
        wrapObserved=null;autoWrapRuns=0;observeWrapBaseline();updateWrapBinding();
        setStatus("wrap-last-run","本次绑定尚未执行自动检查，等待新笔触。");
        startWrapMonitor();
        let eventError="";
        if(!paintListenerRegistered){try{await ps.action.addNotificationListener(["paint"],onPaint);paintListenerRegistered=true;}catch(error){eventError="；paint监听不可用，使用历史检测";}}
        setStatus("operation-status","已开启绘画事件与画笔历史检测；仅当前普通层，不支持橡皮在对边同步。");
        setStatus("wrap-status","自动回落已就绪：等待新笔触；paint事件＋画笔历史检测"+eventError+"。");
    }catch(error){stopWrapMonitor();autoWrapDocument=null;wrapObserved=null;updateWrapBinding();$("auto-wrap").checked=false;setStatus("operation-status",error.message||String(error),true);setStatus("wrap-status",`无法开启：${error.message||error}`,true);}
}
function tryAutoWrap(){
    if(!autoWrapPending||operationBusy||busy)return;
    if(!$("auto-wrap").checked||mode()!==1){autoWrapPending=null;return;}
    if(!updateWrapBinding()||!autoWrapPending){autoWrapPending=null;return;}
    if(Date.now()-autoWrapPending.at<(autoWrapPending.delay||200))return;
    const expected=autoWrapPending;autoWrapPending=null;operation(()=>reportedWrap(expected));
}
async function reportedWrap(expected=null){
    try{
        const count=await wrapCurrentLayer(expected);
        if(expected&&$("auto-wrap").checked&&autoWrapDocument&&autoWrapDocument.id===expected.documentID){
            const state=wrapObservation();
            if(!state||state.documentID!==expected.documentID||state.layerID!==expected.layerID){setStatus("wrap-last-run","本次因文档或图层切换而跳过，未修改图层。");return count;}
            if(!count&&state&&state.layerID===expected.layerID&&state.historyID!==expected.historyID&&!(expected.historyIDs||[]).includes(state.historyID)&&/brush|pencil|画笔|畫筆|ブラシ|브러시|pinsel|pinceau/i.test(state.name)){
                queueAutoWrap({...state,beforeHistoryID:expected.beforeHistoryID},"新笔触补检",200);
                setStatus("wrap-last-run","检测到后续笔触，已排队补检。");
            }else{
                autoWrapRuns++;
                setStatus("wrap-last-run",`最近自动检查：第${autoWrapRuns}次，处理${count||0}个越界／边界像素（历史${expected.historyID}）。`);
            }
        }
        return count;
    }catch(error){
        if(expected&&error.number===9&&$("auto-wrap").checked&&autoWrapDocument&&autoWrapDocument.id===expected.documentID){
            autoWrapPending={...expected,at:Date.now(),delay:300,retries:(expected.retries||0)+1};
            setStatus("wrap-status",`PS暂时忙碌，自动回落等待重试（${autoWrapPending.retries}次）；任务未丢弃。`);
            setStatus("wrap-last-run","等待PS释放绘画／模态操作后回落。");
            return;
        }
        setStatus("wrap-status",`${expected?"自动":"手动"}回落未完成：${error.message||error}`,true);
        if(expected)setStatus("wrap-last-run","自动回落失败："+(error.message||String(error)),true);
        throw error;
    }
}
async function newGridDocument(){
    const {columns,rows}=dimensions(),tw=number("width",2,1024),th=number("height",2,1024);
    if(!Number.isInteger(tw)||!Number.isInteger(th)||tw%2||th%2)throw new Error("单格宽高必须为 2–1024 的偶数像素");
    const width=(columns+rows)*tw/2,height=(columns+rows)*th/2,spec={columns,rows,tw,th};
    if(width*height>16*1024*1024||width>8192||height>8192)throw new Error("网格文档最大 8192 像素每边、总计 16777216 像素，请减小格子尺寸");
    await ps.core.executeAsModal(async()=>{
        const doc=await ps.app.documents.add({name:`Tile ${columns}×${rows} · ${tw}×${th}`,width,height,resolution:72,depth:8,mode:ps.constants.NewDocumentMode.RGB,fill:ps.constants.DocumentFill.TRANSPARENT,profile:"sRGB IEC61966-2.1"});
        const paint=doc.layers[0];paint.name="绘画（在网格下方绘制）";
        const guide=await doc.createPixelLayer({name:`辅助网格 [UYTile] ${columns}x${rows} ${tw}x${th}`,blendMode:ps.constants.BlendMode.MULTIPLY});
        guide.move(paint,ps.constants.ElementPlacement.PLACEBEFORE);
        let pixels;
        try{pixels=await ps.imaging.createImageDataFromBuffer(gridRaster(width,height,spec),{width,height,components:4,colorSpace:"RGB",colorProfile:"sRGB IEC61966-2.1"});await ps.imaging.putPixels({documentID:doc.id,layerID:guide.id,imageData:pixels,replace:true});}
        finally{if(pixels)pixels.dispose();}
        guide.blendMode=ps.constants.BlendMode.MULTIPLY;guide.allLocked=true;
        await ps.action.batchPlay([{_obj:"select",_target:[{_ref:"layer",_id:paint.id}],makeVisible:false,_options:{dialogOptions:"dontDisplay"}}],{});
    },{commandName:"新建 Tile 文档与锁定网格"});
    setStatus("operation-status",`已新建 ${width}×${height} 文档，${columns}×${rows} 个菱形格。请在网格下方绘制，另存 PSD 保留网格。`);
}
async function readSlotPixels(context,doc,tw,th){
    return readWithoutGrid(context,doc,async()=>{
        let data;
        try{
            const dw=Number(doc.width),dh=Number(doc.height);
            const result=await ps.imaging.getPixels({documentID:doc.id,sourceBounds:{left:0,top:0,right:dw,bottom:dh},targetSize:{width:tw,height:th},componentSize:8,colorSpace:"RGB",colorProfile:"sRGB IEC61966-2.1",applyAlpha:false});
            data=result.imageData;const rgba=new Uint8Array(tw*th*4),bounds=result.sourceBounds;
            if(!data){if(bounds&&(!(bounds.right>bounds.left)||!(bounds.bottom>bounds.top)))return rgba;throw new Error(`${doc.title} 无法读取像素`);}
            const raw=await data.getData({chunky:true}),levelScale=2**(result.level||0),bw=bounds.right-bounds.left,bh=bounds.bottom-bounds.top,mask=diamondMask(tw,th);
            if(bw>0&&bh>0)for(let y=0;y<th;y++)for(let x=0;x<tw;x++){
                if(!mask[y*tw+x])continue;
                const sx=Math.floor(((x+.5)*dw/tw/levelScale-bounds.left)*data.width/bw),sy=Math.floor(((y+.5)*dh/th/levelScale-bounds.top)*data.height/bh);
                if(sx<0||sy<0||sx>=data.width||sy>=data.height)continue;
                const from=(sy*data.width+sx)*data.components,to=(y*tw+x)*4;
                rgba[to]=raw[from];rgba[to+1]=raw[from+1];rgba[to+2]=raw[from+2];rgba[to+3]=data.components===4?raw[from+3]:255;
            }
            return rgba;
        }finally{if(data)data.dispose();}
    },"读取 Slot 文档");
}
async function convertToSingleDocument(){
    if(mode()!==2)throw new Error("请先切换到多文档 Slot 模式");
    if($("elevated").checked)throw new Error("转换暂仅支持平面菱形，请先关闭带高度 Tile");
    const {columns,rows}=dimensions(),tw=number("width",2,1024),th=number("height",2,1024);
    if(!Number.isInteger(tw)||!Number.isInteger(th)||tw%2||th%2)throw new Error("单格宽高必须为 2–1024 的偶数像素");
    const width=(columns+rows)*tw/2,height=(columns+rows)*th/2,spec={columns,rows,tw,th};
    if(width>8192||height>8192||width*height>16*1024*1024)throw new Error("转换画布最大8192像素每边、总计16777216像素");
    const slots=activeBindings().map(binding=>binding?{...binding}:null);
    if(!slots.some(Boolean))throw new Error("请先为至少一个 Slot 绑定文档");
    let output;
    try{
        await ps.core.executeAsModal(async context=>{
            const docs=documents(),sources=new Map(),pixels=new Map();
            for(const binding of slots)if(binding&&!sources.has(binding.id)){
                const doc=docs.find(doc=>doc.id===binding.id);if(!doc)throw new Error(`${binding.title} 已关闭，请重新绑定后转换`);
                sources.set(binding.id,doc);
            }
            // Read every source before creating the destination; never modify source artwork.
            for(const [id,doc]of sources){if(context.isCancelled)throw new Error("用户取消转换");pixels.set(id,await readSlotPixels(context,doc,tw,th));}
            output=await ps.app.documents.add({name:`Tile ${columns}×${rows} · ${tw}×${th}`,width,height,resolution:72,depth:8,mode:ps.constants.NewDocumentMode.RGB,fill:ps.constants.DocumentFill.TRANSPARENT,profile:"sRGB IEC61966-2.1"});
            let previous=null,first=null;
            for(let row=0;row<rows;row++)for(let col=0;col<columns;col++){
                if(context.isCancelled)throw new Error("用户取消转换");
                const binding=slots[row*columns+col];
                const layer=previous?await output.createPixelLayer({name:"Slot"}):output.layers[0];
                layer.name=`Slot ${row+1},${col+1} · ${binding?sources.get(binding.id).title:"空白"}`;
                if(previous)layer.move(previous,ps.constants.ElementPlacement.PLACEAFTER);else first=layer;
                previous=layer;
                if(!binding)continue;
                let imageData;
                try{
                    imageData=await ps.imaging.createImageDataFromBuffer(pixels.get(binding.id),{width:tw,height:th,components:4,colorSpace:"RGB",colorProfile:"sRGB IEC61966-2.1"});
                    const b=cellBounds(row,col,spec);
                    await ps.imaging.putPixels({documentID:output.id,layerID:layer.id,imageData,replace:true,targetBounds:{left:b.left,top:b.top}});
                }finally{if(imageData)imageData.dispose();}
            }
            const guide=await output.createPixelLayer({name:`辅助网格 [UYTile] ${columns}x${rows} ${tw}x${th}`,blendMode:ps.constants.BlendMode.MULTIPLY});
            guide.move(first,ps.constants.ElementPlacement.PLACEBEFORE);
            let imageData;
            try{imageData=await ps.imaging.createImageDataFromBuffer(gridRaster(width,height,spec),{width,height,components:4,colorSpace:"RGB",colorProfile:"sRGB IEC61966-2.1"});await ps.imaging.putPixels({documentID:output.id,layerID:guide.id,imageData,replace:true});}
            finally{if(imageData)imageData.dispose();}
            guide.blendMode=ps.constants.BlendMode.MULTIPLY;guide.allLocked=true;
            await ps.action.batchPlay([{_obj:"select",_target:[{_ref:"layer",_id:first.id}],makeVisible:false,_options:{dialogOptions:"dontDisplay"}}],{});
        },{commandName:"将 Slot 转换为单文档"});
    }catch(error){throw new Error(`${error.message||error}${output?"；已保留未完成的新文档，源文档未合并，请检查后重试":"；未创建目标文档"}`);}
    selected=0;$("mode").selectedIndex=1;selectedMode=1;$("elevated").checked=false;controlsKey="";cache.clear();slotUI();
    setStatus("operation-status",`已转换为单文档：${columns}×${rows} 格，${slots.length} 个独立像素图层；源文档可见合成画面按设定尺寸读取，图层未合并。请另存 PSD。`);
}
function cellBounds(row,col,spec){const top=atlasPoint(col,row,spec);return {left:top.x-spec.tw/2,top:top.y,right:top.x+spec.tw/2,bottom:top.y+spec.th};}
async function cellPNG(doc,row,col,spec){
    const b=cellBounds(row,col,spec);let data;
    try{
        const result=await ps.imaging.getPixels({documentID:doc.id,sourceBounds:b,componentSize:8,colorSpace:"RGB",colorProfile:"sRGB IEC61966-2.1",applyAlpha:false});
        data=result.imageData;
        if(!data){const s=result.sourceBounds;if(s&&(!(s.right>s.left)||!(s.bottom>s.top)))return png(spec.tw,spec.th,new Uint8Array(spec.tw*spec.th*4),true);throw new Error("当前格无法读取，请先在绘画层绘制内容");}
        if(result.level)throw new Error("PS 返回了缩小的像素缓存，已停止导出以免丢失原始分辨率");
        const raw=await data.getData({chunky:true}),rgba=new Uint8Array(spec.tw*spec.th*4),s=result.sourceBounds,mask=diamondMask(spec.tw,spec.th);
        const bw=s.right-s.left,bh=s.bottom-s.top;
        if(bw>0&&bh>0)for(let y=0;y<spec.th;y++)for(let x=0;x<spec.tw;x++){
            if(!mask[y*spec.tw+x])continue;
            const sx=Math.floor((b.left+x+.5-s.left)*data.width/bw),sy=Math.floor((b.top+y+.5-s.top)*data.height/bh);
            if(sx<0||sy<0||sx>=data.width||sy>=data.height)continue;
            const from=(sy*data.width+sx)*data.components,to=(y*spec.tw+x)*4;
            rgba[to]=raw[from];rgba[to+1]=raw[from+1];rgba[to+2]=raw[from+2];rgba[to+3]=data.components===4?raw[from+3]:255;
        }
        return png(spec.tw,spec.th,rgba,true);
    }finally{if(data)data.dispose();}
}
async function exportGrid(all){
    if(!ps.app.documents.length)throw new Error("请先打开要切片的文档");
    const doc=ps.app.activeDocument,spec=atlasSpec(doc);
    if(!Number.isInteger(spec.tw)||!Number.isInteger(spec.th)||spec.tw%2||spec.th%2||spec.tw>1024||spec.th>1024)throw new Error("导出要求单格宽高为不超过 1024 的偶数像素；文档宽高需是（行数＋列数）的整数倍");
    const storage=require("uxp").storage,parent=await storage.localFileSystem.getFolder();if(!parent)return;
    const name=String(doc.title||"Tile").replace(/[\\/:*?"<>|\x00-\x1f]/g,"_").replace(/[. ]+$/g,"");
    const folder=await parent.createFolder(`${name}-Tiles-${Date.now()}-${Math.floor(Math.random()*10000)}`);
    const cells=[];for(let r=0;r<spec.rows;r++)for(let c=0;c<spec.columns;c++)if(all||r*8+c===selected)cells.push([r,c]);
    let count=0;
    try{
        // A modal read keeps all exported cells at the same document state.
        const run=async context=>{
            for(const [r,c] of cells){if(context&&context.isCancelled)throw new Error("用户取消导出");
                const bytes=await cellPNG(doc,r,c,spec),file=await folder.createFile(`tile_r${String(r+1).padStart(2,"0")}_c${String(c+1).padStart(2,"0")}.png`,{overwrite:false});
                await file.write(bytes,{format:storage.formats.binary});count++;setStatus("operation-status",`正在导出 ${count}/${cells.length}…`);
            }
        };
        if(gridLayers(doc).length)await withoutGrid(doc,run,"导出菱形 Tile PNG");
        else await ps.core.executeAsModal(run,{commandName:"导出菱形 Tile PNG"});
        setStatus("operation-status",`已导出 ${count} 张透明 PNG：${folder.nativePath||folder.name}`);
    }catch(e){throw new Error(`已导出 ${count}/${cells.length}，文件夹保留：${folder.nativePath||folder.name}；${e.message||e}`);}
}
async function operation(task){
    if(operationBusy||busy){setStatus("operation-status","正在读取画面，请稍后重试。");return;}
    operationBusy=true;
    try{await task();}catch(e){setStatus("operation-status",e.message||String(e),true);}
    finally{observeWrapBaseline();operationBusy=false;request(true);}
}
function clear(){ $("preview").innerHTML="";lastKey=""; }
function dimensions(){return {columns:Math.trunc(number("columns",1,8)),rows:Math.trunc(number("rows",1,8))};}
function activeBindings(){const {columns,rows}=dimensions(),result=[];for(let r=0;r<rows;r++)for(let c=0;c<columns;c++)result.push(bindings[r*8+c]);return result;}
function staggerSettings(){
    const mode=Number($("stagger-mode").selectedIndex)||0;
    $("stagger-settings").style.display=mode?"block":"none";
    $("stagger-columns").disabled=mode===0||mode===2;
    $("stagger-rows").disabled=mode===0||mode===1;
    const count=id=>{const value=number(id,0,8);if(!Number.isInteger(value))throw new Error("错位格数必须是0–8的整数");return value;};
    return {mode,columns:mode===1||mode===3?count("stagger-columns"):0,rows:mode===2||mode===3?count("stagger-rows"):0};
}
function staggerSlot(i,j,columns,rows,stagger){
    const mod=(value,period)=>(value%period+period)%period;
    const col=i-(mod(Math.floor(j/rows),2)?stagger.columns:0);
    const row=j-(mod(Math.floor(i/columns),2)?stagger.rows:0);
    return {column:mod(col,columns),row:mod(row,rows)};
}
function chooseSlot(index){selected=index;slotUI();request();}
function documentState(doc){return doc?[doc.id,doc.title,doc.activeHistoryState.id,Number(doc.width),Number(doc.height)]:null;}
function documentList(force=false){
    const docs=documents();const key=JSON.stringify([docs.map(d=>[d.id,d.title]),selected,bindings[selected]]);
    if(!force&&key===documentListKey)return;
    documentListKey=key;documentOptions=[null,...docs.map(d=>({id:d.id,title:d.title}))];
    const bound=bindings[selected];
    let selectedIndex=bound?documentOptions.findIndex(d=>d&&d.id===bound.id):0;
    if(bound&&selectedIndex<0){documentOptions.push(bound);selectedIndex=documentOptions.length-1;}
    const menu=$("document-options");menu.innerHTML="";
    documentOptions.forEach((option,index)=>{
        const item=document.createElement("sp-menu-item");
        item.textContent=option?`${option.title} [${option.id}]${bound&&index===selectedIndex&&!docs.some(d=>d.id===option.id)?"（已关闭）":""}`:"未绑定（留空）";
        item.selected=index===selectedIndex;menu.appendChild(item);
    });
    $("document-picker").selectedIndex=selectedIndex;
}
function slotUI(){
    const atlas=mode()===1,multiple=mode()===2;
    $("slot-controls").style.display=atlas||multiple?"block":"none";
    $("multi-document-controls").style.display=multiple?"block":"none";
    $("atlas-actions").style.display=atlas?"block":"none";
    $("elevated").disabled=atlas;
    $("width-label").textContent=atlas?"单格宽度":"地面宽度";$("height-label").textContent=atlas?"单格高度":"地面高度";
    const {columns,rows}=dimensions();
    selected=Math.min(Math.floor(selected/8),rows-1)*8+Math.min(selected%8,columns-1);
    const area=$("slots");
    const layout=`${columns}x${rows}`;if(slotLayout!==layout){area.innerHTML="";slotLayout=layout;}
    for(let row=0;row<rows;row++)for(let col=0;col<columns;col++){
        const index=row*8+col,binding=bindings[index];
        let button=area.children[row*columns+col];
        if(!button){button=document.createElement("button");button.addEventListener("click",()=>chooseSlot(index));area.appendChild(button);}
        button.textContent=`${row+1},${col+1}${multiple&&binding?"●":""}`;
        button.style.width=`${100/columns}%`;
        button.style.backgroundColor=selected===index?"#315e8c":"#292929";button.style.color="#fff";
        button.style.border="2px solid #ddd";button.style.borderRadius="13px";
        button.title=`Slot ${row*columns+col+1}（${row+1},${col+1}）: ${binding?binding.title:"未绑定"}`;
    }
    $("selected-slot").textContent=`当前 Slot ${Math.floor(selected/8)*columns+selected%8+1}（行 ${Math.floor(selected/8)+1}，列 ${selected%8+1}）`;
    $("grid-size").textContent=atlas?`新建画布：${(columns+rows)*Number($("width").value)/2}×${(columns+rows)*Number($("height").value)/2} 像素，共 ${columns*rows} 格`:"";
    if(multiple)documentList();
}
async function capture(doc,elevated,limit,atlas){
    const dw=Number(doc.width),dh=Number(doc.height),id=doc.id;
    const captureKey=()=>JSON.stringify([doc.activeHistoryState.id,dw,dh,elevated,limit,atlas]);
    const key=captureKey();
    const cached=cache.get(id);if(cached&&cached.key===key){cached.title=doc.title;cached.sourceState=documentState(doc);return cached;}
    let data;
    try{
        const scale=Math.min(1,limit/dw,limit/dh),w=Math.max(1,Math.round(dw*scale)),h=Math.max(1,Math.round(dh*scale));
        const result=await withoutGrid(doc,()=>ps.imaging.getPixels({documentID:id,sourceBounds:{left:0,top:0,right:dw,bottom:dh},targetSize:{width:w,height:h},componentSize:8,colorSpace:"RGB",colorProfile:"sRGB IEC61966-2.1",applyAlpha:false}));
        data=result.imageData;
        if(!data)throw new Error("文档暂无可读取的像素，请先画一笔再刷新");
        const raw=await data.getData({chunky:true});
        const rgba=new Uint8Array(w*h*4),bounds=result.sourceBounds,levelScale=2**(result.level||0),mask=atlas?diamondMask(w,h,atlas.columns,atlas.rows):!elevated?diamondMask(w,h):null;
        const bw=bounds.right-bounds.left,bh=bounds.bottom-bounds.top;
        if(bw>0&&bh>0){
            for(let y=0;y<h;y++)for(let x=0;x<w;x++){
                if(mask&&!mask[y*w+x])continue;
                const sx=Math.floor(((x+0.5)*dw/w/levelScale-bounds.left)*data.width/bw);
                const sy=Math.floor(((y+0.5)*dh/h/levelScale-bounds.top)*data.height/bh);
                if(sx<0||sy<0||sx>=data.width||sy>=data.height)continue;
                const s=(sy*data.width+sx)*data.components,d=(y*w+x)*4;
                rgba[d]=raw[s];rgba[d+1]=raw[s+1];rgba[d+2]=raw[s+2];rgba[d+3]=data.components===4?raw[s+3]:255;
            }
        }
        const value={key:captureKey(),sourceState:documentState(doc),url:"data:image/png;base64,"+base64(png(w,h,rgba)),dw,dh,w,h,title:doc.title};
        cache.set(id,value);return value;
    }finally{if(data)data.dispose();}
}
// Reuse mounted image nodes: clearing the panel restarts image decoding and flashes.
let previewGridCache=null;
function previewGridImage(width,height,columns,rows,left,top,rasterWidth=width,rasterHeight=height){
    const key=JSON.stringify([rasterWidth,rasterHeight,columns,rows]);
    if(!previewGridCache||previewGridCache.key!==key){
        const scale=Math.min(1,512/Math.max(rasterWidth,rasterHeight));
        const w=Math.max(2,Math.round(rasterWidth*scale)),h=Math.max(2,Math.round(rasterHeight*scale)),k=columns+rows;
        // Use the same full-span coordinates and rasterizer as the PSD guide layer.
        // Sampling at source resolution keeps the overlay anchored when zoom changes.
        const rgba=gridRaster(w,h,{columns,rows,tw:2*w/k,th:2*h/k},true);
        previewGridCache={key,url:"data:image/png;base64,"+base64(png(w,h,rgba))};
    }
    const img=document.createElement("img");img.className="preview-grid";img.src=previewGridCache.url;
    Object.assign(img.style,{left:`${left}px`,top:`${top}px`,width:`${width}px`,height:`${height}px`,pointerEvents:"none"});return img;
}
function commitPreview(scene){
    const panel=$("preview");
    for(let i=0;i<scene.length;i++){
        const desired=scene[i],existing=panel.children[i];
        if(existing&&desired.src&&existing.src){
            if(existing.src!==desired.src)existing.src=desired.src;
            existing.className=desired.className;
            existing.style.pointerEvents=desired.style.pointerEvents||"auto";
            for(const key of ["left","top","width","height"])existing.style[key]=desired.style[key];
        }else if(existing){panel.replaceChild(desired,existing);}
        else panel.appendChild(desired);
    }
    while(panel.children.length>scene.length)panel.removeChild(panel.children[panel.children.length-1]);
}
function render(frames,view,fw,fh,zoom,elevated,multiple,columns,stagger={columns:0,rows:0}){
    const panel=$("preview"),scene=[];
    const first=frames.find(Boolean);if(!first&&!multiple){commitPreview(scene);return;}
    const groundW=multiple||elevated?fw:first.dw,groundH=multiple||elevated?fh:first.dh;
    const factor=Math.max(1,24/(Math.min(groundW,groundH)*zoom));
    const sw=groundW*zoom*factor,sh=groundH*zoom*factor;
    const gridColumns=Math.min(16,Math.ceil(view.width/sw/2)+2);
    const repeatRows=Math.max(1,Math.ceil(frames.length/columns));
    const height=frame=>elevated?frame.dh*zoom*factor:sh;
    const maxHeight=Math.max(sh,...frames.filter(Boolean).map(height)),markers=[],grids=[];
    const rows=Math.min(32,Math.ceil((view.height+maxHeight)/sh)+2);
    for(let row=-rows;row<=rows;row++)for(let col=-gridColumns;col<=gridColumns;col++){
        const i=col+Math.floor((row+1)/2),j=Math.floor(row/2)-col;
        const slot=multiple?staggerSlot(i,j,columns,repeatRows,stagger):{column:0,row:0};
        const index=slot.row*columns+slot.column;
        const frame=frames[index];
        const iw=elevated&&frame?frame.dw*zoom*factor:sw,ih=frame?height(frame):sh;
        const left=view.width/2+(col+((row%2+2)%2)/2)*sw-iw/2;
        const top=view.height/2+row*sh/2+sh/2-ih;
        if(left+iw<0||left>view.width||top+ih<0||top>view.height)continue;
        if(frame){
            const img=document.createElement("img");img.src=frame.url;
            img.style.left=`${left}px`;img.style.top=`${top}px`;img.style.width=`${iw}px`;img.style.height=`${ih}px`;scene.push(img);
        }
        if($("show-grid").checked)grids.push(previewGridImage(sw,sh,1,1,view.width/2+(col+((row%2+2)%2)/2)*sw-sw/2,view.height/2+row*sh/2-sh/2,groundW,groundH));
        if(multiple&&$("show-slots").checked){
            const r=slot.row,c=slot.column;
            const marker=document.createElement("div");marker.className="slot-marker";
            marker.textContent=`${r+1},${c+1}`;
            marker.title=`Slot ${r*columns+c+1}: ${frame?frame.title:"未绑定或文档已关闭"}`;
            marker.style.left=`${view.width/2+(col+((row%2+2)%2)/2)*sw-21}px`;
            marker.style.top=`${view.height/2+row*sh/2-8}px`;
            if(selected===r*8+c){marker.style.backgroundColor="#276ca8";marker.style.borderColor="#ffdf50";}
            marker.addEventListener("click",()=>chooseSlot(r*8+c));markers.push(marker);
        }
    }
    for(const grid of grids)scene.push(grid);
    for(const marker of markers)scene.push(marker);
    commitPreview(scene);
}
function renderAtlas(frame,view,spec,zoom){
    const panel=$("preview"),scene=[];if(!frame||!spec){commitPreview(scene);return;}
    const {columns,rows,tw,th}=spec,z=zoom*Math.max(1,24/(Math.min(tw,th)*zoom));
    const iw=frame.dw*z,ih=frame.dh*z,markers=[],grids=[];
    const corners=[];for(const x of [-view.width/2/z-frame.dw/2,view.width/2/z+frame.dw/2])for(const y of [-view.height/2/z-frame.dh/2,view.height/2/z+frame.dh/2])corners.push({m:(x/tw+y/th)/columns,n:(y/th-x/tw)/rows});
    const m0=Math.max(-32,Math.floor(Math.min(...corners.map(c=>c.m)))-1),m1=Math.min(32,Math.ceil(Math.max(...corners.map(c=>c.m)))+1);
    const n0=Math.max(-32,Math.floor(Math.min(...corners.map(c=>c.n)))-1),n1=Math.min(32,Math.ceil(Math.max(...corners.map(c=>c.n)))+1);
    for(let m=m0;m<=m1;m++)for(let n=n0;n<=n1;n++){
        const left=view.width/2-iw/2+(m*columns-n*rows)*tw*z/2;
        const top=view.height/2-ih/2+(m*columns+n*rows)*th*z/2;
        if(left+iw<0||top+ih<0||left>view.width||top>view.height)continue;
        const img=document.createElement("img");img.src=frame.url;img.style.left=`${left}px`;img.style.top=`${top}px`;img.style.width=`${iw}px`;img.style.height=`${ih}px`;scene.push(img);
        if($("show-grid").checked)grids.push(previewGridImage(iw,ih,columns,rows,left,top,frame.w,frame.h));
        if($("show-slots").checked)for(let r=0;r<rows;r++)for(let c=0;c<columns;c++){
            const p=atlasPoint(c+.5,r+.5,spec),x=left+p.x*z,y=top+p.y*z;if(x<0||y<0||x>view.width||y>view.height)continue;
            const marker=document.createElement("div");marker.className="slot-marker";marker.textContent=`${r+1},${c+1}`;
            marker.title=`源文档第 ${r+1} 行、第 ${c+1} 列`;marker.style.left=`${x-21}px`;marker.style.top=`${y-8}px`;
            if(selected===r*8+c){marker.style.backgroundColor="#276ca8";marker.style.borderColor="#ffdf50";}
            marker.addEventListener("click",()=>chooseSlot(r*8+c));markers.push(marker);
        }
    }
    for(const grid of grids)scene.push(grid);
    for(const marker of markers)scene.push(marker);
    commitPreview(scene);
}
async function refresh(force=false){
    if(!visible||operationBusy)return;
    if(busy){if(force)pending=true;return;}
    busy=true;const epoch=generation;
    try{
        previewBackground();
        reconcileControls();
        const docs=documents(),multiple=mode()===2,atlas=mode()===1,elevated=!atlas&&$("elevated").checked;
        if(multiple)documentList();
        const {columns}=dimensions();
        const slots=multiple?activeBindings():[docs.length?{id:ps.app.activeDocument.id}:null];
        const sources=slots.map(binding=>binding?docs.find(doc=>doc.id===binding.id):null);
        const view=viewport(),fw=number("width",2,8192),fh=number("height",2,8192),zoom=number("zoom",0.1,10);
        const spec=atlas&&sources[0]?atlasSpec(sources[0]):null,stagger=multiple?staggerSettings():{mode:0,columns:0,rows:0};
        const renderKey=states=>JSON.stringify([states,slots,multiple,elevated,columns,fw,fh,zoom,view,selected,$("show-slots").checked,$("show-grid").checked,spec,stagger]);
        const key=renderKey(sources.map(documentState));
        if(!force&&key===lastKey)return;
        const used=new Set(sources.filter(Boolean).map(doc=>doc.id));
        for(const id of cache.keys())if(!used.has(id))cache.delete(id);
        if(!used.size){render(sources.map(()=>null),view,fw,fh,zoom,elevated,multiple,columns,stagger);setStatus("status",multiple?"请选择 Slot，再从下拉菜单绑定打开的文档；关闭的文档需重新绑定。":"请打开一个 Photoshop 文档。");lastKey=key;lastViewport=JSON.stringify(view);return;}
        const results=new Map(),failures=[];
        // Bound aggregate raw thumbnail pixels to 16 MiB (before PNG/base64 overhead).
        const limit=Math.min(512,Math.floor(Math.sqrt(16*1024*1024/4/used.size)));
        for(const doc of sources){
            if(epoch!==generation||!visible)return;
            if(!doc||results.has(doc.id))continue;
            try{results.set(doc.id,await capture(doc,elevated,limit,spec));}
            catch(e){results.set(doc.id,null);failures.push(`${doc.title}: ${e.message||e}`);}
        }
        if(epoch!==generation||!visible)return;
        const currentIds=new Set(documents().map(doc=>doc.id));
        const frames=sources.map(doc=>doc&&currentIds.has(doc.id)?results.get(doc.id):null);
        if(!failures.length){if(atlas)renderAtlas(frames[0],view,spec,zoom);else render(frames,view,fw,fh,zoom,elevated,multiple,columns,stagger);}
        const missing=slots.filter((binding,index)=>binding&&!sources[index]).length;
        const first=frames.find(Boolean);
        setStatus("status",multiple?`已显示 ${frames.filter(Boolean).length}/${slots.length} 个 Slot · ${used.size} 个文档 · 采样上限 ${limit}px${missing?`；${missing} 个文档已关闭，请重新绑定`:""}`:(first?`${first.title} · ${first.dw}×${first.dh} · 预览采样 ${first.w}×${first.h}`:"预览未更新"));
        if(atlas&&spec)$("status").textContent+=` · 菱形网格 ${spec.columns}×${spec.rows}`;
        if(multiple&&stagger.mode)$("status").textContent+=` · 错位预览（列${stagger.columns}／行${stagger.rows}）`;
        if(failures.length)setStatus("status",$("status").textContent+"；"+failures.join("；"),true);
        if(!failures.length)lastKey=renderKey(sources.map(doc=>doc?results.get(doc.id).sourceState:null));lastViewport=JSON.stringify(view);
    }catch(error){setStatus("status",`预览未更新：${error.message||error}`,true);}
    finally{busy=false;if(pending&&visible){pending=false;refresh(true);}}
}
function startWrapMonitor(){if(wrapTimer)return;wrapTimer=setInterval(()=>{pollAutoWrap();tryAutoWrap();},300);}
function stopWrapMonitor(){if(wrapTimer)clearInterval(wrapTimer);wrapTimer=null;autoWrapPending=null;}
function start(){visible=true;generation++;if(timer)clearInterval(timer);lastKey="";timer=setInterval(()=>{if(!visible||autoWrapPending||operationBusy)return;if($("live").checked||JSON.stringify(viewport())!==lastViewport)refresh();},300);refresh(true);}
function stop(){visible=false;generation++;pending=false;if(timer)clearInterval(timer);timer=null;}
function destroyPanel(){stop();stopWrapMonitor();autoWrapDocument=null;wrapObserved=null;$("auto-wrap").checked=false;if(paintListenerRegistered){ps.action.removeNotificationListener(["paint"],onPaint).catch(()=>{});paintListenerRegistered=false;}}
function request(forcePixels=false,wake=false){lastKey="";if(forcePixels)cache.clear();if(!visible){if(wake)start();return;}refresh(true);}
$("refresh").addEventListener("click",()=>request(true,true));
$("background-mode").addEventListener("change",previewBackground);
$("background-color").addEventListener("change",previewBackground);
$("background-swatch").addEventListener("click",pickBackgroundColor);
let lastZoom=1;
function updateZoom(source,commit=false){
    const value=Number(source.value);
    if(!String(source.value).trim()||!Number.isFinite(value)){if(commit){$("zoom").value=String(lastZoom);$("zoom-slider").value=String(lastZoom);}return;}
    if(!commit&&(value<0.1||value>10))return;
    const zoom=Math.max(0.1,Math.min(10,value));
    if(source!==$("zoom")||commit)$("zoom").value=String(zoom);
    $("zoom-slider").value=String(zoom);
    if(lastZoom!==zoom){lastZoom=zoom;request();}
}
for(const id of ["zoom","zoom-slider"]){
    $(id).addEventListener("input",()=>updateZoom($(id)));
    $(id).addEventListener("change",()=>updateZoom($(id),true));
}
$("zoom-reset").addEventListener("click",()=>{$("zoom").value="1";updateZoom($("zoom"),true);});
const previewStage=$("preview-stage");
function focusPreview(){if(typeof previewStage.focus==="function")previewStage.focus();}
previewStage.addEventListener("mouseenter",focusPreview);
previewStage.addEventListener("mousedown",focusPreview);
previewStage.addEventListener("wheel",event=>{
    let delta=Number(event.deltaY);
    if(!Number.isFinite(delta)||delta===0)return;
    if(event.deltaMode===1)delta*=16;
    else if(event.deltaMode===2)delta*=viewport().height;
    delta=Math.max(-120,Math.min(120,delta));
    event.preventDefault();event.stopPropagation();
    $("zoom").value=String(Math.round(Math.max(0.1,Math.min(10,lastZoom*Math.exp(-delta*0.002)))*1000)/1000);
    updateZoom($("zoom"),true);
},{capture:true,passive:false});
for(const id of ["width","height","elevated","show-slots","show-grid"])$(id).addEventListener("change",()=>{slotUI();request();});
for(const id of ["stagger-mode","stagger-columns","stagger-rows"])$(id).addEventListener("change",()=>request());
for(const id of ["columns","rows"])$(id).addEventListener("change",()=>{try{slotUI();request();}catch(e){setStatus("status",e.message,true);}});
for(const size of [2,3,4])$("preset-"+size).addEventListener("click",()=>{$("columns").value=String(size);$("rows").value=String(size);slotUI();request();});
$("mode").addEventListener("change",()=>{slotUI();request();});
$("new-grid-document").addEventListener("click",()=>operation(newGridDocument));
$("wrap-layer").addEventListener("click",()=>operation(()=>reportedWrap()));
$("auto-wrap").addEventListener("change",()=>enableAutoWrap(false));
$("bind-auto-wrap").addEventListener("click",()=>{ $("auto-wrap").checked=true;return enableAutoWrap(true); });
$("convert-single-document").addEventListener("click",()=>operation(convertToSingleDocument));
$("export-cell").addEventListener("click",()=>operation(()=>exportGrid(false)));
$("export-grid").addEventListener("click",()=>operation(()=>exportGrid(true)));
$("reload-documents").addEventListener("click",()=>{documentList(true);request();});
$("document-picker").addEventListener("change",()=>{if(mode()!==2)return;const index=$("document-picker").selectedIndex;if(index<0||index>=documentOptions.length)return;const binding=documentOptions[index];if((bindings[selected]?.id??null)===(binding?.id??null))return;bindings[selected]=binding;slotUI();request();});
$("bind").addEventListener("click",()=>{if(!ps.app.documents.length){setStatus("status","请先打开要绑定的 PSD 文档。");return;}const doc=ps.app.activeDocument;bindings[selected]={id:doc.id,title:doc.title};slotUI();request();});
$("unbind").addEventListener("click",()=>{bindings[selected]=null;slotUI();request();});
entrypoints.setup({plugin:{destroy:destroyPanel},panels:{tilePreview:{create:start,show:start,hide:stop,destroy:destroyPanel}}});
start();
