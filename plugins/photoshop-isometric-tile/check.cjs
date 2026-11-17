// Run with node check.cjs; no Photoshop or native build required.
const assert=require('node:assert/strict');
const zlib=require('node:zlib');
const {png,base64}=require('./png.js');
const {unfilter}=require('./png-test-utils.cjs');
let random=1234567;
for(const [w,h] of [[1,1],[128,64],[257,67],[512,512]]) for(const pattern of [0,1,2])for(const web of [false,true]){
    const rgba=Uint8Array.from({length:w*h*4},(_,i)=>{random=(Math.imul(random,1664525)+1013904223)>>>0;return pattern===0?(i*31)%256:pattern===1?random>>>24:(i%4===3?Math.floor(i/4)%256:100);});
    const bytes=Buffer.from(png(w,h,rgba,web));
    assert.equal(base64(bytes),bytes.toString('base64'));
    const chunks=[];
    for(let pos=8;pos<bytes.length;) {
        const n=bytes.readUInt32BE(pos),name=bytes.toString('ascii',pos+4,pos+8);
        let crc=0xffffffff;
        for(const v of bytes.subarray(pos+4,pos+8+n)){crc^=v;for(let i=0;i<8;i++)crc=(crc>>>1)^((crc&1)?0xedb88320:0);}
        assert.equal((crc^0xffffffff)>>>0,bytes.readUInt32BE(pos+8+n));
        if(name==='IDAT')chunks.push(bytes.subarray(pos+8,pos+8+n));
        pos+=12+n;
    }
    const raw=unfilter(zlib.inflateSync(Buffer.concat(chunks)),w,h);
    for(let y=0;y<h;y++) {
        assert.equal(raw[y*(w*4+1)],0);
        assert.deepEqual(raw.subarray(y*(w*4+1)+1,(y+1)*(w*4+1)),Buffer.from(rgba.subarray(y*w*4,(y+1)*w*4)));
    }
}
const sample=Uint8Array.from({length:128*64*4},(_,i)=>i%4===3?255:Math.floor(i/4/128));
const plain=png(128,64,sample).length,web=png(128,64,sample,true).length;
assert(web<plain/2,'web PNG must compress structured pixels without palette loss');
console.log(`PNG lossless round-trip, filters, CRC, random data and compressed/stored blocks passed; 128x64 gradient ${plain} -> ${web} bytes.`);
