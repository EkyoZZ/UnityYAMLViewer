// Dependency-free PNG-24/RGBA encoder. Web export uses PNG filters + LZ77 DEFLATE.
function paeth(a,b,c){const p=a+b-c,pa=Math.abs(p-a),pb=Math.abs(p-b),pc=Math.abs(p-c);return pa<=pb&&pa<=pc?a:pb<=pc?b:c;}
function filteredRows(w,h,rgba){
    const stride=w*4,raw=new Uint8Array(h*(stride+1));
    for(let y=0;y<h;y++){
        let best=Infinity,bestFilter=0;const candidates=Array.from({length:5},()=>new Uint8Array(stride));
        for(let filter=0;filter<5;filter++){
            let score=0;
            for(let x=0;x<stride;x++){
                const index=y*stride+x,a=x>=4?rgba[index-4]:0,b=y?rgba[index-stride]:0,c=y&&x>=4?rgba[index-stride-4]:0;
                const predictor=filter===0?0:filter===1?a:filter===2?b:filter===3?Math.floor((a+b)/2):paeth(a,b,c),value=(rgba[index]-predictor)&255;
                candidates[filter][x]=value;score+=Math.min(value,256-value);
            }
            if(score<best){best=score;bestFilter=filter;}
        }
        raw[y*(stride+1)]=bestFilter;raw.set(candidates[bestFilter],y*(stride+1)+1);
    }return raw;
}
function deflateFixed(raw){
    const out=[];let bits=0,count=0;
    function write(value,length){bits|=value<<count;count+=length;while(count>=8){out.push(bits&255);bits>>>=8;count-=8;}}
    const reverse=(value,length)=>{let result=0;for(let i=0;i<length;i++){result=(result<<1)|(value&1);value>>>=1;}return result;};
    function symbol(value){const length=value<144?8:value<256?9:value<280?7:8,code=value<144?48+value:value<256?400+value-144:value<280?value-256:192+value-280;write(reverse(code,length),length);}
    const lengthBase=[3,4,5,6,7,8,9,10,11,13,15,17,19,23,27,31,35,43,51,59,67,83,99,115,131,163,195,227,258];
    const lengthExtra=[0,0,0,0,0,0,0,0,1,1,1,1,2,2,2,2,3,3,3,3,4,4,4,4,5,5,5,5,0];
    const distanceBase=[1,2,3,4,5,7,9,13,17,25,33,49,65,97,129,193,257,385,513,769,1025,1537,2049,3073,4097,6145,8193,12289,16385,24577];
    const head=new Int32Array(65536),previous=new Int32Array(32768);head.fill(-1);previous.fill(-1);
    const hash=pos=>((raw[pos]*251+raw[pos+1])*251+raw[pos+2])&65535;
    function insert(pos){if(pos+2>=raw.length)return;const key=hash(pos);previous[pos&32767]=head[key];head[key]=pos;}
    write(3,3); // final block, fixed Huffman coding
    for(let pos=0;pos<raw.length;){
        let length=0,distance=0;
        if(pos+2<raw.length){
            let candidate=head[hash(pos)],attempts=0;const maximum=Math.min(258,raw.length-pos);
            while(candidate>=0&&pos-candidate<=32768&&attempts++<64){
                let match=0;while(match<maximum&&raw[candidate+match]===raw[pos+match])match++;
                if(match>length&&match>=3){length=match;distance=pos-candidate;if(match===maximum)break;}
                const next=previous[candidate&32767];if(next>=candidate)break;candidate=next;
            }
        }
        if(length>=3){
            let index=0;while(index<28&&lengthBase[index+1]<=length)index++;
            symbol(257+index);write(length-lengthBase[index],lengthExtra[index]);
            let code=0;while(code<29&&distanceBase[code+1]<=distance)code++;
            write(reverse(code,5),5);write(distance-distanceBase[code],code<4?0:Math.floor(code/2)-1);
            for(let offset=0;offset<length;offset++)insert(pos+offset);pos+=length;
        }else{symbol(raw[pos]);insert(pos++);}
    }
    symbol(256);if(count)out.push(bits&255);return out;
}
function png(w,h,rgba,web=false) {
    if (rgba.length!==w*h*4) throw new Error("Invalid RGBA dimensions");
    const raw=web?filteredRows(w,h,rgba):new Uint8Array(h*(w*4+1));
    if(!web)for(let y=0;y<h;y++) raw.set(rgba.subarray(y*w*4,(y+1)*w*4),y*(w*4+1)+1);
    const z=[0x78,0x01];
    let a=1,b=0;
    for(let i=0;i<raw.length;i++){a=(a+raw[i])%65521;b=(b+a)%65521;}
    const compressed=web?deflateFixed(raw):null;
    if(compressed&&compressed.length<raw.length+Math.ceil(raw.length/65535)*5){for(const byte of compressed)z.push(byte);}
    else for(let pos=0;pos<raw.length;pos+=65535){
        const n=Math.min(65535,raw.length-pos);
        z.push(pos+n===raw.length?1:0,n&255,n>>>8,(~n)&255,((~n)>>>8)&255);
        for(let i=0;i<n;i++)z.push(raw[pos+i]);
    }
    const u32=n=>[(n>>>24)&255,(n>>>16)&255,(n>>>8)&255,n&255];
    z.push(...u32(((b<<16)|a)>>>0));
    const out=[137,80,78,71,13,10,26,10];
    function chunk(name,data){
        const bytes=[...Array.from(name,c=>c.charCodeAt(0)),...data];
        let crc=0xffffffff;
        for(const v of bytes){crc^=v;for(let k=0;k<8;k++)crc=(crc>>>1)^((crc&1)?0xedb88320:0);}
        out.push(...u32(data.length));
        // Avoid spreading large buffers into function arguments.
        for(const v of bytes)out.push(v);
        out.push(...u32((crc^0xffffffff)>>>0));
    }
    chunk("IHDR",[...u32(w),...u32(h),8,6,0,0,0]);chunk("IDAT",z);chunk("IEND",[]);
    return new Uint8Array(out);
}
function base64(bytes){
    const alphabet="ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let result="";
    for(let i=0;i<bytes.length;i+=3){
        const n=(bytes[i]<<16)|((bytes[i+1]||0)<<8)|(bytes[i+2]||0);
        result+=alphabet[(n>>>18)&63]+alphabet[(n>>>12)&63]+(i+1<bytes.length?alphabet[(n>>>6)&63]:"=")+(i+2<bytes.length?alphabet[n&63]:"=");
    }
    return result;
}
module.exports={png,base64};
