const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const source=fs.readFileSync(__dirname+'/main.js','utf8');
const host={diamondMasks:new Map()};
vm.runInNewContext(source.slice(source.indexOf('function atlasUV('),source.indexOf('function gridLayers(')),host);
vm.runInNewContext(source.slice(source.indexOf('function gridRaster('),source.indexOf('function unpackLayerPixels(')),host);
const color=[230,40,20,170],pixel=(bytes,w,x,y)=>Array.from(bytes.subarray((y*w+x)*4,(y*w+x)*4+4));
for(const [gc,gr]of [[1,1],[1,2],[2,1],[2,2]])for(const [u,v]of [[0,0],[2-gc,4-gr]]){
    const spec={columns:2,rows:4,tw:16,th:8,group:{columns:gc,rows:gr,u,v}},w=48,h=24,before=new Uint8Array(w*h*4);
    const raw=new Uint8Array(before),top=host.atlasPoint(u,v,spec);
    const ix=Math.floor(top.x+(gc-gr)*4),iy=Math.floor(top.y+(gc+gr)*2);
    raw.set(color,(iy*w+ix)*4);
    const inside=host.wrapPixels(raw,w,h,0,0,spec,before);
    assert.deepEqual(Buffer.from(inside.rgba),Buffer.from(raw),'an interior stroke must not overwrite other subgroups');
    // A stroke crosses the selected subgroup's v edge: the matching destination
    // is exactly gr * (8,-4) pixels away, in every subgroup.
    const edge=host.atlasPoint(u+gc/2,v+gr,spec),x=Math.floor(edge.x),y=Math.floor(edge.y)+1;
    if(x<0||y<0||x>=w||y>=h)continue;
    const stroke=new Uint8Array(before);stroke.set(color,(y*w+x)*4);
    const result=host.wrapPixels(stroke,w,h,0,0,spec,before);
    for(let r=0;r<4;r+=gr)for(let c=0;c<2;c+=gc){
        const tx=x+gr*8+(c-u-(r-v))*8,ty=y-gr*4+(c-u+r-v)*4;
        assert.deepEqual(pixel(result.rgba,w,tx,ty),color,`linked ${gc}x${gr} group at ${c},${r}`);
    }
    assert.deepEqual(Buffer.from(host.wrapPixels(result.rgba,w,h,0,0,spec,before).rgba),Buffer.from(result.rgba),'rechecking must not accumulate alpha');
    assert.throws(()=>host.wrapPixels(stroke,w,h,0,0,spec),/笔触前/);
}
console.log('Subgroup checks passed: 1x1/1x2/2x1/2x2, selected group origins, independent interiors, linked edge offsets, alpha stability and missing-history guard.');

// A new translucent crossing stroke must not punch through an existing subgroup.
{
 const spec={columns:2,rows:4,tw:16,th:8,group:{columns:1,rows:1,u:0,v:0}},w=48,h=24;
 const before=new Uint8Array(w*h*4),mask=host.diamondMask(w,h,2,4);
 for(let y=0;y<h;y++)for(let x=0;x<w;x++)if(mask[y*w+x])before.set([20,80,180,200],(y*w+x)*4);
 const edge=host.atlasPoint(.5,1,spec),x=Math.floor(edge.x),y=Math.floor(edge.y)+1,i=(y*w+x)*4;
 const raw=new Uint8Array(before);raw.set([160,140,90,228],i);
 const result=host.wrapPixels(raw,w,h,0,0,spec,before);let changed=0;
 for(let j=0;j<raw.length;j+=4)if(j!==i&&result.rgba[j+3]!==raw[j+3]){assert(result.rgba[j+3]>=raw[j+3],'added paint cannot reduce destination alpha');changed++;}
 assert(changed>0,'crossing stroke should update another subgroup');
 const stable=host.wrapPixels(result.rgba,w,h,0,0,spec,result.rgba);assert.deepEqual(Buffer.from(stable.rgba),Buffer.from(result.rgba));
}
{
 const out=Uint8Array.from([0,0,255,128]),paint=Uint8Array.from([255,0,0,128]);
 host.compositeWrapStroke(out,0,paint,0,new Uint8Array(4));assert.equal(out[3],192);assert(out[0]>out[2]);
 const erased=Uint8Array.from([30,40,50,200]);host.compositeWrapStroke(erased,0,Uint8Array.from([1,2,3,64]),0,Uint8Array.from([1,2,3,128]));assert.equal(erased[3],100);
}
console.log('Subgroup paint-over checks passed: existing alpha retained, source-over color, erasing and repeated-run stability.');

// A soft green mark on opaque cyan must not transfer the entire cyan background.
{const target=Uint8Array.from([0,0,0,255]),old=Uint8Array.from([128,200,210,255]),next=Uint8Array.from([64,228,105,255]);host.compositeWrapStroke(target,0,next,0,old);assert(target[0]<=2&&target[2]<=2,'source background must not contaminate black target');assert(target[1]>100&&target[1]<200,'soft paint remains a partial contribution');assert.equal(target[3],255);
const stable=new Uint8Array(target);host.compositeWrapStroke(target,0,next,0,next);assert.deepEqual(Buffer.from(target),Buffer.from(stable),'unchanged color contributes no paint');}

// Distinct brush samples landing at one periodic position must both survive.
{
 const spec={columns:4,rows:4,tw:16,th:8,group:{columns:2,rows:2,u:0,v:0}},w=64,h=32,before=new Uint8Array(w*h*4),mask=host.diamondMask(w,h,4,4),classes=new Map();
 const origin=host.atlasPoint(0,0,spec),gl=origin.x-16;
 for(let y=0;y<h;y++)for(let x=0;x<w;x++)if(mask[y*w+x]){
  const u=(x+.5)/16+(y+.5)/8-2,v=(y+.5)/8-(x+.5)/16+2;
  if(u>=0&&u<2&&v>=0&&v<2)continue;
  const q=host.wrappedPixel(x-gl+.5,y+.5,{columns:2,rows:2,tw:16,th:8}),key=Math.floor(q.y)*32+Math.floor(q.x);
  if(!classes.has(key))classes.set(key,[]);classes.get(key).push((y*w+x)*4);
 }
 const pair=[...classes.values()].find(list=>list.length>=2);assert(pair);
 const raw=new Uint8Array(before);raw.set([255,0,0,128],pair[0]);raw.set([0,255,0,128],pair[1]);
 const result=host.wrapPixels(raw,w,h,0,0,spec,before);let merged=false;
 for(let i=0;i<raw.length;i+=4)if(i!==pair[0]&&i!==pair[1]&&result.rgba[i+3]===192&&result.rgba[i]>0&&result.rgba[i+1]>0)merged=true;
 assert(merged,'two mapped samples must composite, rather than choosing the last one');
}
console.log('Periodic collision regression passed: both distinct paint contributions survive.');
