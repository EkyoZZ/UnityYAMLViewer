// Independent PNG row decoder used only by host-independent checks.
function unfilter(raw,width,height){
    const stride=width*4,result=Buffer.alloc(height*(stride+1));
    for(let y=0;y<height;y++){
        const row=y*(stride+1),type=raw[row];
        if(type>4)throw Error('Invalid PNG filter');
        for(let x=0;x<stride;x++){
            const a=x>=4?result[row+1+x-4]:0,b=y?result[row+1+x-stride-1]:0,c=y&&x>=4?result[row+1+x-stride-5]:0;
            let predictor=0;
            if(type===1)predictor=a;if(type===2)predictor=b;if(type===3)predictor=(a+b)>>>1;
            if(type===4){const estimate=a+b-c,distances=[Math.abs(estimate-a),Math.abs(estimate-b),Math.abs(estimate-c)];predictor=distances[0]<=distances[1]&&distances[0]<=distances[2]?a:distances[1]<=distances[2]?b:c;}
            result[row+1+x]=(raw[row+1+x]+predictor)&255;
        }
    }return result;
}
module.exports={unfilter};
