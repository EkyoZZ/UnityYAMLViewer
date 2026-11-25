//! Distance-spaced dabs and stroke-local flow, implemented for Tile's pixel mapping.
use super::*;
#[derive(Clone,Copy)]
pub(super) struct Sample {pub pos:Pos2,pub pressure:Option<f32>}
#[derive(Default)]
pub(super) struct Sampler { last:Option<Sample>, remaining:f32, diameter:f32 }
impl Sampler {
    pub fn reset(&mut self){*self=Self::default();}
    fn feed(&mut self,sample:Sample,size:f32,spacing:f32)->Vec<Sample>{
        let Some(last)=self.last else{self.last=Some(sample);self.diameter=size;self.remaining=(size*spacing).max(0.25);return vec![sample];};
        let delta=sample.pos-last.pos;let length=delta.length();self.last=Some(sample);
        let previous_size=self.diameter;self.diameter=size;
        if length<1e-5{return if sample.pressure!=last.pressure {vec![sample]}else{Vec::new()};}
        let mut output=Vec::new();let mut distance=self.remaining;
        while distance<=length && output.len()<65536 {
            let t=distance/length;
            output.push(Sample{pos:last.pos+delta*t,pressure:sample.pressure.map(|p|last.pressure.unwrap_or(p)+(p-last.pressure.unwrap_or(p))*t)});
            distance+=((previous_size+(size-previous_size)*t)*spacing).max(0.25);
        }
        self.remaining=(distance-length).max(0.0001);output
    }
}
pub(super) fn accumulate(old:f32,tip:f32,flow:f32,ceiling:f32)->f32 {
    old+(ceiling.clamp(0.,1.)-old).max(0.)*tip.clamp(0.,1.)*flow.clamp(0.,1.)
}
impl Editor {
    pub(super) fn brush_segment(&mut self,from:Pos2,to:Pos2,layout:bool){
        if !self.prepare_pixel_paint(|editor| [from, to].into_iter().any(|p| {
            editor.paint_hit((p.x.round() as i32, p.y.round() as i32), layout)
        })) {return;}
        if self.before.is_none(){self.before=Some(self.snapshot());self.brush_sampler.reset();}
        let sample=Sample{pos:to,pressure:self.input_pressure};
        let size=self.pressure_size(sample.pressure) as f32;
        let spacing=f32::from(self.brush_spacing.max(1))/100.;
        let mut dabs=Vec::new();
        if self.brush_sampler.last.is_none(){dabs.extend(self.brush_sampler.feed(Sample{pos:from,pressure:self.input_pressure},size,spacing));}
        dabs.extend(self.brush_sampler.feed(sample,size,spacing));
        for dab in dabs {
            let diameter=self.pressure_size(dab.pressure) as f32;
            let radius=diameter/2.;
            let center=dab.pos+egui::vec2(0.5,0.5);
            let ceiling=self.pressure_alpha(dab.pressure);
            let flow=f32::from(self.brush_flow)/100.*if self.pressure_flow{dab.pressure.unwrap_or(1.).clamp(0.,1.)}else{1.};
            if flow<=0. || ceiling<=0.{continue;}
            for y in (center.y-radius).floor() as i32..(center.y+radius).ceil() as i32 {
                for x in (center.x-radius).floor() as i32..(center.x+radius).ceil() as i32 {
                    let mut tip=0.;
                    for sy in 0..4 {for sx in 0..4 {
                        let dx=(x as f32+(sx as f32+0.5)/4.-center.x)/radius;
                        let dy=(y as f32+(sy as f32+0.5)/4.-center.y)/radius;
                        tip+=round_tip_coverage((dx*dx+dy*dy).sqrt(),self.brush_hardness);
                    }}
                    tip/=16.;if tip<=0.{continue;}
                    if let Some((tile,px,py))=self.plane_hit(x,y,layout){
                        if !self.selection.allows(tile,px,py){continue;}
                        if self.subgroup_active() && layout{self.subgroup_touches.insert((x,y));}
                        let old=self.brush_coverage.get(&(tile,px,py)).copied().unwrap_or(0.);
                        let coverage=accumulate(old,tip,flow,ceiling);
                        if coverage<=old{continue;}
                        self.brush_coverage.insert((tile,px,py),coverage);
                        let before=self.before_edit_pixel(tile,px,py);
                        let color=self.blend_edit_pixel(before,Rgba(self.color),coverage,self.tool==Tool::Eraser,self.brush_mode);
                        self.write_pixel(tile,px,py,color);
                    }
                }
            }
        }
        self.stroke_pressure=self.input_pressure;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn flow_builds_up_but_stroke_opacity_remains_a_ceiling(){let a=accumulate(0.,1.,0.2,0.5);assert!((a-0.1).abs()<1e-6);let b=accumulate(a,1.,0.2,0.5);assert!(b>a&&b<0.5);assert_eq!(accumulate(b,1.,1.,0.1),b);assert_eq!(accumulate(b,1.,0.,1.),b);}
    #[test]fn sampling_is_independent_of_frame_chunks_and_stationary_frames(){let mut a=Sampler::default();let mut b=Sampler::default();let sample=|x|Sample{pos:Pos2::new(x,0.),pressure:None};a.feed(sample(0.),10.,0.25);b.feed(sample(0.),10.,0.25);let one=a.feed(sample(20.),10.,0.25);let mut split=b.feed(sample(7.),10.,0.25);split.extend(b.feed(sample(20.),10.,0.25));assert_eq!(one.iter().map(|s|s.pos).collect::<Vec<_>>(),split.iter().map(|s|s.pos).collect::<Vec<_>>());assert!(b.feed(sample(20.),10.,0.25).is_empty());}
}
