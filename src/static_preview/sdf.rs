//! On-demand glyph distance fields. CPU thresholding keeps the normal egui renderer.
use super::*;
use std::hash::{Hash,Hasher};
const SIDE:usize=1024;
const PAD:usize=32;
const MAX_FIELDS:usize=16*1024*1024;
const MAX_PAGES:usize=4;
struct Field {size:[usize;2],distance:Vec<f32>}
struct Page {texture:egui::TextureHandle,x:usize,y:usize,row:usize}
#[derive(Clone)]
struct Tile {page:usize,rect:egui::Rect,size:[usize;2]}
#[derive(Default)]
pub(super) struct Atlas {
    fields:HashMap<u64,Field>, pending:HashMap<u64,usize>, tiles:HashMap<(u64,u16),Tile>, pages:Vec<Page>,
    bytes:usize, receiver:Option<mpsc::Receiver<(u64,Field)>>,
    sender:Option<mpsc::Sender<(u64,Field)>>,
}
impl Atlas {
    pub(super) fn has_state(&self) -> bool {self.sender.is_some() || !self.fields.is_empty() || !self.pages.is_empty()}
    pub(super) fn begin(&mut self) {
        if self.bytes>=MAX_FIELDS { *self=Self::default(); }
        if let Some(receiver)=&self.receiver {
            while let Ok((key,field))=receiver.try_recv() {
                self.pending.remove(&key);
                if self.bytes+field.distance.len()*4<=MAX_FIELDS {
                    self.bytes+=field.distance.len()*4;self.fields.insert(key,field);
                }
            }
        }
    }
    pub(super) fn paint(&mut self,ctx:&egui::Context,painter:&egui::Painter,galley:&egui::Galley,origin:Pos2,angle:f32,width:f32,offset:Vec2,color:Color32) {
        if color.a()==0 || width<=0.0 {return;}
        if self.sender.is_none() {let (sender,receiver)=mpsc::channel();self.sender=Some(sender);self.receiver=Some(receiver);}
        // Rasterize used characters at a stable reference size, independent of preview zoom.
        let mut reference_uv=HashMap::new();
        for row in &galley.rows {for glyph in &row.glyphs {
            let family=galley.job.sections[glyph.section_index as usize].format.font_id.family.clone();
            reference_uv.entry((glyph.chr,family.clone())).or_insert_with(|| {
                ctx.fonts(|fonts|fonts.layout_no_wrap(glyph.chr.to_string(),egui::FontId::new(64.0,family),Color32::WHITE))
                    .rows.first().and_then(|row|row.glyphs.first()).map(|g|g.uv_rect).unwrap_or_default()
            });
        }}
        let source=ctx.fonts(|fonts|fonts.texture_atlas());
        let atlas=source.lock();let image=atlas.image();
        let mut requests=Vec::new();let mut meshes:HashMap<usize,egui::Mesh>=HashMap::new();
        let rotate=|v:Vec2|egui::vec2(v.x*angle.cos()-v.y*angle.sin(),v.x*angle.sin()+v.y*angle.cos());
        for row in &galley.rows {for glyph in &row.glyphs {
            let uv=glyph.uv_rect;if uv.is_nothing() {continue;}
            let family=galley.job.sections[glyph.section_index as usize].format.font_id.family.clone();
            let source_uv=reference_uv[&(glyph.chr,family)];if source_uv.is_nothing() {continue;}
            let w=(source_uv.max[0]-source_uv.min[0]) as usize;let h=(source_uv.max[1]-source_uv.min[1]) as usize;
            if w+2*PAD>SIDE || h+2*PAD>SIDE {continue;}
            let mut coverage=Vec::with_capacity(w*h);
            for y in source_uv.min[1] as usize..source_uv.max[1] as usize {
                for x in source_uv.min[0] as usize..source_uv.max[0] as usize {coverage.push((image.pixels[y*image.size[0]+x].clamp(0.0,1.0)*255.0).round() as u8);}
            }
            let mut hash=std::collections::hash_map::DefaultHasher::new();w.hash(&mut hash);h.hash(&mut hash);coverage.hash(&mut hash);let key=hash.finish();
            if !self.fields.contains_key(&key) {
                let cost=(w+2*PAD)*(h+2*PAD)*4;
                let pending_bytes:usize=self.pending.values().sum();
                if !self.pending.contains_key(&key) && self.pending.len()<256 && self.bytes+pending_bytes+cost<=MAX_FIELDS {
                    self.pending.insert(key,cost);requests.push((key,[w,h],coverage));
                }
                continue;
            }
            let pixel_scale=(w as f32/uv.size.x).max(h as f32/uv.size.y);
            let radius=((width*pixel_scale).clamp(0.25,(PAD-2) as f32)*4.0).round() as u16;
            let tile_key=(key,radius);
            if !self.tiles.contains_key(&tile_key) {
                let field=&self.fields[&key];let size=field.size;
                let pixels=field.distance.iter().map(|distance| {
                    let alpha=((radius as f32/4.0-distance+0.5).clamp(0.0,1.0)*255.0).round() as u8;
                    Color32::from_rgba_premultiplied(alpha,alpha,alpha,alpha)
                }).collect();
                if let Some(tile)=self.insert(ctx,egui::ColorImage{size,pixels}) {self.tiles.insert(tile_key,tile);} else {continue;}
            }
            let tile=&self.tiles[&tile_key];
            let padding=egui::vec2(PAD as f32*uv.size.x/w as f32,PAD as f32*uv.size.y/h as f32);
            let local=glyph.pos+uv.offset-padding;
            let dimensions=egui::vec2(tile.size[0] as f32*uv.size.x/w as f32,tile.size[1] as f32*uv.size.y/h as f32);
            let corners=[local,local+egui::vec2(dimensions.x,0.0),local+dimensions,local+egui::vec2(0.0,dimensions.y)];
            let tex=[tile.rect.left_top(),tile.rect.right_top(),tile.rect.right_bottom(),tile.rect.left_bottom()];
            let mesh=meshes.entry(tile.page).or_insert_with(||egui::Mesh::with_texture(self.pages[tile.page].texture.id()));
            let base=mesh.vertices.len() as u32;
            for i in 0..4 {mesh.vertices.push(egui::epaint::Vertex{pos:origin+rotate(corners[i].to_vec2())+offset,uv:tex[i],color});}
            mesh.indices.extend([base,base+1,base+2,base,base+2,base+3]);
        }}
        drop(atlas);
        if !requests.is_empty() {
            let sender=self.sender.as_ref().unwrap().clone();let ctx=ctx.clone();
            std::thread::spawn(move || {for (key,size,coverage) in requests {
                if sender.send((key,distance_field(size,&coverage))).is_err() {return;}
                ctx.request_repaint();
            }});
        }
        for (_,mesh) in meshes {painter.add(egui::Shape::mesh(mesh));}
    }
    fn insert(&mut self,ctx:&egui::Context,image:egui::ColorImage) -> Option<Tile> {
        let [w,h]=image.size;
        for index in 0..=self.pages.len() {
            if index==self.pages.len() {
                if self.pages.len()>=MAX_PAGES {return None;}
                let texture=ctx.load_texture("preview_sdf_outline_atlas",egui::ColorImage::new([SIDE,SIDE],Color32::TRANSPARENT),egui::TextureOptions::LINEAR);
                self.pages.push(Page{texture,x:0,y:0,row:0});
            }
            let page=&mut self.pages[index];
            if page.x+w>SIDE {page.x=0;page.y+=page.row;page.row=0;}
            if page.y+h>SIDE {continue;}
            let pos=[page.x,page.y];page.texture.set_partial(pos,image,egui::TextureOptions::LINEAR);
            page.x+=w;page.row=page.row.max(h);
            return Some(Tile{page:index,size:[w,h],rect:egui::Rect::from_min_max(egui::pos2(pos[0] as f32/SIDE as f32,pos[1] as f32/SIDE as f32),egui::pos2((pos[0]+w) as f32/SIDE as f32,(pos[1]+h) as f32/SIDE as f32))});
        }
        None
    }
}
fn distance_field(size:[usize;2],coverage:&[u8]) -> Field {
    let [w,h]=[size[0]+2*PAD,size[1]+2*PAD];
    let mut inside=vec![false;w*h];let mut alpha=vec![0.0;w*h];
    for y in 0..size[1] {for x in 0..size[0] {
        let index=(y+PAD)*w+x+PAD;alpha[index]=coverage[y*size[0]+x] as f32/255.0;inside[index]=alpha[index]>=0.5;
    }}
    let to_inside=distances(w,h,&inside,true);let to_outside=distances(w,h,&inside,false);
    let distance=(0..w*h).map(|i| {
        if alpha[i]>0.0 && alpha[i]<1.0 {0.5-alpha[i]}
        else if inside[i] {-(to_outside[i]-0.5)} else {to_inside[i]-0.5}
    }).collect();
    Field{size:[w,h],distance}
}
// Two-pass chamfer transform; diagonal cost approximates Euclidean distance.
fn distances(w:usize,h:usize,inside:&[bool],target:bool) -> Vec<f32> {
    let mut d:Vec<f32>=inside.iter().map(|v|if *v==target {0.0}else {1e6}).collect();
    for y in 0..h {for x in 0..w {
        let i=y*w+x;
        if x>0 {d[i]=d[i].min(d[i-1]+1.0);}
        if y>0 {d[i]=d[i].min(d[i-w]+1.0);if x>0 {d[i]=d[i].min(d[i-w-1]+std::f32::consts::SQRT_2);}if x+1<w {d[i]=d[i].min(d[i-w+1]+std::f32::consts::SQRT_2);}}
    }}
    for y in (0..h).rev() {for x in (0..w).rev() {
        let i=y*w+x;
        if x+1<w {d[i]=d[i].min(d[i+1]+1.0);}
        if y+1<h {d[i]=d[i].min(d[i+w]+1.0);if x>0 {d[i]=d[i].min(d[i+w-1]+std::f32::consts::SQRT_2);}if x+1<w {d[i]=d[i].min(d[i+w+1]+std::f32::consts::SQRT_2);}}
    }}
    d
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn field_has_inside_outside_and_diagonal_expansion() {
        let f=distance_field([1,1],&[255]);let width=f.size[0];let i=PAD*width+PAD;
        assert!(f.distance[i]<0.0);assert!((f.distance[i+1]-0.5).abs()<0.001);
        assert!(f.distance[i+width+1]>0.5 && f.distance[i+width+1]<1.0);
        assert!(f.distance[0]>20.0);
    }
    #[test]
    fn empty_glyph_does_not_generate_a_solid_outline() {let f=distance_field([2,2],&[0;4]);assert!(f.distance.iter().all(|d|*d>100.0));}
}
