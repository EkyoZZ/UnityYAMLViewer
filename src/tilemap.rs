//! Read-only, cached Tilemap data and virtualized inspectors.
use std::sync::Arc;
use eframe::egui::{self, RichText};
use crate::static_preview::{Matrix, reference};

#[derive(Default, Debug)]
pub struct Cell {
    pub position: [f64;3],
    pub tile: usize,
    pub sprite: usize,
    pub matrix: usize,
    pub color: usize,
    pub flags: u32,
}
pub struct Data {
    source: Arc<str>,
    lines: Vec<std::ops::Range<usize>>,
    pub properties: Vec<(String,String)>,
    pub cells: Vec<Cell>,
    pub assets: Vec<String>,
    pub sprites: Vec<String>,
    pub matrices: Vec<Matrix>,
    pub colors: Vec<[f64;4]>,
    pub anchor: [f64;3],
    pub tint: [f64;4],
    pub orientation: Matrix,
}
fn inline<const N: usize>(value: &str, axes: [&str;N], fallback: [f64;N]) -> [f64;N] {
    crate::inline_vector(value, &axes).and_then(|v| v.try_into().ok()).unwrap_or(fallback)
}
#[cfg(test)]
pub fn parse(text: &str) -> Data {
    parse_shared(Arc::from(text),None,true,None).unwrap()
}
fn parse_shared(source: Arc<str>, cancelled: Option<&std::sync::atomic::AtomicBool>, index_lines: bool, paused:Option<&std::sync::atomic::AtomicBool>) -> Option<Data> {
    let text=source.as_ref();
    let mut data = Data {source: source.clone(), lines: Vec::new(), properties: Vec::new(),cells:Vec::new(),assets:Vec::new(),sprites:Vec::new(),matrices:Vec::new(),colors:Vec::new(),anchor:[0.5,0.5,0.0],tint:[1.0;4],orientation:Matrix::identity()};
    let mut section = "";
    let mut offset = 0;
    for (line_index,line) in text.split_inclusive('\n').enumerate() {
        if line_index%1024==0 && !crate::static_preview::checkpoint(cancelled.unwrap_or(&std::sync::atomic::AtomicBool::new(false)),paused) {return None;}
        if cancelled.is_some_and(|flag|flag.load(std::sync::atomic::Ordering::Relaxed)) {return None;}
        if index_lines {data.lines.push(offset..offset+line.trim_end_matches(['\n','\r']).len());}
        offset += line.len();
        if let Some(top) = line.strip_prefix("  ").filter(|s| !s.starts_with([' ', '-'])) {
            if let Some((key,value)) = top.trim().split_once(':') {
                section = key;
                if !value.trim().is_empty() && !key.ends_with("Array") && key != "m_Tiles" {
                    data.properties.push((key.to_owned(),value.trim().to_owned()));
                }
                match key {
                    "m_TileAnchor" => data.anchor=inline(value,["x","y","z"],data.anchor),
                    "m_Color" => data.tint=inline(value,["r","g","b","a"],data.tint),
                    _=>{}
                }
            }
        }
        let trimmed=line.trim();
        if section=="m_Tiles" {
            if let Some(value)=trimmed.strip_prefix("- first:") {
                data.cells.push(Cell{position:inline(value,["x","y","z"],[0.0;3]),..Default::default()});
            } else if let (Some(cell),Some((key,value)))=(data.cells.last_mut(),trimmed.split_once(':')) {
                let index=value.trim().parse().unwrap_or(usize::MAX);
                match key {"m_TileIndex"=>cell.tile=index,"m_TileSpriteIndex"=>cell.sprite=index,"m_TileMatrixIndex"=>cell.matrix=index,"m_TileColorIndex"=>cell.color=index,"m_AllTileFlags"=>cell.flags=value.trim().parse().unwrap_or(0),_=>{}}
            }
        } else if matches!(section,"m_TileAssetArray"|"m_TileSpriteArray") {
            if let Some(value)=trimmed.strip_prefix("m_Data:") {
                if section=="m_TileAssetArray" {data.assets.push(value.trim().into());} else {data.sprites.push(value.trim().into());}
            }
        } else if section=="m_TileColorArray" {
            if let Some(value)=trimmed.strip_prefix("m_Data:") {data.colors.push(inline(value,["r","g","b","a"],[1.0;4]));}
        } else if section=="m_TileMatrixArray" || section=="m_TileOrientationMatrix" {
            if trimmed.starts_with("- m_RefCount:") {data.matrices.push(Matrix::identity());}
            if let Some((key,value))=trimmed.split_once(':') {
                let bytes=key.as_bytes();
                if bytes.len()==3 && bytes[0]==b'e' && (b'0'..=b'3').contains(&bytes[1]) && (b'0'..=b'3').contains(&bytes[2]) {
                    let matrix=if section=="m_TileOrientationMatrix" {Some(&mut data.orientation)} else {data.matrices.last_mut()};
                    if let Some(matrix)=matrix {matrix.0[(bytes[1]-b'0') as usize][(bytes[2]-b'0') as usize]=value.trim().parse::<f64>().ok().filter(|v|v.is_finite()).unwrap_or(0.0);}
                }
            }
        }
    }
    Some(data)
}
pub fn cached(ctx: &egui::Context, id: &str, text: &Arc<str>) -> Arc<Data> {
    let key=egui::Id::new(("tilemap_data",id,text.as_ptr() as usize));
    if let Some(data)=ctx.data(|d|d.get_temp::<Arc<Data>>(key)) {
        if Arc::ptr_eq(&data.source,text) {return data;}
    }
    let data=Arc::new(parse_shared(text.clone(),None,true,None).unwrap());
    ctx.data_mut(|d|d.insert_temp(key,data.clone()));
    data
}
pub fn preview_data(text: &Arc<str>, cancelled: &std::sync::atomic::AtomicBool, paused:Option<&std::sync::atomic::AtomicBool>) -> Option<Data> {
    parse_shared(text.clone(),Some(cancelled),false,paused)
}
pub fn forget(ctx: &egui::Context, id: &str, text: &Arc<str>) {
    ctx.data_mut(|d|d.remove::<Arc<Data>>(egui::Id::new(("tilemap_data",id,text.as_ptr() as usize))));
}
pub fn show(ui: &mut egui::Ui, data: &Data, yaml: bool) {
    if yaml {
        egui::ScrollArea::both().id_salt("tilemap_yaml").max_height(500.0).show_rows(ui,18.0,data.lines.len(),|ui,rows| {
            for row in rows {ui.add(egui::Label::new(RichText::new(&data.source[data.lines[row].clone()]).monospace()).wrap_mode(egui::TextWrapMode::Extend));}
        });
        return;
    }
    let hide_types=ui.ctx().data(|data|data.get_temp::<bool>(egui::Id::new("inspector_hide_property_types"))).unwrap_or(false);
    egui::ScrollArea::horizontal().id_salt(("tilemap_property_scroll",hide_types)).auto_shrink([false,true]).show(ui,|ui| {
    egui::Grid::new(("tilemap_properties",hide_types)).num_columns(if hide_types {2} else {3}).min_col_width(160.0).striped(false).show(ui,|ui| {
        for (key,value) in &data.properties {
            ui.add(egui::Label::new(key.strip_prefix("m_").unwrap_or(key)).wrap_mode(egui::TextWrapMode::Extend));
            if !hide_types {
                let kind=match key.as_str() {
                    "m_Origin"|"m_Size"|"m_ChunkSize"=>"Vector3Int",
                    "m_TileAnchor"|"m_CellSize"|"m_CellGap"|"m_ChunkCullingBounds"=>"Vector3",
                    "m_Color"=>"Color",
                    "m_CellLayout"|"m_CellSwizzle"|"m_TileOrientation"|"m_Mode"|"m_SortOrder"=>"Enum",
                    "m_AnimationFrameRate"=>"float",
                    "m_Enabled"=>"bool",
                    _ if value.contains("fileID:")=>"Object reference",
                    _ if value.starts_with('{')=>"Object",
                    _ if value.parse::<i64>().is_ok()=>"int",
                    _ if value.parse::<f64>().is_ok()=>"float",
                    _=>"Value",
                };
                ui.add(egui::Label::new(RichText::new(kind).small().weak()).wrap_mode(egui::TextWrapMode::Extend));
            }
            if key=="m_Color" {
                let color=inline(value,["r","g","b","a"],[1.0;4]);
                let (rect,response)=ui.allocate_exact_size(egui::vec2(120.0,20.0),egui::Sense::hover());
                crate::color_property::paint_swatch(ui,rect,color);
                response.on_hover_text(value);
            } else if matches!(key.as_str(),"m_Origin"|"m_Size"|"m_ChunkSize"|"m_TileAnchor"|"m_CellSize"|"m_CellGap"|"m_ChunkCullingBounds") {
                let vector=inline(value,["x","y","z"],[0.0;3]);
                ui.horizontal(|ui| {
                    for (axis,number) in ["x","y","z"].into_iter().zip(vector) {
                        ui.label(axis);readonly_box(ui,&number.to_string(),70.0);
                    }
                });
            } else if key=="m_CellLayout" {
                let name=match value.as_str() {"0"=>"Rectangle","1"=>"Hexagon","2"=>"Isometric (Diamond)","3"=>"Isometric Z as Y",_=>"Unknown"};
                readonly_box(ui,&format!("{value} · {name}"),300.0);
            } else {readonly_box(ui,value,300.0);}
            ui.end_row();
        }
    });
    });
    ui.small(format!("Tiles: {} · Tile assets: {} · Sprites: {}",data.cells.len(),data.assets.len(),data.sprites.len()));
    ui.small(crate::localization::tr("只读 Inspector"));
    ui.collapsing(format!("Tiles ({})",data.cells.len()),|ui| {
        egui::ScrollArea::horizontal().id_salt("tilemap_table_horizontal").auto_shrink([false,true]).show(ui,|ui| {
            use egui_extras::{TableBuilder,Column};
            TableBuilder::new(ui).id_salt("tilemap_cells").striped(false).resizable(true)
                .column(Column::initial(48.0)).columns(Column::initial(64.0),7).column(Column::initial(120.0))
                .min_scrolled_height(160.0).max_scroll_height(320.0)
                .header(24.0,|mut row| {
                    for title in ["Index","X","Y","Z","Tile","Sprite","Matrix","Color","Flags"] {
                        row.col(|ui| {ui.strong(title);});
                    }
                }).body(|body|body.rows(24.0,data.cells.len(),|mut row| {
                    let index=row.index();let cell=&data.cells[index];
                    let values=[index.to_string(),cell.position[0].to_string(),cell.position[1].to_string(),cell.position[2].to_string(),cell.tile.to_string(),cell.sprite.to_string(),cell.matrix.to_string(),cell.color.to_string(),cell.flags.to_string()];
                    for value in values {row.col(|ui| {readonly_box(ui,&value,ui.available_width());});}
                }));
        });
    });
    for (name,items) in [("TileAssetArray",&data.assets),("TileSpriteArray",&data.sprites)] {
        ui.collapsing(format!("{name} ({})",items.len()),|ui| {
            egui::ScrollArea::both().id_salt(name).auto_shrink([false,false]).min_scrolled_height(120.0).max_height(240.0).show_rows(ui,20.0,items.len(),|ui,rows| {
                for index in rows {ui.horizontal(|ui| {ui.label(index.to_string());readonly_box(ui,&items[index],500.0);});}
            });
        });
    }
    ui.collapsing(format!("TileMatrixArray ({})",data.matrices.len()),|ui| {
        egui::ScrollArea::both().id_salt("tile_matrices").max_height(240.0).show_rows(ui,20.0,data.matrices.len(),|ui,rows| {
            for index in rows {ui.monospace(format!("{index}: {:?}",data.matrices[index].0));}
        });
    });
    ui.collapsing(format!("TileColorArray ({})",data.colors.len()),|ui| {
        egui::ScrollArea::both().id_salt("tile_colors").max_height(240.0).show_rows(ui,20.0,data.colors.len(),|ui,rows| {
            for index in rows {ui.horizontal(|ui| {
                ui.label(index.to_string());
                let (rect,response)=ui.allocate_exact_size(egui::vec2(120.0,20.0),egui::Sense::hover());
                crate::color_property::paint_swatch(ui,rect,data.colors[index]);
                response.on_hover_text(format!("RGBA: {:?}",data.colors[index]));
            });}
        });
    });
}

fn readonly_box(ui: &mut egui::Ui, value: &str, width: f32) {
    let mut text=value;
    ui.add(egui::TextEdit::singleline(&mut text).desired_width(width));
}

pub fn cell_position(position: [f64;3], anchor: [f64;3], size: [f64;3], gap: [f64;3], layout: i32, swizzle: usize) -> Option<[f64;3]> {
    let permutations=[[0,1,2],[0,2,1],[1,0,2],[1,2,0],[2,0,1],[2,1,0]];
    let permutation=permutations.get(swizzle)?;
    let p=std::array::from_fn::<_,3,_>(|i|position[permutation[i]]+anchor[permutation[i]]);
    let step=std::array::from_fn::<_,3,_>(|i|size[i]+gap[i].max(-size[i]));
    match layout {
        0=>Some([p[0]*step[0],p[1]*step[1],p[2]*step[2]]),
        2=>Some([(p[0]-p[1])*step[0]*0.5,(p[0]+p[1])*step[1]*0.5,p[2]*step[2]]),
        3=>Some([(p[0]-p[1])*step[0]*0.5,(p[0]+p[1])*step[1]*0.5+p[2]*step[2],0.0]),
        _=>None,
    }
}
pub fn sprite_reference(value: &str) -> Option<(String,String)> {
    let id=reference(value,"fileID").filter(|id|id!="0")?;
    Some((reference(value,"guid")?.to_lowercase(),id))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_cells_and_preserves_null_reference_indices() {
        let data=parse("Tilemap:\n  m_Tiles:\n  - first: {x: -2, y: 3, z: 0}\n    second:\n      m_TileIndex: 1\n      m_TileSpriteIndex: 1\n      m_TileMatrixIndex: 0\n      m_TileColorIndex: 0\n  m_TileSpriteArray:\n  - m_RefCount: 0\n    m_Data: {fileID: 0}\n  - m_RefCount: 1\n    m_Data: {fileID: 213, guid: abc, type: 3}\n  m_TileColorArray:\n  - m_RefCount: 1\n    m_Data: {r: 1, g: 0.5, b: 1, a: 0.25}\n");
        assert_eq!(data.cells.len(),1);assert_eq!(data.cells[0].position,[-2.0,3.0,0.0]);assert_eq!(data.cells[0].sprite,1);assert_eq!(data.sprites.len(),2);assert_eq!(data.colors[0],[1.0,0.5,1.0,0.25]);
    }
    #[test]
    fn isometric_grid_uses_cell_size_and_anchor() {
        assert_eq!(cell_position([1.0,0.0,0.0],[0.5,0.5,0.0],[128.0,64.0,0.0],[0.0;3],2,0),Some([64.0,64.0,0.0]));
        assert_eq!(cell_position([0.0;3],[0.0;3],[1.0;3],[0.0;3],1,0),None);
    }
}
