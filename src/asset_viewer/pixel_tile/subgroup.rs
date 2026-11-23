//! Subgroup boundary linkage, committed with the originating stroke.
use super::*;
impl Editor {
    pub(super) fn subgroup_active(&self)->bool {
        !self.mask_editing && self.subgroup_enabled && self.multiple && !self.elevated && self.hex==0 && self.subgroup_size.iter().all(|n|*n>0)
            && self.columns%self.subgroup_size[0] as usize==0 && self.rows%self.subgroup_size[1] as usize==0
    }
    pub(super) fn subgroup_sync(&mut self) {
        if !self.subgroup_active() || self.subgroup_touches.is_empty() || self.before.is_none(){return;}
        let gc=self.subgroup_size[0] as i32;let gr=self.subgroup_size[1] as i32;
        let start_col=(self.selected_slot%8) as i32/gc*gc;let start_row=(self.selected_slot/8) as i32/gr*gr;
        let (w,h)=self.pixels.dimensions();
        let coords=|x:i32,y:i32| {let x=(x as f32+0.5)/w as f32;let y=(y as f32+0.5)/h as f32;if self.diamond {(x+y+0.5-start_col as f32,y-x+0.5-start_row as f32)}else{(x+0.5-start_col as f32,y+0.5-start_row as f32)}};
        let before=self.before.as_ref().unwrap();
        let mut changed=Vec::new();let (mut low_u,mut high_u,mut low_v,mut high_v)=(0_f32,0_f32,0_f32,0_f32);
        for &(x,y) in &self.subgroup_touches {
            let cell=self.mapped_cell(x,y,w,h);let tile=self.tile_at(cell.col,cell.row);
            let old=*before.tiles[tile].get_pixel(cell.x,cell.y);let new=*self.tiles[tile].get_pixel(cell.x,cell.y);
            if old==new{continue;}
            let (u,v)=coords(x,y);low_u=low_u.max(-u);high_u=high_u.max(u-gc as f32);low_v=low_v.max(-v);high_v=high_v.max(v-gr as f32);
            changed.push((cell,tile,new,u,v));
        }
        let epsilon=1./w as f32+1./h as f32;
        let mut canonical=std::collections::BTreeMap::new();
        for (cell,tile,new,u,v) in changed {
            let outside=u<0. || v<0. || u>=gc as f32 || v>=gr as f32;
            let edge=u<=epsilon || v<=epsilon || u>=gc as f32-epsilon || v>=gr as f32-epsilon;
            let paired=(low_u>0. && u<=low_u+epsilon)||(high_u>0. && u>=gc as f32-high_u-epsilon)||(low_v>0. && v<=low_v+epsilon)||(high_v>0. && v>=gr as f32-high_v-epsilon);
            if !(outside||edge||paired){continue;}
            let key=((cell.col-start_col).rem_euclid(gc),(cell.row-start_row).rem_euclid(gr),cell.x,cell.y);
            let coverage=if self.soft_brush {self.brush_coverage.get(&(tile,cell.x,cell.y)).copied()}else{None};
            let priority=(outside as u8,coverage.unwrap_or(1.));
            let entry=canonical.entry(key).or_insert((new,coverage,priority));
            if priority>=entry.2 {*entry=(new,coverage,priority);}
        }
        let mut writes=Vec::new();
        for ((lc,lr,x,y),(color,coverage,_)) in canonical {
            for row in (0..self.rows).step_by(gr as usize) {for col in (0..self.columns).step_by(gc as usize) {
                let tile=self.tile_at(col as i32+lc,row as i32+lr);
                let result=if let Some(coverage)=coverage {self.blend_edit_pixel(*before.tiles[tile].get_pixel(x,y),Rgba(self.color),coverage,self.tool==Tool::Eraser,self.brush_mode)}else{color};
                writes.push((tile,x,y,result));
            }}
        }
        for (tile,x,y,color) in writes {self.write_pixel(tile,x,y,color);}
    }
    pub(super) fn subgroup_controls(&mut self,ui:&mut egui::Ui){
        if self.subgroup_size.iter().any(|v|*v==0) || self.columns%self.subgroup_size[0].max(1) as usize!=0 || self.rows%self.subgroup_size[1].max(1) as usize!=0 {self.finish_stroke();self.subgroup_size=[1,1];}
        ui.separator();
        egui::CollapsingHeader::new(egui::RichText::new(tr("子组绕回")).heading().strong()).id_salt("tile_subgroup_wrap").default_open(false).show(ui,|ui|{
            let supported=self.multiple && !self.elevated && self.hex==0;
            let mut enabled=self.subgroup_enabled;let mut size=self.subgroup_size;
            ui.add_enabled_ui(supported,|ui|{
                ui.checkbox(&mut enabled,tr("启用子组边缘联动"));
                ui.horizontal(|ui|{
                    for (axis,total,label) in [(0,self.columns,"列"),(1,self.rows,"行")] {
                        ui.label(tr(label));
                        egui::ComboBox::from_id_salt(("subgroup",axis)).selected_text(size[axis].to_string()).width(44.).show_ui(ui,|ui|{
                            for value in 1..=total {if total%value==0 {ui.selectable_value(&mut size[axis],value as u32,value.to_string());}}
                        });
                    }
                });
            });
            if !supported {ui.small(tr("子组绕回仅支持多格矩形或菱形布局。"));}
            if enabled && supported {
                let c=size[0].max(1) as usize;let r=size[1].max(1) as usize;
                ui.label(format!("{}: {} / {}",tr("当前子组"),(self.selected_slot/8/r)*(self.columns/c)+self.selected_slot%8/c+1,(self.columns/c)*(self.rows/r)));
            }
            ui.small(tr("选中格决定当前子组；边缘抬笔后联动，内部独立。"));
            if (enabled,size)!=(self.subgroup_enabled,self.subgroup_size) {self.finish_stroke();self.subgroup_enabled=enabled;self.subgroup_size=size;self.changed=true;}
        });
    }
    pub(super) fn subgroup_outline(&self,painter:&egui::Painter,origin:Pos2,scale:f32){
        if !self.subgroup_active(){return;}
        let gc=self.subgroup_size[0] as usize;let gr=self.subgroup_size[1] as usize;let (w,h)=self.pixels.dimensions();
        for row in (0..self.rows).step_by(gr){for col in (0..self.columns).step_by(gc){
            let base=self.mapped_center(col as i32,row as i32,w,h);
            let points=if self.diamond {vec![egui::vec2(0.,-(h as f32)/2.),egui::vec2(gc as f32*w as f32/2.,(gc as f32-1.)*h as f32/2.),egui::vec2((gc as f32-gr as f32)*w as f32/2.,(gc as f32+gr as f32-1.)*h as f32/2.),egui::vec2(-(gr as f32)*w as f32/2.,(gr as f32-1.)*h as f32/2.)]}else{vec![egui::vec2(-(w as f32)/2.,-(h as f32)/2.),egui::vec2((gc as f32-0.5)*w as f32,-(h as f32)/2.),egui::vec2((gc as f32-0.5)*w as f32,(gr as f32-0.5)*h as f32),egui::vec2(-(w as f32)/2.,(gr as f32-0.5)*h as f32)]};
            let active=self.selected_slot%8/gc==col/gc && self.selected_slot/8/gr==row/gr;
            painter.add(egui::Shape::closed_line(points.into_iter().map(|p|origin+(base+p)*scale).collect(),egui::Stroke::new(if active{2_f32}else{1_f32},if active{Color32::YELLOW}else{Color32::from_rgb(99,179,237)})));
        }}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn editor()->Editor {
        let mut e=Editor::default();e.diamond=false;e.reset_document(RgbaImage::new(32,16));
        e.multiple=true;e.columns=4;e.rows=4;e.independent_slots().unwrap();
        e.subgroup_enabled=true;e.subgroup_size=[2,2];e.tool=Tool::Brush;e.brush_size=3;e
    }
    #[test]
    fn interior_is_independent_but_subgroup_boundary_links_equivalent_cells() {
        let mut e=editor();e.plane_stroke((0,0),(0,0),true);e.finish_stroke();
        assert!(e.tiles[0].pixels().any(|p|p[3]!=0));
        assert!(e.tiles.iter().skip(1).all(|t|t.pixels().all(|p|p[3]==0)));
        let mut e=editor();e.plane_stroke((47,0),(47,0),true);e.finish_stroke();
        for row in [0,2] {for col in [0,2] {
            assert_eq!(e.tiles[e.tile_at(col,row)],e.tiles[e.tile_at(0,0)]);
            assert_eq!(e.tiles[e.tile_at(col+1,row)],e.tiles[e.tile_at(1,0)]);
        }}
        assert!(e.tiles[e.tile_at(0,0)].pixels().any(|p|p[3]!=0));
        e.history(false);assert!(e.tiles.iter().all(|t|t.pixels().all(|p|p[3]==0)));
    }
    #[test]
    fn soft_group_linkage_does_not_accumulate_alpha() {
        let mut e=editor();e.soft_brush=true;e.color=[80,180,100,128];e.brush_size=7;
        e.plane_stroke((47,0),(49,0),true);e.finish_stroke();
        assert!(e.tiles.iter().flat_map(|t|t.pixels()).all(|p|p[3]<=128));
    }
}
