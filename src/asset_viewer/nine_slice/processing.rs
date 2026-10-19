//! Exact RGBA processing. Hashes only index candidates; equality is always verified.
use std::collections::HashMap;

#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,serde::Serialize,serde::Deserialize)]
pub(crate) enum Direction { #[default] Both, Horizontal, Vertical }
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,serde::Serialize,serde::Deserialize)]
pub(crate) enum Treatment { #[default] Stretch, Repeat }

#[derive(Clone,Debug,PartialEq,Eq,serde::Serialize,serde::Deserialize)]
pub(crate) struct Request {
    pub automatic: bool,
    pub direction: Direction,
    pub treatment: Treatment,
    pub preserve_transparent: bool,
    pub multiple_of_four: bool,
    pub copies: [u32;2],
    /// Distances from the original image edges, not from the trimmed rectangle.
    pub borders: Option<[u32;4]>,
    pub center: [u32;2],
}
impl Default for Request {
    fn default() -> Self {Self {automatic:false,direction:Direction::Both,treatment:Treatment::Stretch,preserve_transparent:true,multiple_of_four:true,copies:[1,1],borders:None,center:[4,4]}}
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(super) enum Reason {Disabled,TransparentImage,ProtectedSpan,NoUniformRun,NoCycle,ManualUniformMismatch,ManualCycleMismatch,CenterKeepsAll,CopiesKeepAll,AlignmentKeepsAll,Reduced}
impl Reason {
    pub fn label(self)->&'static str {match self {
        Self::Disabled=>"当前方案未启用此方向。",
        Self::TransparentImage=>"全图透明，保留透明边缘已开启，不删除原像素。",
        Self::ProtectedSpan=>"保留透明边缘后，可识别区域不足两个像素。",
        Self::NoUniformRun=>"未找到连续且完整 RGBA 一致的行列，不做近似合并。",
        Self::NoCycle=>"未找到至少两个完整的 RGBA 重复周期。",
        Self::ManualUniformMismatch=>"手动边界内存在 RGBA 差异，不能按拉伸方式精简。",
        Self::ManualCycleMismatch=>"手动边界内没有可删除的完整重复周期。",
        Self::CenterKeepsAll=>"重复区不大于中心保留尺寸，已全部保留。",
        Self::CopiesKeepAll=>"循环倍数已保留全部完整周期，没有多余周期可删。",
        Self::AlignmentKeepsAll=>"重复像素可删除，但 4 倍数对齐抵消了尺寸节省。",
        Self::Reduced=>"已安全精简此方向。",
    }}
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub(super) struct Difference {pub points:[[u32;2];2],pub rgba:[[u8;4];2]}
#[derive(Clone,Debug,PartialEq,Eq)]
pub(super) struct AxisReport {pub reason:Reason,pub span:u32,pub kept:u32,pub protected:u32,pub difference:Option<Difference>}
impl AxisReport {fn new(reason:Reason)->Self {Self {reason,span:0,kept:0,protected:0,difference:None}}}
#[derive(Clone,Debug)]
pub(super) struct Trial {pub direction:Direction,pub treatment:Treatment,pub size:[u32;2],pub reports:[AxisReport;2],pub eligible:bool}

#[derive(Clone,Debug,PartialEq,Eq)]
pub(super) struct Plan {
    pub reports: [AxisReport;2],
    pub crop: [u32;4],
    pub values: [u32;6],
    pub padding: [u32;2],
    pub alignment_tail: [u32;2],
    pub periods: [u32;2],
    pub blocked: [bool;2],
    pub treatment: Treatment,
    pub original: [u32;2],
}

fn alpha_bounds(source: &image::RgbaImage) -> Option<[u32;4]> {
    let (w,h)=source.dimensions();let (mut l,mut t,mut r,mut b)=(w,h,0,0);
    for (x,y,pixel) in source.enumerate_pixels() {if pixel.0[3]!=0 {l=l.min(x);t=t.min(y);r=r.max(x+1);b=b.max(y+1);}}
    (l<r && t<b).then_some([l,t,r,b])
}

fn symbols(source: &image::RgbaImage, columns: bool) -> Vec<u32> {
    let (w,h)=source.dimensions();let count=if columns {w}else{h};
    let mut hashes=vec![0xcbf29ce484222325u64;count as usize];
    for (x,y,pixel) in source.enumerate_pixels() {
        let index=if columns {x}else{y} as usize;
        hashes[index]=(hashes[index]^u32::from_le_bytes(pixel.0) as u64).wrapping_mul(0x100000001b3);
    }
    let mut buckets:HashMap<u64,Vec<(u32,u32)>>=HashMap::new();let mut next=0;let mut ids=Vec::with_capacity(count as usize);
    for (index,hash) in hashes.into_iter().enumerate() {
        let bucket=buckets.entry(hash).or_default();
        let equal=|other:u32|if columns {(0..h).all(|y|source.get_pixel(index as u32,y)==source.get_pixel(other,y))}
            else {let stride=w as usize*4;source.as_raw()[index*stride..(index+1)*stride]==source.as_raw()[other as usize*stride..(other as usize+1)*stride]};
        let id=if let Some((_,id))=bucket.iter().find(|(other,_)|equal(*other)) {*id}
            else {let id=next;next+=1;bucket.push((index as u32,id));id};
        ids.push(id);
    }
    ids
}

/// Shortest period of the finite sequence, including a clipped last cycle.
fn minimum_period(ids: &[u32]) -> usize {
    let mut prefix=vec![0;ids.len()];
    for i in 1..ids.len() {let mut j=prefix[i-1];while j>0 && ids[i]!=ids[j] {j=prefix[j-1];}if ids[i]==ids[j] {j+=1;}prefix[i]=j;}
    ids.len()-prefix.last().copied().unwrap_or(0)
}

fn region(ids: &[u32],start: usize,end: usize,treatment: Treatment,requested:u32,align:bool) -> Option<(usize,usize)> {
    let mut best:Option<(usize,usize,i64)>=None;
    let mut consider=|first:usize,length:usize,period:usize| {
        let span=if treatment==Treatment::Repeat {length/period*period}else{length} as u32;
        let (keep,pad)=retained(span,ids.len() as u32-span,period as u32,requested,align);
        let saving=i64::from(span)-i64::from(keep)-i64::from(pad);
        if best.is_none_or(|(old,n,score)|saving>score || (saving==score && (2*first+length).abs_diff(start+end)<(2*old+n).abs_diff(start+end))) {best=Some((first,length,saving));}
    };
    if treatment==Treatment::Stretch {
        let mut first=start;
        for i in start+1..=end {if i==end || ids[i]!=ids[first] {if i-first>=2 {consider(first,i-first,1);}first=i;}}
    }else {
        // O(n²) over exact integer symbols, off-thread; at most 8192 entries per axis.
        for period in 1..=(end-start)/2 {
            let mut matches=0;
            for i in start+period..=end {
                if i<end && ids[i]==ids[i-period] {matches+=1;}
                else {if matches>=period {consider(i-matches-period,matches+period,period);}matches=0;}
            }
        }
    }
    best.map(|(first,n,_)|(first,n))
}

fn difference(source:&image::RgbaImage,ids:&[u32],start:usize,end:usize,axis:usize,origin:[u32;2])->Option<Difference> {
    let first=(start+1..end).find(|i|ids[i-1]!=ids[*i])? as u32;
    let limit=if axis==0 {source.height()}else{source.width()};
    for across in 0..limit {
        let points=if axis==0 {[[first-1,across],[first,across]]}else{[[across,first-1],[across,first]]};
        let rgba=points.map(|[x,y]|source.get_pixel(x,y).0);
        if rgba[0]!=rgba[1] {return Some(Difference {points:points.map(|[x,y]|[x+origin[0],y+origin[1]]),rgba});}
    }
    None
}

fn retained(span:u32,border:u32,period:u32,requested:u32,align:bool) -> (u32,u32) {
    let count=(span/period).max(1);let copies=requested.max(1).min(count);
    let mut center=copies*period;
    if !align {return (center,0);}
    // Stretch has period 1: add exactly the missing 0/1/2/3 center pixels.
    if period==1 {return (center+(4-(border+center)%4)%4,0);}
    // Repeat must retain complete cycles, even when extending beyond the source span.
    for extra in 0..4 {
        let candidate=(copies+extra)*period;
        if (border+candidate)%4==0 {return (candidate,0);}
    }
    // Some border/period combinations cannot align using whole cycles alone.
    // Preserve the cycle and use transparent outer padding instead of clipping it.
    (center,(4-(border+center)%4)%4)
}

impl Plan {
    pub fn new(source:&image::RgbaImage,request:&Request) -> Self {
        let (w,h)=source.dimensions();let bounds=alpha_bounds(source);
        let [al,at,ar,ab]=bounds.unwrap_or([0,0,1,1]);
        let mut crop=[0,0,w,h];
        if !request.preserve_transparent {
            if request.direction!=Direction::Vertical {crop[0]=al;crop[2]=ar-al;}
            if request.direction!=Direction::Horizontal {crop[1]=at;crop[3]=ab-at;}
        }
        let working=image::imageops::crop_imm(source,crop[0],crop[1],crop[2],crop[3]).to_image();
        let mut v=[0,0,0,0,crop[2],crop[3]];let mut padding=[0,0];let mut alignment_tail=[0,0];let mut periods=[0,0];let mut blocked=[false,false];
        let mut reports=[AxisReport::new(Reason::Disabled),AxisReport::new(Reason::Disabled)];
        for axis in 0..2 {
            let full=crop[axis+2];let origin=crop[axis];let original=if axis==0 {w}else{h};
            let inactive=if axis==0 {request.direction==Direction::Vertical}else{request.direction==Direction::Horizontal};
            let (first,last,center)=(axis,axis+2,axis+4);
            if inactive || (request.preserve_transparent && bounds.is_none()) {
                reports[axis].reason=if inactive {Reason::Disabled}else{Reason::TransparentImage};
                padding[axis]=if request.multiple_of_four {(4-full%4)%4}else{0};continue;
            }
            let ids=symbols(&working,axis==0);
            let lo=if request.preserve_transparent {if axis==0 {al}else{at}}else{0};
            let hi=if request.preserve_transparent {if axis==0 {ar}else{ab}}else{full};
            let requested=if request.treatment==Treatment::Stretch {request.center[axis]}else{request.copies[axis]};
            reports[axis].span=hi-lo;reports[axis].kept=full;reports[axis].protected=full-(hi-lo);
            let selected=request.borders.map(|border| {
                let left=border[first].saturating_sub(origin).clamp(lo,hi-1);
                let right=border[last].saturating_sub(original-origin-full).clamp(full-hi,full-1-left);
                (left as usize,(full-left-right) as usize)
            }).or_else(||region(&ids,lo as usize,hi as usize,request.treatment,requested,request.multiple_of_four));
            let Some((left,length))=selected else {
                reports[axis].reason=if hi-lo<2 {Reason::ProtectedSpan}else if request.treatment==Treatment::Stretch {Reason::NoUniformRun}else{Reason::NoCycle};
                reports[axis].difference=difference(&working,&ids,lo as usize,hi as usize,axis,[crop[0],crop[1]]);
                padding[axis]=if request.multiple_of_four {(4-full%4)%4}else{0};continue;
            };
            if length==0 {blocked[axis]=true;continue;}
            reports[axis].span=length as u32;
            let sequence=&ids[left..left+length];
            let period=if request.treatment==Treatment::Stretch {if sequence.iter().all(|id|*id==sequence[0]) {1}else{length}}else{minimum_period(sequence)};
            if period>=length && length>1 {
                reports[axis].reason=if request.treatment==Treatment::Stretch {Reason::ManualUniformMismatch}else{Reason::ManualCycleMismatch};
                reports[axis].difference=difference(&working,&ids,left,left+length,axis,[crop[0],crop[1]]);
                blocked[axis]=true;v[first]=left as u32;v[last]=full-left as u32-length as u32;v[center]=length as u32;
                reports[axis].kept=length as u32;
                padding[axis]=if request.multiple_of_four {(4-full%4)%4}else{0};continue;
            }
            let copies=if request.treatment==Treatment::Stretch {request.center[axis]}else{request.copies[axis]};
            // Keep a clipped last cycle in the fixed border, never cut a stored loop short.
            let span=if request.treatment==Treatment::Repeat {(length/period)*period}else{length} as u32;
            let (keep,pad)=retained(span,full-span,period as u32,copies,request.multiple_of_four);
            if request.treatment==Treatment::Repeat && full-span+keep+pad>=original {
                // Alignment is useful only if the final axis is smaller than its source.
                // Keep original pixels and dimensions rather than enlarging a no-op result.
                v[first]=left as u32;v[last]=full-left as u32-span;v[center]=span;
                periods[axis]=period as u32;
                reports[axis].kept=span;
                reports[axis].reason=if request.multiple_of_four {Reason::AlignmentKeepsAll}else{Reason::CopiesKeepAll};
                continue;
            }
            v[first]=left as u32;v[last]=full-left as u32-span;v[center]=keep;
            if request.treatment==Treatment::Stretch {
                let base=copies.max(1).min(span);
                let extra=keep-base;
                v[center]=base;v[last]+=extra;alignment_tail[axis]=extra;
            }
            periods[axis]=period as u32;padding[axis]=pad;
            reports[axis].span=length as u32;reports[axis].kept=keep;
            reports[axis].reason=if keep<span {
                if v[first]+v[last]+v[center]+pad>=original {Reason::AlignmentKeepsAll}else{Reason::Reduced}
            }else if u64::from(copies)*(period as u64)<u64::from(span) {Reason::AlignmentKeepsAll}
            else if request.treatment==Treatment::Stretch {Reason::CenterKeepsAll}else{Reason::CopiesKeepAll};
        }
        Self {reports,crop,values:v,padding,alignment_tail,periods,blocked,treatment:request.treatment,original:[w,h]}
    }
    pub fn size(&self) -> [u32;2] {[self.values[0]+self.values[2]+self.values[4]+self.padding[0],self.values[1]+self.values[3]+self.values[5]+self.padding[1]]}
    pub fn borders(&self) -> [u32;6] {self.values}
    pub fn source_borders(&self) -> [u32;6] {
        let mut v=self.values;v[0]+=self.crop[0];v[1]+=self.crop[1];
        v[2]+=self.original[0]-self.crop[0]-self.crop[2];v[3]+=self.original[1]-self.crop[1]-self.crop[3];v
    }
    pub fn padding_sides(&self)->[u32;4] {[self.padding[0]/2,self.padding[1]/2,self.padding[0].div_ceil(2),self.padding[1].div_ceil(2)]}
    pub fn render(&self,source:&image::RgbaImage) -> image::RgbaImage {
        let [w,h]=self.size();let [l,t,r,b,cw,ch]=self.values;
        let map=|p:u32,first:u32,last:u32,center:u32,full:u32,period:u32,tail:u32| {
            if p<first {p}
            else if p<first+center+tail {if period>0 {first+(p-first)%period}else{p}}
            else {full-last+p-first-center}
        };
        let [pl,pt,pr,pb]=self.padding_sides();
        image::RgbaImage::from_fn(w,h,|x,y| {
            if x<pl || y<pt || x>=w-pr || y>=h-pb {image::Rgba([0;4])}
            else {*source.get_pixel(self.crop[0]+map(x-pl,l,r,cw,self.crop[2],self.periods[0],self.alignment_tail[0]),self.crop[1]+map(y-pt,t,b,ch,self.crop[3],self.periods[1],self.alignment_tail[1]))}
        })
    }
    #[cfg(test)]
    fn reconstructed_pixel(&self,output:&image::RgbaImage,x:u32,y:u32) -> image::Rgba<u8> {
        if x<self.crop[0] || y<self.crop[1] {return image::Rgba([0;4]);}
        let (x,y)=(x-self.crop[0],y-self.crop[1]);
        if x>=self.crop[2] || y>=self.crop[3] {return image::Rgba([0;4]);}
        let [l,t,r,b,cw,ch]=self.values;
        let map=|p:u32,first:u32,last:u32,keep:u32,full:u32| {
            if p<first {p}else if p>=full-last {first+keep+p-(full-last)}
            else if keep==full-first-last {p}
            else if self.treatment==Treatment::Repeat {first+(p-first)%keep}else{first}
        };
        *output.get_pixel(map(x,l,r,cw,self.crop[2])+self.padding[0]/2,map(y,t,b,ch,self.crop[3])+self.padding[1]/2)
    }

}

/// Respect pixel/loop retention and both checkboxes; only direction and treatment are selected.
pub(super) fn automatic(source:&image::RgbaImage,template:&Request)->(Request,Plan,Vec<Trial>) {
    let mut best:Option<((u64,u8,u8,u8,u8,u8),Request,Plan)>=None;
    let mut trials=Vec::with_capacity(6);
    for treatment in [Treatment::Stretch,Treatment::Repeat] {for direction in [Direction::Both,Direction::Horizontal,Direction::Vertical] {
        let mut request=template.clone();request.automatic=false;request.borders=None;request.direction=direction;request.treatment=treatment;
        let plan=Plan::new(source,&request);let size=plan.size();
        // A solid strip never needs Tiled solely to bypass the user's Stretch center retention.
        let eligible=treatment==Treatment::Stretch||plan.periods.iter().any(|period|*period>1);
        trials.push(Trial {direction,treatment,size,reports:plan.reports.clone(),eligible});
        if !eligible {continue;}
        let active=if direction==Direction::Both {2u8}else{1};
        let reduced=(0..2).filter(|axis|plan.crop[axis+2]<plan.original[*axis]||plan.values[*axis]+plan.values[axis+2]+plan.values[axis+4]<plan.crop[axis+2]).count() as u8;
        let score=(u64::from(size[0])*u64::from(size[1]),u8::from(treatment==Treatment::Repeat),if reduced>0 {active.saturating_sub(reduced)}else{0},if reduced>0 {active}else{0},u8::from(direction!=template.direction),match direction {Direction::Both=>0,Direction::Horizontal=>1,Direction::Vertical=>2});
        if best.as_ref().is_none_or(|(old,_,_)|score<*old) {best=Some((score,request,plan));}
    }}
    let (_,request,plan)=best.unwrap();(request,plan,trials)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn repeat_image() -> image::RgbaImage {
        image::RgbaImage::from_fn(26,18,|x,y| {
            let x=if x<2{x}else if x>=24{x-18}else{2+(x-2)%3};
            let y=if y<2{y}else if y>=16{y-10}else{2+(y-2)%2};
            image::Rgba([x as u8,y as u8,70,255])
        })
    }
    #[test]
    fn repeat_minimum_cycle_and_copies_reconstruct_every_rgba_pixel() {
        let source=repeat_image();
        for copies in [[1,1],[2,3]] {
            let request=Request {treatment:Treatment::Repeat,copies,multiple_of_four:false,..Default::default()};
            let plan=Plan::new(&source,&request);assert_eq!(plan.periods,[3,2]);
            assert_eq!([plan.values[4],plan.values[5]],[copies[0]*3,copies[1]*2]);
            let output=plan.render(&source);
            for y in 0..18 {for x in 0..26 {assert_eq!(plan.reconstructed_pixel(&output,x,y),*source.get_pixel(x,y));}}
        }
    }
    #[test]
    fn defaults_preserve_transparent_edges_and_align_both_sides_to_four() {
        let source=image::RgbaImage::from_fn(20,16,|x,y|if x<2||x>=18||y<3||y>=13 {image::Rgba([9,8,7,0])}else{image::Rgba([1,2,3,255])});
        let plan=Plan::new(&source,&Request::default());assert_eq!(plan.crop,[0,0,20,16]);
        assert!(plan.values[0]>=2&&plan.values[2]>=2&&plan.values[1]>=3&&plan.values[3]>=3);
        assert!(plan.size().iter().all(|v|v%4==0));let output=plan.render(&source);
        assert_eq!(output.get_pixel(0,0).0,[9,8,7,0]);
        let request=Request {preserve_transparent:false,multiple_of_four:false,center:[1,1],..Default::default()};
        let plan=Plan::new(&source,&request);assert_eq!(plan.crop,[2,3,16,10]);assert_eq!(plan.size(),[1,1]);
    }
    #[test]
    fn repeat_alignment_retains_whole_cycles_or_pads_without_resampling() {
        let source=repeat_image();let request=Request {treatment:Treatment::Repeat,..Default::default()};
        let plan=Plan::new(&source,&request);assert!(plan.size().iter().all(|v|v%4==0));
        assert_eq!(plan.values[4]%plan.periods[0],0);assert_eq!(plan.values[5]%plan.periods[1],0);
        let output=plan.render(&source);for y in 0..18 {for x in 0..26 {assert_eq!(plan.reconstructed_pixel(&output,x,y),*source.get_pixel(x,y));}}
        let odd=image::RgbaImage::from_fn(13,5,|x,y|image::Rgba([x as u8,y as u8,5,255]));
        let plan=Plan::new(&odd,&Request::default());assert_eq!(plan.size(),[16,8]);
        let output=plan.render(&odd);assert_eq!(output.get_pixel(15,7).0,[0;4]);
        for y in 0..5 {for x in 0..13 {assert_eq!(output.get_pixel(x+1,y+1),odd.get_pixel(x,y));assert_eq!(plan.reconstructed_pixel(&output,x,y),*odd.get_pixel(x,y));}}
    }
    #[test]
    fn padding_is_balanced_with_extra_on_right_bottom_and_excluded_from_border() {
        for side in [25,26,27] {
            let source=image::RgbaImage::from_fn(side,side,|x,y|image::Rgba([x as u8,y as u8,7,255]));
            let plan=Plan::new(&source,&Request::default());
            let total=28-side;
            let first=total/2;let last=total.div_ceil(2);
            assert_eq!(plan.padding_sides(),[first,first,last,last]);
            assert_eq!(plan.size(),[28,28]);
            assert_eq!(&plan.borders()[..4],&[0,0,0,0]);
            let output=plan.render(&source);
            for y in 0..28 {for x in 0..28 {
                let expected=if x>=first&&y>=first&&x<first+side&&y<first+side {*source.get_pixel(x-first,y-first)}else{image::Rgba([0;4])};
                assert_eq!(*output.get_pixel(x,y),expected);
            }}
        }
        let source=image::RgbaImage::from_fn(11,26,|x,y|image::Rgba([x as u8,y as u8,9,255]));
        let plan=Plan::new(&source,&Request {borders:Some([2,0,3,0]),..Default::default()});
        assert_eq!(&plan.borders()[..4],&[2,0,3,0]);
        assert_eq!(plan.padding_sides(),[0,1,1,1]);
    }
    #[test]
    fn stretch_alignment_pixels_belong_to_trailing_border() {
        for extra in 1..=3 {
            let right=7-extra;
            let source=image::RgbaImage::from_fn(1+16+right,4,|x,y|image::Rgba([if x==0 {9}else if x>=17 {8}else{1},y as u8,7,255]));
            let plan=Plan::new(&source,&Request {direction:Direction::Horizontal,..Default::default()});
            assert_eq!(plan.values,[1,0,right+extra,0,4,4]);
            assert_eq!(plan.padding,[0,0]);
            assert_eq!(plan.size(),[12,4]);
            let output=plan.render(&source);
            for y in 0..4 {for x in 0..source.width() {assert_eq!(plan.reconstructed_pixel(&output,x,y),*source.get_pixel(x,y));}}
        }
    }
    #[test]
    fn repeat_alignment_without_size_savings_keeps_original_pixels() {
        for width in [6,9,12] {
            let source=image::RgbaImage::from_fn(width,4,|x,y|image::Rgba([(x%3) as u8,y as u8,7,255]));
            let plan=Plan::new(&source,&Request {direction:Direction::Horizontal,treatment:Treatment::Repeat,..Default::default()});
            assert_eq!(plan.size(),[width,4]);
            assert_eq!(plan.padding,[0,0]);
            assert_eq!(plan.render(&source),source);
            assert_eq!(plan.reports[0].reason,Reason::AlignmentKeepsAll);
        }
        let source=image::RgbaImage::from_fn(15,4,|x,y|image::Rgba([(x%3) as u8,y as u8,7,255]));
        let plan=Plan::new(&source,&Request {direction:Direction::Horizontal,treatment:Treatment::Repeat,..Default::default()});
        assert_eq!(plan.size(),[12,4]);
        assert_eq!(plan.reports[0].reason,Reason::Reduced);
    }
    #[test]
    fn stretch_adds_only_missing_pixels_and_repeat_never_clips_cycles() {
        for missing in 1..=3 {
            assert_eq!(retained(8,missing,1,4,true),(8-missing,0));
        }
        assert_eq!(retained(9,1,3,1,true),(3,0));
        assert_eq!(retained(9,0,3,1,true),(12,0));
        assert_eq!(retained(8,1,2,1,true),(2,1));
        assert_eq!(retained(8,0,2,1,true),(4,0));
    }
    #[test]
    fn alignment_extends_active_centers_and_only_pads_inactive_axes() {
        let source=image::RgbaImage::from_fn(7,7,|x,y| {
            image::Rgba([if x==0 {9}else if x==6 {8}else{1},if y==0 {9}else if y==6 {8}else{2},7,255])
        });
        for direction in [Direction::Horizontal,Direction::Vertical,Direction::Both] {
            let plan=Plan::new(&source,&Request {direction,..Default::default()});
            assert_eq!(plan.size(),[8,8]);
            for axis in 0..2 {
                let active=direction==Direction::Both || (axis==0)==(direction==Direction::Horizontal);
                assert_eq!(plan.padding[axis],if active {0}else{1});
                assert_eq!(plan.values[axis],if active {1}else{0});
                assert_eq!(plan.values[axis+2],if active {3}else{0});
                assert_eq!(plan.values[axis+4],if active {4}else{7});
            }
            let output=plan.render(&source);
            for y in 0..7 {for x in 0..7 {assert_eq!(plan.reconstructed_pixel(&output,x,y),*source.get_pixel(x,y));}}
        }
    }
    #[test]
    fn hidden_rgb_and_alpha_differences_are_not_merged_and_one_axis_is_respected() {
        let source=image::RgbaImage::from_fn(12,8,|x,y|image::Rgba([x as u8,y as u8,0,0]));
        let plan=Plan::new(&source,&Request::default());assert_eq!(plan.render(&source),source);
        let alpha=image::RgbaImage::from_fn(12,8,|x,y|image::Rgba([10,20,30,(1+x+y*12) as u8]));
        assert_eq!(Plan::new(&alpha,&Request::default()).render(&alpha),alpha);
        let source=image::RgbaImage::from_fn(12,8,|x,_|image::Rgba([x as u8,40,50,255]));
        let request=Request {direction:Direction::Horizontal,multiple_of_four:false,..Default::default()};
        let plan=Plan::new(&source,&request);assert_eq!(plan.size(),[12,8]);
        let request=Request {direction:Direction::Vertical,multiple_of_four:false,..Default::default()};
        let plan=Plan::new(&source,&request);assert_eq!(plan.size(),[12,4]);
        assert_eq!(minimum_period(&[1,2,3,1,2,3,1]),3);
    }
    #[test]
    fn manual_borders_are_clamped_to_protected_edges_without_overflow() {
        let source=image::RgbaImage::from_fn(16,12,|x,y|if (4..12).contains(&x)&&(3..9).contains(&y) {image::Rgba([1,2,3,255])}else{image::Rgba([0;4])});
        for borders in [[15,11,15,11],[0,0,0,0],[u32::MAX;4]] {
            let request=Request {borders:Some(borders),..Default::default()};let plan=Plan::new(&source,&request);
            assert!(plan.values[0]>=4&&plan.values[2]>=4&&plan.values[1]>=3&&plan.values[3]>=3);
            assert_eq!(plan.crop,[0,0,16,12]);assert!(plan.size().iter().all(|v|v%4==0));
        }
    }
    #[test]
    fn smart_detection_selects_needed_axes_and_avoids_tiled_for_solid_strips() {
        for (source,direction) in [
            (image::RgbaImage::from_fn(32,16,|_,y|image::Rgba([y as u8,20,30,255])),Direction::Horizontal),
            (image::RgbaImage::from_fn(32,16,|x,_|image::Rgba([x as u8,20,30,255])),Direction::Vertical),
            (image::RgbaImage::from_pixel(32,16,image::Rgba([10,20,30,255])),Direction::Both),
        ] {
            let (request,plan,trials)=automatic(&source,&Request::default());
            assert_eq!(request.direction,direction);assert_eq!(request.treatment,Treatment::Stretch);assert_eq!(trials.len(),6);
            assert!(plan.size().iter().all(|n|n%4==0));
        }
        let source=repeat_image();let (request,plan,_)=automatic(&source,&Request::default());
        assert_eq!(request.direction,Direction::Both);assert_eq!(request.treatment,Treatment::Repeat);
        let output=plan.render(&source);for y in 0..18 {for x in 0..26 {assert_eq!(plan.reconstructed_pixel(&output,x,y),*source.get_pixel(x,y));}}
    }
    #[test]
    fn smart_detection_respects_retention_and_ranks_actual_aligned_size() {
        let mut ids:Vec<u32>=(1000..1040).collect();ids[1..11].fill(7);
        for i in 15..30 {ids[i]=100+(i-15) as u32%3;}
        assert_eq!(region(&ids,0,40,Treatment::Repeat,3,true),Some((1,10)));
        let request=Request {center:[8,6],copies:[2,3],preserve_transparent:false,multiple_of_four:false,borders:Some([1,1,1,1]),..Default::default()};
        let (resolved,_,trials)=automatic(&repeat_image(),&request);
        assert_eq!(resolved.center,[8,6]);assert_eq!(resolved.copies,[2,3]);assert!(!resolved.multiple_of_four);assert!(!resolved.preserve_transparent);assert_eq!(resolved.borders,None);
        let area=trials.iter().filter(|trial|trial.eligible).map(|trial|u64::from(trial.size[0])*u64::from(trial.size[1])).min().unwrap();
        let selected=Plan::new(&repeat_image(),&resolved).size();assert_eq!(u64::from(selected[0])*u64::from(selected[1]),area);
    }
    #[test]
    fn diagnostics_distinguish_rgba_noise_retention_alignment_and_cycles() {
        let gradient=image::RgbaImage::from_fn(12,8,|x,y|image::Rgba([x as u8,y as u8,5,255]));
        let plan=Plan::new(&gradient,&Request::default());assert_eq!(plan.reports[0].reason,Reason::NoUniformRun);assert!(plan.reports[0].difference.is_some());assert_eq!(plan.reports[0].kept,12);
        let hidden=image::RgbaImage::from_fn(12,8,|x,y|if y==0||y==7 {image::Rgba([0,0,0,255])}else{image::Rgba([x as u8,0,0,0])});
        let plan=Plan::new(&hidden,&Request::default());let diff=plan.reports[0].difference.as_ref().unwrap();assert_eq!([diff.rgba[0][3],diff.rgba[1][3]],[0,0]);assert_ne!(diff.rgba[0],diff.rgba[1]);
        let solid=image::RgbaImage::from_pixel(12,8,image::Rgba([1,2,3,255]));
        let request=Request {borders:Some([3,0,3,0]),..Default::default()};let plan=Plan::new(&solid,&request);assert_eq!(plan.reports[0].reason,Reason::AlignmentKeepsAll);
        let request=Request {borders:Some([4,0,4,0]),..Default::default()};assert_eq!(Plan::new(&solid,&request).reports[0].reason,Reason::CenterKeepsAll);
        let request=Request {treatment:Treatment::Repeat,copies:[10,10],borders:Some([2,2,2,2]),multiple_of_four:false,..Default::default()};assert_eq!(Plan::new(&repeat_image(),&request).reports[0].reason,Reason::CopiesKeepAll);
        let clear=image::RgbaImage::from_pixel(12,8,image::Rgba([8,9,10,0]));assert_eq!(Plan::new(&clear,&Request::default()).reports[0].reason,Reason::TransparentImage);
    }
}
