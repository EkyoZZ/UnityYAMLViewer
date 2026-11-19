const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const source=fs.readFileSync(__dirname+'/main.js','utf8'),host={diamondMasks:new Map()};
vm.runInNewContext(source.slice(source.indexOf('function atlasUV('),source.indexOf('function gridLayers(')),host);
vm.runInNewContext(source.slice(source.indexOf('function gridRaster('),source.indexOf('function unpackLayerPixels(')),host);
const get=(a,w,x,y)=>Array.from(a.subarray((y*w+x)*4,(y*w+x)*4+4));
for(const group of [null,{columns:1,rows:1,u:0,v:0},{columns:2,rows:1,u:0,v:0},{columns:1,rows:2,u:0,v:0}]){
    const spec={columns:2,rows:2,tw:16,th:8,...(group?{group}:{})},tw=32,th=16,w=96,h=64,ox=32,oy=24,mask=host.diamondMask(tw,th,2,2);
    const before=new Uint8Array(w*h*4),localBefore=new Uint8Array(tw*th*4);
    for(let y=0;y<th;y++)for(let x=0;x<tw;x++)if(mask[y*tw+x]){
        before.set([20,130,40,255],((y+oy)*w+x+ox)*4);localBefore.set([20,130,40,255],(y*tw+x)*4);
    }
    const raw=new Uint8Array(before),local=new Uint8Array(localBefore);
    for(let y=10;y<=14;y++)for(let x=7;x<=10;x++){
        raw.set([230,30,50,170],((y+oy)*w+x+ox)*4);local.set([230,30,50,170],(y*tw+x)*4);
    }
    const expected=host.wrapPixels(local,tw,th,0,0,spec,localBefore);
    const result=host.wrapPixels(raw,w,h,0,0,{...spec,left:ox,top:oy,workspace:true},before);
    for(let y=0;y<h;y++)for(let x=0;x<w;x++){
        const inside=x>=ox&&x<ox+tw&&y>=oy&&y<oy+th;
        assert.deepEqual(get(result.rgba,w,x,y),inside?get(expected.rgba,tw,x-ox,y-oy):[0,0,0,0],`center mapping ${x},${y}`);
    }
    assert.equal(result.moved,expected.moved);assert.equal(result.copied,expected.copied);
    if(group)assert.throws(()=>host.wrapPixels(raw,w,h,0,0,{...spec,left:ox,top:oy,workspace:true}),/笔触前/);
    else{
        // Ordinary current-layer wrapping remains usable without previous history.
        const actual=host.wrapPixels(raw,w,h,0,0,{...spec,left:ox,top:oy,workspace:true});
        const localOnly=host.wrapPixels(local,tw,th,0,0,spec);
        for(let y=0;y<th;y++)for(let x=0;x<tw;x++)assert.deepEqual(get(actual.rgba,w,x+ox,y+oy),get(localOnly.rgba,tw,x,y));
    }
}
console.log('Workspace checks passed: centered normal/subgroup wrap matches the original tile region, transparent margins, and unchanged history requirements.');
