const ps=require("photoshop");
const {entrypoints}=require("uxp");
// Register before loading resources or building the UI. UXP may invoke create
// immediately, so defer panel work until all controls and handlers are ready.
let panelReady=false;
const panels={tilePreview:{content:"controls",root:null,shown:false},tilePreviewView:{content:"view-area",root:null,shown:false}};
function panelVisibility(id,root,shown){
    const panel=panels[id];if(root)panel.root=root;panel.shown=shown;
    if(panelReady)syncPanels();
}
function reportStartupFailure(error){
    const message=`插件初始化失败：${error.message||error}。请关闭并重新打开插件。`;
    const status=document.getElementById("status"),refreshButton=document.getElementById("refresh");
    if(status){status.textContent="";}
    if(refreshButton)refreshButton.disabled=true;
    const controls=document.getElementById("controls"),notice=document.getElementById("panel-error");
    if(controls)controls.style.display="flex";
    if(notice){notice.textContent=message;notice.style.color="#ff6e6e";}
    console.error(message);
}
try{entrypoints.setup({plugin:{
    // PS 2024 requires create whenever plugin-level lifecycle hooks are supplied.
    create:()=>{},destroy:()=>{for(const panel of Object.values(panels))panel.shown=false;if(panelReady)destroyPanel();}
},panels:{tilePreview:{
    create:root=>panelVisibility("tilePreview",root,true),show:root=>panelVisibility("tilePreview",root,true),
    hide:()=>panelVisibility("tilePreview",null,false),destroy:()=>panelVisibility("tilePreview",null,false)
},tilePreviewView:{
    create:root=>panelVisibility("tilePreviewView",root,true),show:root=>panelVisibility("tilePreviewView",root,true),
    hide:()=>panelVisibility("tilePreviewView",null,false),destroy:()=>panelVisibility("tilePreviewView",null,false)
}}});}catch(error){reportStartupFailure(error);throw error;}
const {png,base64}=require("./png.js");
// UXP panels have separate roots but share this script. Keep direct references:
// document.getElementById may no longer find a control after its root is detached.
const uiNodes=new Map(Array.from(document.querySelectorAll("[id]"),node=>[node.id,node]));
const $=id=>uiNodes.get(id);
let autoWrapEnabled=false;
$("version").textContent=`版本 ${require("./manifest.json").version}`;
let timer=null,busy=false,visible=false,generation=0,lastKey="",pending=false;
let selected=0,bindings=Array(64).fill(null),documentListKey="",documentOptions=[];
let operationBusy=false,slotLayout="";
let selectedMode=1,controlsKey="";
let documentSettingsOpen=true;
const documentSettingsSession=new Map();
let settingsDocumentID=null;
let autoWrapPending=null,paintListenerRegistered=false;
let wrapObserved=null,autoWrapDocument=null,wrapTimer=null,autoWrapRuns=0;
const cache=new Map();
const diamondMasks=new Map();
let backgroundKey="",checkerURL="";
let subgroupGridCache=new Map();
function setStatus(id,text,error=false){
    const panelErrors=setStatus.errors||(setStatus.errors=new Map());
    const dismissed=setStatus.dismissed||(setStatus.dismissed=new Map());
    const node=$(id),color=error?"#ff6e6e":"#ddd";
    if(node.textContent!==text)node.textContent=text;
    if(node.style.color!==color)node.style.color=color;
    if(error&&text){
        if(dismissed.get(id)===text){panelErrors.delete(id);}
        else if(panelErrors.get(id)!==text){panelErrors.delete(id);panelErrors.set(id,text);}
    }else {panelErrors.delete(id);dismissed.delete(id);}
    const centralized=id==="operation-status"||id==="preview-panel-status";
    if(centralized){
        const info=setStatus.info||(setStatus.info=new Map());
        if(!error&&text){if(info.get(id)!==text){
            info.delete(id);info.set(id,text);
            const versions=setStatus.infoVersions||(setStatus.infoVersions=new Map());
            const version=(versions.get(id)||0)+1;versions.set(id,version);
            if(typeof setTimeout==="function")setTimeout(()=>{
                if(versions.get(id)===version&&info.get(id)===text&&!panelErrors.has(id))setStatus(id,"");
            },5000);
        }}else info.delete(id);
    }
    const display=error||centralized||id==="wrap-status"||id==="wrap-last-run"?"none":text?"block":"none";
    if(node.style.display!==display)node.style.display=display;
    const notice=$("panel-error");
    if(notice){
        const messages=[...panelErrors.values()];
        // Only one error is shown, even when several operations report the same failure.
        const info=Array.from((setStatus.info||new Map()).values());
        const message=messages.length?messages[messages.length-1]:info.length?info[info.length-1]:"";
        const noticeColor=messages.length?"#ff6e6e":"#ddd";
        if(notice.style.color!==noticeColor)notice.style.color=noticeColor;
        if(notice.textContent!==message)notice.textContent=message;
    }
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
        if(!channels||!channels.every(Number.isFinite))throw new Error("调色板未返回RGB颜色，请重新选择颜色。");
        $("background-color").value="#"+channels.map(v=>Math.round(Math.max(0,Math.min(255,v))).toString(16).padStart(2,"0")).join("").toUpperCase();
        previewBackground();
    }catch(error){if(error.number!==-128&&error.result!==-128)setStatus("background-error",error.message||String(error),true);}
    finally{operationBusy=false;}
}
function mode(){const index=$("mode").selectedIndex;if(Number.isInteger(index)&&index>=0&&index<3)selectedMode=index;return selectedMode;}
const noDocumentMessage="请打开一个文档，或点击“新建文档”。";
function updateDocumentAvailability(){
    const available=ps.app.documents.length>0;
    if($("expand-grid-document").disabled!==!available)$("expand-grid-document").disabled=!available;
    if(!available){
        if($("preview").children.length)commitPreview([]);
        cache.clear();lastKey="";
        if($("status").textContent!==noDocumentMessage)setStatus("status",noDocumentMessage);
    }
    return available;
}
function validateGridSettings(){
    let gridSettingsError="";
    if(mode()===1&&ps.app.documents.length){
        try{atlasSpec(ps.app.activeDocument);}catch(error){gridSettingsError=error.message||String(error);}
    }
    setStatus("grid-settings-status",gridSettingsError,true);
    return !gridSettingsError;
}
function rememberDocumentSettings(){
    // Multi-document Slot mode is one shared composition, not an active-document workspace.
    if(mode()===2){settingsDocumentID=null;return;}
    const ids=["mode","width","height","columns","rows","elevated","joint-wrap","group-columns","group-rows","stagger-mode","stagger-columns","stagger-rows"];
    const capture=()=>({selected,controls:ids.map(id=>{const node=$(id);return {id,value:node.value,checked:node.checked,selectedIndex:node.selectedIndex};})});
    const docs=documents(),doc=docs.length?ps.app.activeDocument:null,id=doc?doc.id:null;
    for(const key of documentSettingsSession.keys())if(!docs.some(doc=>doc.id===key))documentSettingsSession.delete(key);
    if(id===settingsDocumentID){if(id!==null)documentSettingsSession.set(id,capture());return;}
    if(settingsDocumentID!==null)documentSettingsSession.set(settingsDocumentID,capture());
    const saved=documentSettingsSession.get(id);
    settingsDocumentID=id;
    if(saved){
        for(const entry of saved.controls){const node=$(entry.id);if(node.value!==entry.value)node.value=entry.value;if(node.checked!==entry.checked)node.checked=entry.checked;if(node.selectedIndex!==entry.selectedIndex)node.selectedIndex=entry.selectedIndex;}
        selected=saved.selected;selectedMode=$("mode").selectedIndex;
        autoWrapPending=null;wrapObserved=null;controlsKey="";lastKey="";cache.clear();generation++;
        for(const source of ["grid-settings-status","status"])setStatus(source,"");
    }
    for(const key of documentSettingsSession.keys())if(!docs.some(doc=>doc.id===key))documentSettingsSession.delete(key);
    if(id!==null)documentSettingsSession.set(id,capture());
}
function reconcileControls(){
    rememberDocumentSettings();
    updateDocumentAvailability();
    validateGridSettings();
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
function gridPaths(doc){
    const found=[];
    for(const path of Array.from(doc.pathItems||[])){
        const m=/^辅助网格路径 \[UYTile\] (\d+)x(\d+) (\d+)x(\d+)$/.exec(path.name||"");
        if(m)found.push({path,columns:Number(m[1]),rows:Number(m[2]),tw:Number(m[3]),th:Number(m[4])});
    }
    return found;
}
function atlasSpec(doc){
    const {columns,rows}=dimensions(),k=columns+rows,tw=number("width",2,8192),th=number("height",2,8192);
    if(!Number.isInteger(tw)||!Number.isInteger(th)||tw%2||th%2)throw new Error("单格宽高必须为偶数像素");
    const width=k*tw/2,height=k*th/2,dw=Number(doc.width),dh=Number(doc.height);
    if(dw<width||dh<height)throw new Error(`当前画布 ${dw}×${dh} 放不下 ${columns}×${rows} 网格；中心 Tile 区域需要 ${width}×${height}。请扩大画布或减小网格`);
    const guides=[...gridLayers(doc),...gridPaths(doc)];
    if(guides.length>1)throw new Error("文档有多个辅助网格，请先保留一个再预览或导出");
    if(guides.length&&(guides[0].columns!==columns||guides[0].rows!==rows||guides[0].tw!==tw||guides[0].th!==th))throw new Error(`辅助网格与面板设置不同，请改回 ${guides[0].columns}×${guides[0].rows}，或按新设置新建文档`);
    // Keep every tile on whole pixels when an odd canvas leaves half-pixel margins.
    return {columns,rows,tw,th,width,height,left:Math.floor((dw-width)/2),top:Math.floor((dh-height)/2),workspace:dw>width||dh>height};
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
            for(const s of states){if(s.layer.allLocked)s.layer.allLocked=false;if(s.layer.visible)s.layer.visible=false;}
            return await read(context);
        }finally{
            // Roll back the temporary visibility/lock edits, including on cancellation.
            // This keeps helper lines out of images and avoids adding capture history.
            // Restore explicitly while still suspended: no standalone visibility history.
            // Keep user-hidden guides hidden rather than forcing every guide visible.
            try{
                for(const s of states){
                    const layer=gridLayers(doc).find(g=>g.layer.id===s.layer.id)?.layer;
                    if(!layer)throw new Error("读取后未找到辅助网格");
                    if(layer.allLocked)layer.allLocked=false;
                    if(layer.visible!==s.visible)layer.visible=s.visible;
                    if(layer.allLocked!==s.locked)layer.allLocked=s.locked;
                }
            }finally{await context.hostControl.resumeHistory(token,false);}
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
function compositeWrapStroke(out,to,rgba,from,before){
    const sa=rgba[from+3]/255,ba=before?before[from+3]/255:0,da=out[to+3]/255;
    if(before&&sa<ba){out[to+3]=Math.round(255*da*sa/ba);if(!out[to+3])out.fill(0,to,to+4);return;}
    // Recover the added paint from before/after premultiplied pixels.
    const recover=Boolean(before&&ba<1&&sa>ba);
    let a=recover?(sa-ba)/(1-ba):sa;
    // Opaque (or quantized equal-alpha) before/after samples do not expose brush alpha.
    // Use the minimum source-over contribution consistent with the color change,
    // rather than copying the source background as fully opaque paint.
    const colorOnly=Boolean(before&&sa===ba&&sa>0);
    if(colorOnly){
        a=0;
        for(let c=0;c<3;c++){const old=before[from+c]/255,next=rgba[from+c]/255;
            if(next>old&&old<1)a=Math.max(a,(next-old)/(1-old));
            else if(next<old&&old>0)a=Math.max(a,(old-next)/old);
        }
    }
    if(a<=0)return;
    const alpha=a+da*(1-a);
    for(let c=0;c<3;c++){
        const paint=recover?Math.max(0,Math.min(1,(rgba[from+c]/255*sa-before[from+c]/255*ba*(1-a))/a)):colorOnly?Math.max(0,Math.min(1,(rgba[from+c]/255-before[from+c]/255*(1-a))/a)):rgba[from+c]/255;
        out[to+c]=Math.round(255*(paint*a+out[to+c]/255*da*(1-a))/alpha);
    }
    out[to+3]=Math.round(alpha*255);
}
function wrapPixels(rgba,width,height,left,top,spec,before=null){
    if(spec.workspace)return wrapPixels(rgba,width,height,left-spec.left,top-spec.top,{...spec,left:0,top:0,workspace:false},before);
    if(spec.group)return wrapGroupedPixels(rgba,width,height,left,top,spec,before);
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
const groupColors=["#63b3ed","#81c784","#ffb74d","#ba9dea","#f48fb1","#4dd0e1"];
function subgroupSettings(spec){
    if(!$("joint-wrap")||!$("joint-wrap").checked||mode()!==1)return null;
    const columns=number("group-columns",1,8),rows=number("group-rows",1,8);
    if(!Number.isInteger(columns)||!Number.isInteger(rows)||spec.columns%columns||spec.rows%rows)throw new Error("子组行列必须整除整体行列，请调整子组尺寸。");
    const column=Math.floor(selected%8),row=Math.floor(selected/8),u=Math.floor(column/columns)*columns,v=Math.floor(row/rows)*rows;
    return {columns,rows,u,v,index:subgroupIndex(column,row,spec,{columns,rows}),across:spec.columns/columns,down:spec.rows/rows};
}
function subgroupIndex(col,row,spec,group){return Math.floor(row/group.rows)*(spec.columns/group.columns)+Math.floor(col/group.columns);}
function wrapGroupedPixels(rgba,width,height,left,top,spec,before){
    if(!before||before.length!==rgba.length)throw new Error("联合回落缺少可用的笔触前记录，本次未写入；请确认历史记录仍可用后重新绑定再绘制。");
    const group=spec.group,local={columns:group.columns,rows:group.rows,tw:spec.tw,th:spec.th};
    const cw=(spec.columns+spec.rows)*spec.tw/2,ch=(spec.columns+spec.rows)*spec.th/2;
    const gw=(group.columns+group.rows)*spec.tw/2,gh=(group.columns+group.rows)*spec.th/2;
    const gm=diamondMask(gw,gh,group.columns,group.rows),mask=diamondMask(cw,ch,spec.columns,spec.rows);
    const origin=atlasPoint(group.u,group.v,spec),gl=origin.x-group.rows*spec.tw/2,gt=origin.y;
    const changed=[],out=new Uint8Array(rgba),same=(a,i,b,j)=>a[i]===b[j]&&a[i+1]===b[j+1]&&a[i+2]===b[j+2]&&a[i+3]===b[j+3];
    let lowU=0,highU=0,lowV=0,highV=0;
    for(let y=0;y<height;y++)for(let x=0;x<width;x++){
        const i=(y*width+x)*4;if(same(rgba,i,before,i)||(!rgba[i+3]&&!before[i+3]))continue;
        const gx=x+left,gy=y+top,u=(gx+.5)/spec.tw+(gy+.5)/spec.th-spec.rows/2-group.u,v=(gy+.5)/spec.th-(gx+.5)/spec.tw+spec.rows/2-group.v;
        changed.push({i,gx,gy,u,v});lowU=Math.max(lowU,-u);highU=Math.max(highU,u-group.columns);lowV=Math.max(lowV,-v);highV=Math.max(highV,v-group.rows);
    }
    // The raster guide fringe and its half-open partner form one boundary class.
    const aliases=new Map();
    for(let y=0;y<gh;y++)for(let x=0;x<gw;x++)if(gm[y*gw+x]&&!insideWrapUnit(x,y,local)){
        const p=wrappedPixel(x+.5,y+.5,local),tx=Math.floor(p.x),ty=Math.floor(p.y),key=ty*gw+tx;
        if(!aliases.has(key))aliases.set(key,new Map());
        aliases.get(key).set(y*gw+x,{x,y});aliases.get(key).set(key,{x:tx,y:ty});
    }
    const canonical=new Map();let moved=0,copied=0;
    const epsilon=1/spec.tw+1/spec.th;
    for(const item of changed){
        const {i,gx,gy,u,v}=item,p=wrappedPixel(gx-gl+.5,gy-gt+.5,local),x=Math.floor(p.x),y=Math.floor(p.y),key=y*gw+x;
        const outside=u<0||u>=group.columns||v<0||v>=group.rows;
        const strip=outside||aliases.has(key)||(highU>0&&u>=group.columns-highU-epsilon)||(lowU>0&&u<=lowU+epsilon)||(highV>0&&v>=group.rows-highV-epsilon)||(lowV>0&&v<=lowV+epsilon);
        if(!strip)continue;
        const priority=outside?2:1,old=canonical.get(key);
        if(!old)canonical.set(key,{i,x,y,priority,sources:[i]});
        else {old.sources.push(i);if(priority>=old.priority){old.i=i;old.priority=priority;}}
        if(!(gx>=0&&gx<cw&&gy>=0&&gy<ch&&mask[gy*cw+gx])){out.set(before.subarray(i,i+4),i);moved++;}
    }
    // Write every equivalent target exactly once: no repeated alpha compositing.
    const writes=new Map();
    for(const [key,item]of canonical){
        const points=aliases.has(key)?Array.from(aliases.get(key).values()):[{x:item.x,y:item.y}];
        for(let r=0;r<spec.rows;r+=group.rows)for(let c=0;c<spec.columns;c+=group.columns){
            const o=atlasPoint(c,r,spec),l=o.x-group.rows*spec.tw/2;
            for(const point of points){const gx=Math.round(l+point.x),gy=Math.round(o.y+point.y),x=gx-left,y=gy-top;
                if(gx<0||gy<0||gx>=cw||gy>=ch||!mask[gy*cw+gx])continue;
                if(x<0||y<0||x>=width||y>=height)throw new Error("子组回落目标超出读取范围，未写入图层");
                const to=(y*width+x)*4;
                if(!writes.has(to))writes.set(to,new Set());
                for(const from of item.sources)writes.get(to).add(from);
            }
        }
    }
    for(const [to,sources]of writes){
        const seen=new Set();
        for(const from of sources){
            const signature=Array.from(before.subarray(from,from+4)).join(",")+":"+Array.from(rgba.subarray(from,from+4)).join(",");
            if(seen.has(signature))continue;seen.add(signature);
            if(to===from||same(rgba,to,rgba,from)&&same(before,to,before,from))continue;
            compositeWrapStroke(out,to,rgba,from,before);copied++;
        }
    }
    return {rgba:out,moved,copied,translations:[]};
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
    }catch(error){
        const baseline=enableAutoWrap.emptyBaselines&&enableAutoWrap.emptyBaselines.get(`${doc.id}:${layer.id}`);
        const emptyArea=/^No pixels in the requested area[.!]?$/i.test(String(error.message||error).trim());
        if(emptyArea||baseline===historyID)return new Uint8Array((bounds.right-bounds.left)*(bounds.bottom-bounds.top)*4);
        throw new Error(`无法读取笔触前图层像素：${error.message||error}；图层绑定仍保留，本次未写入。`);
    }finally{if(data)data.dispose();}
}
function wrapTarget(expected){
    if(mode()!==1)throw new Error("回落仅用于单文档菱形网格模式");
    if(!ps.app.documents.length)throw new Error("请先打开单文档网格");
    const doc=ps.app.activeDocument,layers=doc.activeLayers;
    if(!layers||layers.length!==1)throw new Error("请仅选中一个普通像素图层");
    const layer=layers[0];
    if(expected&&(!autoWrapEnabled||!isWrapLayerBound(doc.id,layer.id)))return null;
    if(expected&&(doc.id!==expected.documentID||layer.id!==expected.layerID||doc.activeHistoryState.id!==expected.historyID))return null;
    if(layer.kind!==ps.constants.LayerKind.NORMAL||layer.isBackgroundLayer||/^辅助网格 \[UYTile\]/.test(layer.name||""))throw new Error("请选择绘画用的普通像素层，不能回落辅助网格／组／智能对象／背景层");
    if(layer.allLocked||layer.pixelsLocked||layer.transparentPixelsLocked)throw new Error("当前图层像素或透明度已锁定，请先解锁");
    if(layer.visible===false||layer.opacity!==100||layer.fillOpacity!==100)throw new Error("回落暂要求图层可见，不透明度与填充均为100%");
    if(layer.blendMode!==undefined&&layer.blendMode!==ps.constants.BlendMode.NORMAL)throw new Error("回落暂要求图层混合模式为正常");
    if(doc.mode!==ps.constants.DocumentMode.RGB||doc.bitsPerChannel!==ps.constants.BitsPerChannelType.EIGHT)throw new Error("回落暂支持RGB 8位文档，不会自动降低其他文档位深");
    const spec=atlasSpec(doc);
    if(spec.tw%2||spec.th%2||spec.tw>1024||spec.th>1024)throw new Error("回落单格宽高须为2–1024的偶数");
    if(expected&&expected.selectedSlot!==undefined&&expected.selectedSlot!==selected)return null;
    const group=subgroupSettings(spec);if(group)spec.group=group;
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
        const bounds=layer.boundsNoEffects,left=Math.min(spec.left,Math.floor(Number(bounds.left))),top=Math.min(spec.top,Math.floor(Number(bounds.top))),right=Math.max(spec.left+spec.width,Math.ceil(Number(bounds.right))),bottom=Math.max(spec.top+spec.height,Math.ceil(Number(bounds.bottom)));
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
            if(expected&&!wrapTarget(expected)){moved=0;copied=0;setStatus("wrap-status","自动回落已取消：绑定或当前图层已改变。");return;}
            history=await context.hostControl.suspendHistory({documentID:doc.id,name:"菱形回落（当前图层）"});
            await ps.imaging.putPixels({documentID:doc.id,layerID:layer.id,imageData,replace:true,targetBounds:{left,top}});
            if(context.isCancelled)throw new Error("用户取消回落");
            await context.hostControl.resumeHistory(history,true);history=null;
        }finally{
            try{if(history)await context.hostControl.resumeHistory(history,false);}
            finally{if(imageData)imageData.dispose();if(data)data.dispose();}
        }
    },{commandName:"菱形回落（当前图层）"});
    if(moved||copied){cache.clear();lastKey="";setStatus("operation-status","");setStatus("wrap-status",`${label}完成：搬移${moved}／边界复制${copied}，已请求刷新。${copied&&!usedHistory?"无前一历史像素，边界按外缘向内同步。":""}${translations.length?"位移（x,y）："+translations.slice(0,4).map(t=>`(${t.offset}) ${t.count}像素`).join("；"):""}`);}return moved+copied;
}
function wrapObservation(){
    const doc=ps.app.activeDocument;
    if(!doc||!doc.activeLayers||doc.activeLayers.length!==1)return null;
    if(!isWrapLayerBound(doc.id,doc.activeLayers[0].id))return null;
    const history=doc.activeHistoryState;
    if(!history)return null;
    return {documentID:doc.id,layerID:doc.activeLayers[0].id,historyID:history.id,name:String(history.name||""),historyIDs:Array.from(doc.historyStates||[],h=>h.id)};
}
function isWrapLayerBound(documentID,layerID){return Boolean(autoWrapDocument&&autoWrapDocument.id===documentID&&autoWrapDocument.layers.has(layerID));}
function wrapDocumentPath(doc){
    try{
        const path=String(doc.path||"").trim().replace(/\\/g,"/");
        if(/^[a-z]:\//i.test(path)||path.startsWith("//"))return path.toLowerCase();
        return path.startsWith("/")||path.startsWith("file:")?path:"";
    }catch(error){return "";}
}
function updateWrapBinding(){
    const label=$("auto-wrap-document"),button=$("bind-auto-wrap");
    const docs=documents(),doc=docs.length?ps.app.activeDocument:null;
    const current=doc&&doc.activeLayers&&doc.activeLayers.length===1?doc.activeLayers[0]:null;
    const show=(text,bound=false)=>{
        if(label.textContent!==text)label.textContent=text;
        const caption=bound?"解绑当前图层":"绑定当前图层";
        if(button.textContent!==caption)button.textContent=caption;
        const background=bound?"#f2c6c6":"#c8e6c9",foreground=bound?"#682727":"#234d2b",border=bound?"2px solid #dfa4a4":"2px solid #9dcca1";
        if(button.style.backgroundColor!==background)button.style.backgroundColor=background;
        if(button.style.color!==foreground)button.style.color=foreground;
        if(button.style.border!==border)button.style.border=border;
        if(button.style.borderRadius!=="14px")button.style.borderRadius="14px";
        if(button.disabled!==!current)button.disabled=!current;return bound;
    };
    if(!autoWrapDocument)return show("自动回落：尚未绑定图层");
    let bound=docs.find(d=>d.id===autoWrapDocument.id);
    if(!bound){
        autoWrapPending=null;wrapObserved=null;
        const matches=autoWrapDocument.path?docs.filter(d=>wrapDocumentPath(d)===autoWrapDocument.path):[];
        if(matches.length===1){
            const reopened=matches[0];
            try{atlasSpec(reopened);}catch(error){show(`${autoWrapDocument.title}（网格不匹配，暂停）`);setStatus("wrap-status",`同一文件已重开，但网格校验失败：${error.message||error}`,true);return false;}
            cache.delete(autoWrapDocument.id);lastKey="";
            autoWrapDocument={...autoWrapDocument,id:reopened.id,title:reopened.title||String(reopened.id)};
            bound=reopened;
            if(doc&&doc.id===reopened.id)wrapObserved=wrapObservation();
            setStatus("wrap-status","同一文件已重开，仅原图层ID继续生效；新图层需重新绑定。");
        }else{
            show(`自动回落对象：${autoWrapDocument.title}${matches.length>1?"（同一路径有多个文档，请手动绑定）":autoWrapDocument.path?"（已关闭，等待同一文件重开）":"（已关闭；未保存文档需重新绑定图层）"}`);
            return false;
        }
    }
    autoWrapDocument.path=wrapDocumentPath(bound)||autoWrapDocument.path;
    autoWrapDocument.title=bound.title||autoWrapDocument.title;
    const selectedBound=Boolean(current&&isWrapLayerBound(doc.id,current.id));
    if(selectedBound)autoWrapDocument.layers.set(current.id,current.name||String(current.id));
    const state=!doc||doc.id!==autoWrapDocument.id?"其他文档，暂停":!current?"请仅选中一个图层":selectedBound?`${current.name||current.id}（已绑定）`:`${current.name||current.id}（未绑定，暂停）`;
    return show(`${autoWrapDocument.title} · 已绑定 ${autoWrapDocument.layers.size} 个图层\n当前：${state}`,selectedBound);
}
function observeWrapBaseline(){try{wrapObserved=wrapObservation();}catch(error){wrapObserved=null;}}
function queueAutoWrap(state,source,delay){
    const previous=autoWrapPending;
    const firstHistory=previous&&previous.documentID===state.documentID&&previous.layerID===state.layerID?previous.beforeHistoryID:undefined;
    autoWrapPending={...state,selectedSlot:selected,beforeHistoryID:firstHistory===undefined?state.beforeHistoryID:firstHistory,at:previous&&previous.documentID===state.documentID&&previous.layerID===state.layerID?previous.at:Date.now(),delay,source};
    setStatus("wrap-status",`已检测到${source}：${state.name||state.historyID}；等待至少${delay}ms稳定后自动回落。`);
}
function onPaint(){
    if(operationBusy||busy||!autoWrapEnabled||mode()!==1)return;
    try{if(!updateWrapBinding()){autoWrapPending=null;wrapObserved=null;return;}const state=wrapObservation();if(!state)return;const previous=wrapObserved;wrapObserved=state;queueAutoWrap({...state,beforeHistoryID:previous&&previous.documentID===state.documentID&&previous.layerID===state.layerID&&previous.historyID!==state.historyID?previous.historyID:undefined},"paint绘画事件",200);}catch(error){setStatus("wrap-status",`绘画事件读取失败：${error.message||error}`,true);}
}
function pollAutoWrap(){
    if(operationBusy||busy||!autoWrapEnabled||mode()!==1)return;
    try{
        if(!updateWrapBinding()){autoWrapPending=null;wrapObserved=null;return;}
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
    autoWrapPending=null;if(!autoWrapEnabled){stopWrapMonitor();wrapObserved=null;updateWrapBinding();setStatus("wrap-status","自动回落已暂停，图层绑定保留；仍可手动执行。");return;}
    try{
        if(bindCurrent){
            const {doc,layer}=wrapTarget();
            if(!autoWrapDocument||autoWrapDocument.id!==doc.id)autoWrapDocument={id:doc.id,title:doc.title||String(doc.id),path:wrapDocumentPath(doc),layers:new Map()};
            autoWrapDocument.layers.set(layer.id,layer.name||String(layer.id));
            const emptyBaselines=enableAutoWrap.emptyBaselines||(enableAutoWrap.emptyBaselines=new Map());
            const baselineKey=`${doc.id}:${layer.id}`,bounds=layer.boundsNoEffects;
            emptyBaselines.delete(baselineKey);
            if(bounds&&Number(bounds.right)===Number(bounds.left)&&Number(bounds.bottom)===Number(bounds.top))emptyBaselines.set(baselineKey,doc.activeHistoryState.id);
        }else if(!autoWrapDocument||!autoWrapDocument.layers.size)throw new Error("请先点击“绑定当前图层”，可累计绑定多个绘画层。");
        wrapObserved=null;autoWrapRuns=0;observeWrapBaseline();updateWrapBinding();
        setStatus("wrap-last-run","本次绑定尚未执行自动检查，等待新笔触。");
        startWrapMonitor();
        let eventError="";
        if(!paintListenerRegistered){try{await ps.action.addNotificationListener(["paint"],onPaint);paintListenerRegistered=true;}catch(error){eventError="；paint监听不可用，使用历史检测";}}
        setStatus("operation-status","");
        setStatus("wrap-status","自动回落已就绪：选中已绑定层后绘画"+eventError+"。");
    }catch(error){stopWrapMonitor();wrapObserved=null;autoWrapEnabled=false;updateWrapBinding();setStatus("operation-status",error.message||String(error),true);setStatus("wrap-status",`无法开启：${error.message||error}`,true);}
}
async function toggleWrapLayerBinding(){
    updateWrapBinding();
    const doc=documents().length?ps.app.activeDocument:null,layer=doc&&doc.activeLayers&&doc.activeLayers.length===1?doc.activeLayers[0]:null;
    if(!layer){setStatus("wrap-status","请仅选中一个绘画图层。",true);return;}
    if(isWrapLayerBound(doc.id,layer.id)){
        autoWrapDocument.layers.delete(layer.id);autoWrapPending=null;wrapObserved=null;
        if(!autoWrapDocument.layers.size){autoWrapEnabled=false;stopWrapMonitor();}
        updateWrapBinding();setStatus("wrap-status",`已解绑 ${layer.name||layer.id}；其他图层绑定保留。`);return;
    }
    if(operationBusy||busy){setStatus("wrap-status","正在处理画面，请稍后再绑定。");return;}
    autoWrapEnabled=true;await enableAutoWrap(true);
}
function tryAutoWrap(){
    if(!autoWrapPending||operationBusy||busy)return;
    if(!autoWrapEnabled||mode()!==1){autoWrapPending=null;return;}
    if(!updateWrapBinding()||!autoWrapPending){autoWrapPending=null;return;}
    if(Date.now()-autoWrapPending.at<(autoWrapPending.delay||200))return;
    const expected=autoWrapPending;autoWrapPending=null;operation(()=>reportedWrap(expected));
}
async function reportedWrap(expected=null){
    try{
        const count=await wrapCurrentLayer(expected);
        if(expected&&autoWrapEnabled&&isWrapLayerBound(expected.documentID,expected.layerID)){
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
        if(expected&&error.number===9&&autoWrapEnabled&&isWrapLayerBound(expected.documentID,expected.layerID)){
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
function gridExpansion(doc){
    if(gridPaths(doc).length)throw new Error("此文档使用已停止试用的路径网格，暂不支持扩展；请使用辅助图层方式新建文档。");
    const guides=gridLayers(doc);
    if(guides.length!==1)throw new Error("扩展需要文档中保留一个原辅助网格。");
    const old=guides[0],{columns,rows}=dimensions(),tw=number("width",2,1024),th=number("height",2,1024);
    if(tw!==old.tw||th!==old.th)throw new Error(`扩展时请保持单格尺寸 ${old.tw}×${old.th}，只增加行列。`);
    if(columns<old.columns||rows<old.rows)throw new Error("扩展不能减少原行列数，请增加行列后再试。");
    if(!Array.from(doc.layers).some(layer=>layer.id===old.layer.id))throw new Error("请将辅助网格放在文档顶层后再扩展。");
    const oldWidth=(old.columns+old.rows)*tw/2,oldHeight=(old.columns+old.rows)*th/2;
    const width=(columns+rows)*tw/2,height=(columns+rows)*th/2;
    if(width*height>16*1024*1024)throw new Error("Tile区域超过16777216像素，请减少行列或单格尺寸。");
    const dw=Number(doc.width),dh=Number(doc.height);
    if(dw<oldWidth||dh<oldHeight)throw new Error("当前画布已小于原网格，请先恢复画布尺寸再扩展。");
    // Preserve existing margins and all layer pixels. Add rows on the left,
    // columns on the right and both on the bottom, keeping old slot coordinates.
    const leftGrowth=(rows-old.rows)*tw/2;
    return {guide:old.layer,columns,rows,tw,th,width,height,left:Math.floor((dw-oldWidth)/2),top:Math.floor((dh-oldHeight)/2),
        canvasWidth:dw+width-oldWidth,canvasHeight:dh+height-oldHeight,leftGrowth,changed:width!==oldWidth||height!==oldHeight};
}
async function expandGridDocument(){
    if(mode()!==1||!ps.app.documents.length)throw new Error("请先打开要扩展的单文档网格。");
    const doc=ps.app.activeDocument,spec=gridExpansion(doc);
    if(!spec.changed){setStatus("operation-status","行列数未增加，无需扩展。");return;}
    autoWrapPending=null;wrapObserved=null;
    let phase="准备扩展";
    try{await ps.core.executeAsModal(async context=>{
        if(!ps.app.documents.length||ps.app.activeDocument.id!==doc.id)throw new Error("活动文档已改变，请重新点击扩展。");
        let history=await context.hostControl.suspendHistory({documentID:doc.id,name:"扩展 Tile 文档"}),pixels;
        try{
            const activeIDs=Array.from(doc.activeLayers||[],layer=>layer.id),guide=spec.guide;
            const partialLocks={pixelsLocked:guide.pixelsLocked,positionLocked:guide.positionLocked,transparentPixelsLocked:guide.transparentPixelsLocked};
            // Canvas commands may inherit the selected layer as their target.
            // Use only our unlocked guide, never unlock the user's drawing layers.
            phase="选择辅助网格";
            const selected=await ps.action.batchPlay([{_obj:"select",_target:[{_ref:"layer",_id:guide.id}],makeVisible:false,_options:{dialogOptions:"dontDisplay"}}],{});
            if(selected&&selected.some(result=>result._obj==="error"))throw new Error(selected.find(result=>result._obj==="error").message||"无法选择辅助网格");
            phase="解锁辅助网格";
            guide.allLocked=false;
            for(const key of Object.keys(partialLocks))if(partialLocks[key])guide[key]=false;
            phase="扩大画布";
            if(spec.leftGrowth)await doc.resizeCanvas(Number(doc.width)+spec.leftGrowth,Number(doc.height),ps.constants.AnchorPosition.TOPRIGHT);
            if(Number(doc.width)!==spec.canvasWidth||Number(doc.height)!==spec.canvasHeight)await doc.resizeCanvas(spec.canvasWidth,spec.canvasHeight,ps.constants.AnchorPosition.TOPLEFT);
            if(Number(doc.width)!==spec.canvasWidth||Number(doc.height)!==spec.canvasHeight)throw new Error(`画布仍为 ${doc.width}×${doc.height}，需要 ${spec.canvasWidth}×${spec.canvasHeight}`);
            if(context.isCancelled)throw new Error("用户取消扩展");
            phase="更新辅助网格";
            // Re-resolve the layer after canvas changes and explicitly clear every pixel lock.
            const writeGuide=Array.from(doc.layers).find(layer=>layer.id===guide.id);
            if(!writeGuide)throw new Error("扩展后未找到辅助网格");
            writeGuide.allLocked=false;
            writeGuide.pixelsLocked=false;
            writeGuide.positionLocked=false;
            writeGuide.transparentPixelsLocked=false;
            if(writeGuide.allLocked||writeGuide.pixelsLocked||writeGuide.positionLocked||writeGuide.transparentPixelsLocked)throw new Error("辅助网格仍受保护，已取消扩展");
            pixels=await ps.imaging.createImageDataFromBuffer(gridRaster(spec.width,spec.height,spec),{width:spec.width,height:spec.height,components:4,colorSpace:"RGB",colorProfile:"sRGB IEC61966-2.1"});
            await ps.imaging.putPixels({documentID:doc.id,layerID:guide.id,imageData:pixels,replace:true,targetBounds:{left:spec.left,top:spec.top}});
            guide.name=`辅助网格 [UYTile] ${spec.columns}x${spec.rows} ${spec.tw}x${spec.th}`;
            guide.blendMode=ps.constants.BlendMode.MULTIPLY;
            if(doc.layers[0].id!==guide.id)guide.move(doc.layers[0],ps.constants.ElementPlacement.PLACEBEFORE);
            for(const key of Object.keys(partialLocks))if(partialLocks[key])guide[key]=true;
            guide.allLocked=true;
            phase="恢复图层选择";
            if(activeIDs.length){
                const results=await ps.action.batchPlay(activeIDs.map((id,i)=>({_obj:"select",_target:[{_ref:"layer",_id:id}],...(i?{selectionModifier:{_enum:"selectionModifierType",_value:"addToSelection"}}:{}),makeVisible:false,_options:{dialogOptions:"dontDisplay"}})),{});
                if(results&&results.some(result=>result._obj==="error"))throw new Error("无法恢复图层选择，已取消扩展。");
            }
            if(context.isCancelled)throw new Error("用户取消扩展");
            phase="保存扩展";
            await context.hostControl.resumeHistory(history,true);history=null;
        }finally{try{if(history)await context.hostControl.resumeHistory(history,false);}finally{if(pixels)pixels.dispose();}}
    },{commandName:"扩展 Tile 文档"});}
    catch(error){throw new Error(`扩展失败（${phase}）：${error.message||error}`);}
    cache.clear();lastKey="";observeWrapBaseline();updateWrapBinding();
    setStatus("operation-status",`已扩展为 ${spec.columns}×${spec.rows}，画布 ${spec.canvasWidth}×${spec.canvasHeight}；原格位置和四周空间保留，可在PS撤销。`);
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
function cellBounds(row,col,spec){const p=atlasPoint(col,row,spec),x=p.x+(spec.left||0),y=p.y+(spec.top||0);return {left:x-spec.tw/2,top:y,right:x+spec.tw/2,bottom:y+spec.th};}
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
    if(spec.tw>1024||spec.th>1024)throw new Error("导出要求单格宽高为不超过 1024 的偶数像素");
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
function updateStaggerUI(){
    const supported=mode()===1||mode()===2,staggerMode=Number($("stagger-mode").selectedIndex)||0;
    $("preview-arrangement").style.display=supported?"flex":"none";
    $("stagger-settings").style.display=supported&&staggerMode?"flex":"none";
    $("stagger-description").style.display=supported&&staggerMode?"block":"none";
    $("stagger-columns").disabled=staggerMode===0||staggerMode===2;
    $("stagger-rows").disabled=staggerMode===0||staggerMode===1;
    return staggerMode;
}
function staggerSettings(){
    const mode=updateStaggerUI();
    const count=id=>{const value=number(id,0,8);if(!Number.isInteger(value))throw new Error("错位格数必须是0–8的整数");return value;};
    return {mode,columns:mode===1||mode===3?count("stagger-columns"):0,rows:mode===2||mode===3?count("stagger-rows"):0};
}
function staggerSlot(i,j,columns,rows,stagger){
    const mod=(value,period)=>(value%period+period)%period;
    const col=i-(mod(Math.floor(j/rows),2)?stagger.columns:0);
    const row=j-(mod(Math.floor(i/columns),2)?stagger.rows:0);
    return {column:mod(col,columns),row:mod(row,rows)};
}
function updateArrangementUI(spec=null){
    const atlas=mode()===1,multiple=mode()===2;
    const enabled=atlas||multiple;
    if(!enabled)return;
    const stagger=staggerSettings();
    if(atlas&&$("joint-wrap").checked){
        $("group-settings").style.display="block";
        spec=spec||dimensions();
        const group=subgroupSettings(spec),total=group.across*group.down,index=group.index+1;
        $("selected-group").textContent=`当前子组 ${index}／${total}（行 ${Math.floor(group.index/group.across)+1}，列 ${group.index%group.across+1}）`;
        $("group-columns").setAttribute?.("max",String(spec.columns));$("group-rows").setAttribute?.("max",String(spec.rows));
        $("group-columns").style.borderColor=$("group-rows").style.borderColor="";
    }else if(atlas){
        $("group-settings").style.display="none";
        $("selected-group").textContent="";
    }else $("group-settings").style.display="none";
    return stagger;
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
    if(mode()!==2&&ps.app.documents.length)settingsDocumentID=ps.app.activeDocument.id;
    const atlas=mode()===1,multiple=mode()===2;
    updateStaggerUI();
    $("slot-controls").style.display=atlas||multiple?"flex":"none";
    $("grid-dimensions").style.display=atlas||multiple?"flex":"none";
    $("grid-document-actions").style.display=atlas?"flex":"none";
    $("multi-document-controls").style.display=multiple?"flex":"none";
    $("atlas-actions").style.display=atlas?"flex":"none";
    $("atlas-export-actions").style.display=atlas?"flex":"none";
    if($("joint-wrap"))$("joint-wrap").disabled=!atlas;
    $("elevated").disabled=atlas;
    $("width-label").textContent=atlas?"单格宽度":"地面宽度";$("height-label").textContent=atlas?"单格高度":"地面高度";
    const {columns,rows}=dimensions();
    $("slot-selection").style.display=(atlas||multiple)&&columns*rows>1?"flex":"none";
    for(const [id,groupColumns,groupRows]of [["group-preset-1x1",1,1],["group-preset-1x2",1,2],["group-preset-2x1",2,1],["group-preset-2x2",2,2]]){
        const preset=$(id);if(preset)preset.disabled=!atlas||columns%groupColumns!==0||rows%groupRows!==0;
    }
    selected=Math.min(Math.floor(selected/8),rows-1)*8+Math.min(selected%8,columns-1);
    const area=$("slots");
    const layout=`${columns}x${rows}`;if(slotLayout!==layout){area.innerHTML="";slotLayout=layout;}
    for(let row=0;row<rows;row++)for(let col=0;col<columns;col++){
        const index=row*8+col,binding=bindings[index];
        let button=area.children[row*columns+col];
        if(!button){button=document.createElement("button");button.addEventListener("click",()=>chooseSlot(index));area.appendChild(button);}
        button.textContent=`${row*columns+col+1}${multiple&&binding?"●":""}`;
        button.style.width=`${100/columns}%`;
        button.style.backgroundColor=selected===index?"#315e8c":"#292929";button.style.color="#fff";
        button.style.border="2px solid #ddd";button.style.borderRadius="13px";
        button.title=`Slot ${row*columns+col+1}（${row+1},${col+1}）: ${binding?binding.title:"未绑定"}`;
    }
    $("selected-slot").textContent=`当前 Slot ${Math.floor(selected/8)*columns+selected%8+1}（行 ${Math.floor(selected/8)+1}，列 ${selected%8+1}）`;
    $("grid-size").textContent=atlas?`中心 Tile：${(columns+rows)*Number($("width").value)/2}×${(columns+rows)*Number($("height").value)/2} 像素，共 ${columns*rows} 格；画布可更大`:"";
    if(multiple)documentList();
}
async function capture(doc,elevated,limit,atlas){
    const dw=atlas?atlas.width:Number(doc.width),dh=atlas?atlas.height:Number(doc.height),ox=atlas?atlas.left:0,oy=atlas?atlas.top:0,id=doc.id;
    const captureKey=()=>JSON.stringify([doc.activeHistoryState.id,dw,dh,elevated,limit,atlas]);
    const key=captureKey();
    const cached=cache.get(id);if(cached&&cached.key===key){cached.title=doc.title;cached.sourceState=documentState(doc);return cached;}
    let data;
    try{
        const scale=Math.min(1,limit/dw,limit/dh),w=Math.max(1,Math.round(dw*scale)),h=Math.max(1,Math.round(dh*scale));
        const result=await withoutGrid(doc,()=>ps.imaging.getPixels({documentID:id,sourceBounds:{left:ox,top:oy,right:ox+dw,bottom:oy+dh},targetSize:{width:w,height:h},componentSize:8,colorSpace:"RGB",colorProfile:"sRGB IEC61966-2.1",applyAlpha:false}));
        data=result.imageData;
        if(!data)throw new Error("文档暂无可读取的像素，请先画一笔再刷新");
        const raw=await data.getData({chunky:true});
        const rgba=new Uint8Array(w*h*4),bounds=result.sourceBounds,levelScale=2**(result.level||0),mask=atlas?diamondMask(w,h,atlas.columns,atlas.rows):!elevated?diamondMask(w,h):null;
        const bw=bounds.right-bounds.left,bh=bounds.bottom-bounds.top;
        if(bw>0&&bh>0){
            for(let y=0;y<h;y++)for(let x=0;x<w;x++){
                if(mask&&!mask[y*w+x])continue;
                const sx=Math.floor(((ox+(x+0.5)*dw/w)/levelScale-bounds.left)*data.width/bw);
                const sy=Math.floor(((oy+(y+0.5)*dh/h)/levelScale-bounds.top)*data.height/bh);
                if(sx<0||sy<0||sx>=data.width||sy>=data.height)continue;
                const s=(sy*data.width+sx)*data.components,d=(y*w+x)*4;
                rgba[d]=raw[s];rgba[d+1]=raw[s+1];rgba[d+2]=raw[s+2];rgba[d+3]=data.components===4?raw[s+3]:255;
            }
        }
        const value={key:captureKey(),sourceState:documentState(doc),url:"data:image/png;base64,"+base64(png(w,h,rgba)),dw,dh,w,h,title:doc.title,rgba:atlas?rgba:null};
        cache.set(id,value);return value;
    }finally{if(data)data.dispose();}
}
// Reuse mounted image nodes: clearing the panel restarts image decoding and flashes.
const previewGridCache=new Map();
function previewGridImage(width,height,columns,rows,left,top,rasterWidth=width,rasterHeight=height,color=null){
    const key=JSON.stringify([rasterWidth,rasterHeight,columns,rows,color]);
    if(!previewGridCache.has(key)){
        const scale=Math.min(1,512/Math.max(rasterWidth,rasterHeight));
        const w=Math.max(2,Math.round(rasterWidth*scale)),h=Math.max(2,Math.round(rasterHeight*scale)),k=columns+rows;
        // Use the same full-span coordinates and rasterizer as the PSD guide layer.
        // Sampling at source resolution keeps the overlay anchored when zoom changes.
        const rgba=gridRaster(w,h,{columns,rows,tw:2*w/k,th:2*h/k},true);
        if(color){const tint=hexToRgba(color);for(let i=0;i<rgba.length;i+=4)if(rgba[i+3])rgba.set(tint,i);}
        if(previewGridCache.size>=32)previewGridCache.delete(previewGridCache.keys().next().value);
        previewGridCache.set(key,"data:image/png;base64,"+base64(png(w,h,rgba)));
    }
    const img=document.createElement("img");img.className="preview-grid";img.src=previewGridCache.get(key);
    Object.assign(img.style,{left:`${left}px`,top:`${top}px`,width:`${width}px`,height:`${height}px`,pointerEvents:"none"});return img;
}
function subgroupGridImage(spec,group,left,top,zoom){
    const displayWidth=(spec.columns+spec.rows)*spec.tw/2,displayHeight=(spec.columns+spec.rows)*spec.th/2,scale=Math.min(1,512/Math.max(displayWidth,displayHeight));
    spec={...spec,tw:spec.tw*scale,th:spec.th*scale};
    const width=Math.round((spec.columns+spec.rows)*spec.tw/2),height=Math.round((spec.columns+spec.rows)*spec.th/2),key=JSON.stringify([width,height,spec.columns,spec.rows,spec.tw,spec.th,group.columns,group.rows,group.index]);
    let url=subgroupGridCache.get(key);
    if(!url){
        const out=new Uint8Array(width*height*4),gw=Math.round((group.columns+group.rows)*spec.tw/2),gh=Math.round((group.columns+group.rows)*spec.th/2);
        const local={columns:group.columns,rows:group.rows,tw:spec.tw,th:spec.th};
        for(let row=0;row<group.down;row++)for(let col=0;col<group.across;col++){
            const groupIndex=row*group.across+col,color=groupIndex===group.index?[255,216,70,240]:hexToRgba(groupColors[groupIndex%groupColors.length]);
            const raster=gridRaster(gw,gh,local,true),origin=atlasPoint(col*group.columns,row*group.rows,spec),gx=Math.round(origin.x-group.rows*spec.tw/2),gy=Math.round(origin.y);
            for(let y=0;y<gh;y++)for(let x=0;x<gw;x++){
                const si=(y*gw+x)*4;if(!raster[si+3])continue;
                const dx=gx+x,dy=gy+y;if(dx<0||dy<0||dx>=width||dy>=height)continue;
                const di=(dy*width+dx)*4;out.set(color,di);
            }
        }
        url="data:image/png;base64,"+base64(png(width,height,out));
        if(subgroupGridCache.size>=8)subgroupGridCache.delete(subgroupGridCache.keys().next().value);
        subgroupGridCache.set(key,url);
    }
    const image=document.createElement("img");image.className="preview-grid subgroup-grid";image.src=url;
    Object.assign(image.style,{left:`${left}px`,top:`${top}px`,width:`${displayWidth*zoom}px`,height:`${displayHeight*zoom}px`,pointerEvents:"none"});return image;
}
function hexToRgba(hex){const value=String(hex).replace("#","");return [parseInt(value.slice(0,2),16),parseInt(value.slice(2,4),16),parseInt(value.slice(4,6),16),220];}
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
function render(frames,view,fw,fh,zoom,elevated,multiple,columns,stagger={columns:0,rows:0},group=null){
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
        const groupIndex=group?subgroupIndex(slot.column,slot.row,{columns},group):-1;
        const groupColor=group?(groupIndex===group.index?"#ffd846":groupColors[groupIndex%groupColors.length]):null;
        if($("show-grid").checked)grids.push(previewGridImage(sw,sh,1,1,view.width/2+(col+((row%2+2)%2)/2)*sw-sw/2,view.height/2+row*sh/2-sh/2,groundW,groundH,groupColor));
        if(multiple&&frames.length>1&&$("show-slots").checked){
            const r=slot.row,c=slot.column;
            const marker=document.createElement("div");marker.className="slot-marker";
            marker.textContent=String(r*columns+c+1);
            marker.title=`Slot ${r*columns+c+1}: ${frame?frame.title:"未绑定或文档已关闭"}`;
            marker.style.left=`${view.width/2+(col+((row%2+2)%2)/2)*sw-12}px`;
            marker.style.top=`${view.height/2+row*sh/2-8}px`;
            if(selected===r*8+c){marker.style.backgroundColor="#276ca8";marker.style.borderColor="#ffdf50";}
            marker.addEventListener("click",()=>chooseSlot(r*8+c));markers.push(marker);
        }
    }
    for(const grid of grids)scene.push(grid);
    for(const marker of markers)scene.push(marker);
    commitPreview(scene);
}
function atlasTileFrames(frame,spec){
    if(frame.tiles)return frame.tiles;
    const w=Math.max(2,Math.round(spec.tw*frame.w/frame.dw)),h=Math.max(2,Math.round(spec.th*frame.h/frame.dh)),mask=diamondMask(w,h),tiles=[];
    for(let r=0;r<spec.rows;r++)for(let c=0;c<spec.columns;c++){
        const tip=atlasPoint(c,r,spec),left=tip.x-spec.tw/2,rgba=new Uint8Array(w*h*4);
        for(let y=0;y<h;y++)for(let x=0;x<w;x++)if(mask[y*w+x]){
            const sx=Math.floor((left+(x+.5)*spec.tw/w)*frame.w/frame.dw),sy=Math.floor((tip.y+(y+.5)*spec.th/h)*frame.h/frame.dh);
            if(sx>=0&&sx<frame.w&&sy>=0&&sy<frame.h)rgba.set(frame.rgba.subarray((sy*frame.w+sx)*4,(sy*frame.w+sx)*4+4),(y*w+x)*4);
        }
        tiles.push({dw:spec.tw,dh:spec.th,w,h,title:`${frame.title} · ${r+1},${c+1}`,url:"data:image/png;base64,"+base64(png(w,h,rgba))});
    }
    frame.tiles=tiles;return tiles;
}
function renderAtlas(frame,view,spec,zoom,stagger={columns:0,rows:0}){
    const panel=$("preview"),scene=[];if(!frame||!spec){commitPreview(scene);return;}
    const subgroup=$("joint-wrap").checked?subgroupSettings(spec):null;
    // Keep the ground lattice fully covered even when both axes are offset.
    // Reuse the multi-Slot lookup over cached crops; never shift overlapping atlases.
    if(stagger.columns||stagger.rows){render(atlasTileFrames(frame,spec),view,spec.tw,spec.th,zoom,false,true,spec.columns,stagger,subgroup);return;}
    const {columns,rows,tw,th}=spec,z=zoom*Math.max(1,24/(Math.min(tw,th)*zoom));
    const iw=frame.dw*z,ih=frame.dh*z,markers=[],grids=[];
    const corners=[];for(const x of [-view.width/2/z-frame.dw/2,view.width/2/z+frame.dw/2])for(const y of [-view.height/2/z-frame.dh/2,view.height/2/z+frame.dh/2])corners.push({m:(x/tw+y/th)/columns,n:(y/th-x/tw)/rows});
    const padM=1+Math.ceil(stagger.rows/columns),padN=1+Math.ceil(stagger.columns/rows);
    const m0=Math.max(-32,Math.floor(Math.min(...corners.map(c=>c.m)))-padM),m1=Math.min(32,Math.ceil(Math.max(...corners.map(c=>c.m)))+padM);
    const n0=Math.max(-32,Math.floor(Math.min(...corners.map(c=>c.n)))-padN),n1=Math.min(32,Math.ceil(Math.max(...corners.map(c=>c.n)))+padN);
    const selectedGroup=subgroup;
    for(let m=m0;m<=m1;m++)for(let n=n0;n<=n1;n++){
        const offsetColumns=(n%2!==0)?stagger.columns:0,offsetRows=(m%2!==0)?stagger.rows:0;
        const left=view.width/2-iw/2+(m*columns+offsetColumns-n*rows-offsetRows)*tw*z/2;
        const top=view.height/2-ih/2+(m*columns+offsetColumns+n*rows+offsetRows)*th*z/2;
        if(left+iw<0||top+ih<0||left>view.width||top>view.height)continue;
        const img=document.createElement("img");img.src=frame.url;img.style.left=`${left}px`;img.style.top=`${top}px`;img.style.width=`${iw}px`;img.style.height=`${ih}px`;scene.push(img);
        if($("show-grid").checked){
            if(selectedGroup)grids.push(subgroupGridImage(spec,selectedGroup,left,top,z));
            else grids.push(previewGridImage(iw,ih,columns,rows,left,top,frame.w,frame.h));
        }
        if(columns*rows>1&&$("show-slots").checked)for(let r=0;r<rows;r++)for(let c=0;c<columns;c++){
            const p=atlasPoint(c+.5,r+.5,spec),x=left+p.x*z,y=top+p.y*z;if(x<0||y<0||x>view.width||y>view.height)continue;
            const marker=document.createElement("div");marker.className="slot-marker";marker.textContent=String(r*columns+c+1);
            marker.title=`源文档第 ${r+1} 行、第 ${c+1} 列`;marker.style.left=`${x-12}px`;marker.style.top=`${y-8}px`;
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
        reconcileControls();
        previewBackground();
        const docs=documents(),multiple=mode()===2,atlas=mode()===1,elevated=!atlas&&$("elevated").checked;
        if(multiple)documentList();
        if(!docs.length)return;
        const {columns}=dimensions();
        const slots=multiple?activeBindings():[docs.length?{id:ps.app.activeDocument.id}:null];
        const sources=slots.map(binding=>binding?docs.find(doc=>doc.id===binding.id):null);
        const view=viewport(),fw=number("width",2,8192),fh=number("height",2,8192),zoom=number("zoom",0.1,10);
        const spec=atlas&&sources[0]?atlasSpec(sources[0]):null,stagger=(atlas||multiple)?updateArrangementUI(spec)||{mode:0,columns:0,rows:0}:{mode:0,columns:0,rows:0};
        const wrapConfig=atlas?[$("joint-wrap").checked,$("group-columns").value,$("group-rows").value]:null;
        const renderKey=states=>JSON.stringify([states,slots,multiple,elevated,columns,fw,fh,zoom,view,selected,$("show-slots").checked,$("show-grid").checked,spec,stagger,wrapConfig]);
        const key=renderKey(sources.map(documentState));
        if(!force&&key===lastKey)return;
        const used=new Set(sources.filter(Boolean).map(doc=>doc.id));
        for(const id of cache.keys())if(!used.has(id))cache.delete(id);
        if(!used.size){render(sources.map(()=>null),view,fw,fh,zoom,elevated,multiple,columns,stagger);setStatus("status",multiple?"请选择 Slot，再从下拉菜单绑定打开的文档；关闭的文档需重新绑定。":noDocumentMessage);lastKey=key;return;}
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
        if(!updateDocumentAvailability())return;
        const currentIds=new Set(documents().map(doc=>doc.id));
        const frames=sources.map(doc=>doc&&currentIds.has(doc.id)?results.get(doc.id):null);
        if(!failures.length){if(atlas)renderAtlas(frames[0],view,spec,zoom,stagger);else render(frames,view,fw,fh,zoom,elevated,multiple,columns,stagger);}
        const missing=slots.filter((binding,index)=>binding&&!sources[index]).length;
        const first=frames.find(Boolean);
        setStatus("status",multiple?`已显示 ${frames.filter(Boolean).length}/${slots.length} 个 Slot · ${used.size} 个文档 · 采样上限 ${limit}px${missing?`；${missing} 个文档已关闭，请重新绑定`:""}`:(first?`${first.title} · ${first.dw}×${first.dh} · 预览采样 ${first.w}×${first.h}`:"预览未更新"));
        if(atlas&&spec)$("status").textContent+=` · 菱形网格 ${spec.columns}×${spec.rows}`;
        if((atlas||multiple)&&stagger.mode)$("status").textContent+=` · 错位预览（列${stagger.columns}／行${stagger.rows}）`;
        if(failures.length)setStatus("status",$("status").textContent+"；"+failures.join("；"),true);
        if(!failures.length)lastKey=renderKey(sources.map(doc=>doc?results.get(doc.id).sourceState:null));
    }catch(error){setStatus("status",`预览未更新：${error.message||error}`,true);}
    finally{busy=false;if(pending&&visible){pending=false;refresh(true);}}
}
function startWrapMonitor(){if(wrapTimer)return;wrapTimer=setInterval(()=>{pollAutoWrap();tryAutoWrap();},300);}
function stopWrapMonitor(){if(wrapTimer)clearInterval(wrapTimer);wrapTimer=null;autoWrapPending=null;}
function updatePanelControls(){
    try{reconcileControls();}catch(error){setStatus("operation-status",`设置未更新：${error.message||error}`,true);}
    try{updateWrapBinding();}catch(error){setStatus("wrap-status",`绑定状态暂不可用：${error.message||error}`,true);}
}
function start(showPreview=true){
    if(visible!==showPreview){visible=showPreview;generation++;lastKey="";pending=false;}
    updatePanelControls();
    if(!timer)timer=setInterval(()=>{if(operationBusy)return;updatePanelControls();if(!visible||autoWrapPending)return;refresh();},300);
    if(visible)refresh(true);
}
function stop(){visible=false;generation++;pending=false;if(timer)clearInterval(timer);timer=null;}
function destroyPanel(){stop();stopWrapMonitor();autoWrapDocument=null;wrapObserved=null;autoWrapEnabled=false;if(paintListenerRegistered){ps.action.removeNotificationListener(["paint"],onPaint).catch(()=>{});paintListenerRegistered=false;}}
function syncPanels(){
    for(const panel of Object.values(panels)){
        const content=$(panel.content);
        if(panel.root&&content.parentNode!==panel.root){
            Object.assign(panel.root.style,{display:"flex",flexDirection:"column",width:"100%",height:"100%",overflow:"hidden"});
            panel.root.appendChild(content);
        }
        content.style.display=panel.shown?"flex":"none";
    }
    $("open-preview").textContent=panels.tilePreviewView.shown?"显示预览":"打开预览";
    if(panels.tilePreview.shown||panels.tilePreviewView.shown)start(panels.tilePreviewView.shown);else stop();
}
let openingPanel=false;
async function showOwnPanel(id){
    const manager=require("uxp").pluginManager;
    const plugin=manager&&Array.from(manager.plugins).find(item=>item.id===require("./manifest.json").id);
    if(!plugin||typeof plugin.showPanel!=="function")throw new Error("请从增效工具 → Isometric Tile Preview 菜单打开面板");
    await plugin.showPanel(id);
}
async function openPreviewPanel(){
    if(openingPanel)return false;
    openingPanel=true;$("open-preview").disabled=true;
    try{await showOwnPanel("tilePreviewView");setStatus("preview-panel-status","");return true;}
    catch(error){setStatus("preview-panel-status",`无法打开预览：${error.message||error}`,true);return false;}
    finally{openingPanel=false;$("open-preview").disabled=false;}
}
function request(forcePixels=false){lastKey="";if(forcePixels)cache.clear();if(visible)refresh(true);}
$("refresh").addEventListener("click",async()=>{
    cache.clear();lastKey="";updatePanelControls();
    if(!validateGridSettings())return;
    if(panels.tilePreviewView.shown)request();else await openPreviewPanel();
});
$("open-preview").addEventListener("click",openPreviewPanel);
$("clear-errors").addEventListener("click",()=>{
    const errors=setStatus.errors||(setStatus.errors=new Map());
    const dismissed=setStatus.dismissed||(setStatus.dismissed=new Map());
    for(const [id,text] of errors){dismissed.set(id,text);const node=$(id);node.textContent="";node.style.display="none";}
    errors.clear();if(setStatus.info)setStatus.info.clear();$("panel-error").textContent="";
});
$("open-settings").addEventListener("click",async()=>{
    try{await showOwnPanel("tilePreview");}catch(error){setStatus("status",`无法打开设置：${error.message||error}`,true);}
});
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
function releasePanelFocus(root,event){
    const active=document.activeElement;
    if(!active||typeof active.blur!=="function")return;
    if(root.contains&&!root.contains(active))return;
    const tag=String(active.tagName||"").toLowerCase();
    const editing=tag==="input"&&!["checkbox","range","button","hidden"].includes(String(active.type||"").toLowerCase());
    if(event&&event.type==="click"&&editing)return;
    active.blur();
}
for(const id of ["controls","view-area"]){
    const root=$(id);
    root.addEventListener("mouseleave",event=>releasePanelFocus(root,event));
    root.addEventListener("click",event=>{
        const target=event.target,tag=String(target&&target.tagName||"").toLowerCase();
        if(tag==="button")releasePanelFocus(root,event);
    });
    root.addEventListener("keydown",event=>{
        if(event.key==="Escape"||event.key==="Enter")releasePanelFocus(root,event);
    });
}
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
for(const id of ["width","height","elevated","show-slots","show-grid","joint-wrap","group-columns","group-rows"])$(id).addEventListener("change",()=>{slotUI();try{request();}catch(e){setStatus("status",e.message,true);}});
for(const [id,columns,rows]of [["group-preset-1x1",1,1],["group-preset-1x2",1,2],["group-preset-2x1",2,1],["group-preset-2x2",2,2]])$(id).addEventListener("click",()=>{
    try{const overall=dimensions();if(overall.columns%columns||overall.rows%rows)throw new Error(`当前整体 ${overall.columns}×${overall.rows} 不能整除 ${columns}×${rows} 子组`);
        $("group-columns").value=String(columns);$("group-rows").value=String(rows);slotUI();request();
    }catch(error){setStatus("status",error.message||String(error),true);}
});
for(const id of ["stagger-mode","stagger-columns","stagger-rows"])$(id).addEventListener("change",()=>{updateStaggerUI();request();});
for(const id of ["columns","rows"])$(id).addEventListener("change",()=>{try{slotUI();request();}catch(e){setStatus("status",e.message,true);}});
for(const size of [2,3,4])$("preset-"+size).addEventListener("click",()=>{$("columns").value=String(size);$("rows").value=String(size);slotUI();request();});
$("mode").addEventListener("change",()=>{slotUI();request();});
$("document-settings-toggle").addEventListener("click",()=>{
    documentSettingsOpen=!documentSettingsOpen;
    $("document-settings").style.display=documentSettingsOpen?"flex":"none";
    $("document-settings-toggle").textContent=(documentSettingsOpen?"▾":"▸")+" 文档设置(可新建文档)";
    if($("document-settings-toggle").setAttribute)$("document-settings-toggle").setAttribute("aria-expanded",String(documentSettingsOpen));
});
let tileSlotSettingsOpen=false;
$("tile-slot-settings-toggle").addEventListener("click",()=>{
    tileSlotSettingsOpen=!tileSlotSettingsOpen;
    $("tile-slot-settings").style.display=tileSlotSettingsOpen?"flex":"none";
    $("tile-slot-settings-toggle").textContent=(tileSlotSettingsOpen?"▾":"▸")+" TileSlot设置与导出";
    if($("tile-slot-settings-toggle").setAttribute)$("tile-slot-settings-toggle").setAttribute("aria-expanded",String(tileSlotSettingsOpen));
});
$("new-grid-document").addEventListener("click",()=>operation(newGridDocument));
$("expand-grid-document").addEventListener("click",()=>operation(expandGridDocument));
$("wrap-layer").addEventListener("click",()=>operation(()=>reportedWrap()));
$("bind-auto-wrap").addEventListener("click",toggleWrapLayerBinding);
$("convert-single-document").addEventListener("click",()=>operation(convertToSingleDocument));
$("export-cell").addEventListener("click",()=>operation(()=>exportGrid(false)));
$("export-grid").addEventListener("click",()=>operation(()=>exportGrid(true)));
$("reload-documents").addEventListener("click",()=>{documentList(true);request();});
$("document-picker").addEventListener("change",()=>{if(mode()!==2)return;const index=$("document-picker").selectedIndex;if(index<0||index>=documentOptions.length)return;const binding=documentOptions[index];if((bindings[selected]?.id??null)===(binding?.id??null))return;bindings[selected]=binding;slotUI();request();});
$("bind").addEventListener("click",()=>{if(!ps.app.documents.length){setStatus("status","请先打开要绑定的 PSD 文档。");return;}const doc=ps.app.activeDocument;bindings[selected]={id:doc.id,title:doc.title};slotUI();request();});
$("unbind").addEventListener("click",()=>{bindings[selected]=null;slotUI();request();});
$("mode").selectedIndex=selectedMode;
panelReady=true;
syncPanels();
