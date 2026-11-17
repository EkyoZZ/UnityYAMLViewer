const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const zlib=require('node:zlib');
const manifest=require('./manifest.json');
for(const icon of [...manifest.entrypoints[0].icons,...manifest.icons]) {
    assert.deepEqual(icon.species,['generic']);
    assert.deepEqual(fs.readFileSync(path.join(__dirname,icon.path)),fs.readFileSync(path.join(__dirname,icon.path.replace(/\.png$/,'@1x.png'))));
    assert.deepEqual(icon.scale,[1,2]);
    for(const scale of icon.scale) {
        const filename=scale===1?icon.path:icon.path.replace(/\.png$/,`@${scale}x.png`);
        const png=fs.readFileSync(path.join(__dirname,filename));
        const size=23*scale;
        assert.equal(png.readUInt32BE(16),size);
        assert.equal(png.readUInt32BE(20),size);
        assert.equal(png[25],6,'must retain RGBA transparency');
        const idat=[];
        for(let offset=8;offset<png.length;) {
            const length=png.readUInt32BE(offset);
            if(png.toString('ascii',offset+4,offset+8)==='IDAT')idat.push(png.subarray(offset+8,offset+8+length));
            offset+=12+length;
        }
        const raw=zlib.inflateSync(Buffer.concat(idat));
        const alpha=(x,y)=>raw[y*(size*4+1)+1+x*4+3];
        assert.equal(alpha(0,0),0);
        assert.equal(alpha(Math.floor(size/2),Math.floor(size/2)),0,'diamond is hollow');
        assert(raw.some((value,index)=>index%(size*4+1)>0&&(index%(size*4+1)-1)%4===3&&value>0),'outline is visible');
    }
}
console.log('Panel icon paths, @2x resolution, PNG dimensions and hollow alpha checks passed.');
