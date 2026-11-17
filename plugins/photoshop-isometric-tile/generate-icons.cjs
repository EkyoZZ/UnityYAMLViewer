// Deterministic panel artwork; transparent PNGs at native 1x and 2x sizes.
const fs = require('node:fs');
const path = require('node:path');
const { png } = require('./png.js');
for (const size of [23, 46]) {
    for (const [theme, color] of [['dark', 220], ['light', 65]]) {
        const rgba = new Uint8Array(size * size * 4);
        const scale = size / 23;
        const points = [[2,11.5],[11.5,6.75],[21,11.5],[11.5,16.25]];
        for (let y=0;y<size;y++) for (let x=0;x<size;x++) {
            let covered=0;
            for(let sy=0;sy<4;sy++) for(let sx=0;sx<4;sx++) {
                const px=(x+(sx+.5)/4)/scale, py=(y+(sy+.5)/4)/scale;
                let distance=Infinity;
                for(let i=0;i<4;i++) {
                    const a=points[i], b=points[(i+1)%4];
                    const dx=b[0]-a[0], dy=b[1]-a[1];
                    const t=Math.max(0,Math.min(1,((px-a[0])*dx+(py-a[1])*dy)/(dx*dx+dy*dy)));
                    distance=Math.min(distance,Math.hypot(px-a[0]-t*dx,py-a[1]-t*dy));
                }
                if(distance<=.7) covered++;
            }
            const offset=(y*size+x)*4;
            rgba.set([color,color,color,Math.round(255*covered/16)],offset);
        }
        const bytes=png(size,size,rgba);
        fs.writeFileSync(path.join(__dirname,`tile-${theme}${size===46?"@2x":""}.png`),bytes);
        if(size===23)fs.writeFileSync(path.join(__dirname,`tile-${theme}@1x.png`),bytes);
    }
}
