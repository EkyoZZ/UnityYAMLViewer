//! Local mask-guided repair; no image is uploaded or source file overwritten.
use super::{image_history::History, sequence::ColorKey};
use crate::localization::tr;
use eframe::egui;
use image::RgbaImage;
use std::{
    collections::{HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
};
mod matching;
const MAX_PIXELS: usize = 16_777_216;
const MAX_MASK: usize = 1_048_576;
const HISTORY_BYTES: usize = 256 * 1024 * 1024;
#[derive(Clone, PartialEq)]
struct State {
    image: Arc<RgbaImage>,
    mask: Arc<Vec<u8>>,
}
fn selected(mask: &[u8], index: usize) -> bool {
    mask[index / 8] & (1 << (index % 8)) != 0
}
fn mark(mask: &mut [u8], index: usize, value: bool) {
    if value {
        mask[index / 8] |= 1 << (index % 8)
    } else {
        mask[index / 8] &= !(1 << (index % 8))
    }
}
fn any(mask: &[u8]) -> bool {
    mask.iter().any(|&byte| byte != 0)
}
fn neighbors(index: usize, w: usize, h: usize) -> impl Iterator<Item = usize> {
    let x = index % w;
    let y = index / w;
    [
        x.checked_sub(1).map(|_| index - 1),
        (x + 1 < w).then_some(index + 1),
        y.checked_sub(1).map(|_| index - w),
        (y + 1 < h).then_some(index + w),
    ]
    .into_iter()
    .flatten()
}
fn premul(pixel: image::Rgba<u8>) -> [f32; 4] {
    let a = pixel[3] as f32 / 255.;
    [
        pixel[0] as f32 * a,
        pixel[1] as f32 * a,
        pixel[2] as f32 * a,
        pixel[3] as f32,
    ]
}
/// Fill from the nearest unmasked boundary, then diffuse only inside the mask.
fn repair(source: &RgbaImage, mask: &[u8]) -> Result<RgbaImage, String> {
    let (w, h) = (source.width() as usize, source.height() as usize);
    let n = w * h;
    if n == 0 || n > MAX_PIXELS || mask.len() != n.div_ceil(8) {
        return Err(tr("图片过大或蒙版无效。").into());
    }
    let indices: Vec<_> = (0..n).filter(|&i| selected(mask, i)).collect();
    if indices.is_empty() {
        return Err(tr("请先涂抹水印区域。").into());
    }
    if indices.len() > MAX_MASK {
        return Err(tr("单次修补区域最多 1048576 像素，请分步处理。").into());
    }
    if indices.len() == n {
        return Err(tr("请保留未标记的周围像素用于修补。").into());
    }
    let mut slots = vec![usize::MAX; n];
    for (slot, &i) in indices.iter().enumerate() {
        slots[i] = slot;
    }
    let mut filled = vec![false; indices.len()];
    let mut queued = vec![false; indices.len()];
    let mut values = vec![[0.; 4]; indices.len()];
    let mut queue = VecDeque::new();
    for (slot, &i) in indices.iter().enumerate() {
        if neighbors(i, w, h).any(|j| slots[j] == usize::MAX) {
            queue.push_back(i);
            queued[slot] = true;
        }
    }
    while let Some(i) = queue.pop_front() {
        let slot = slots[i];
        let mut sum = [0.; 4];
        let mut count = 0.;
        for j in neighbors(i, w, h) {
            let other = slots[j];
            let value = if other == usize::MAX {
                Some(premul(*source.get_pixel((j % w) as u32, (j / w) as u32)))
            } else if filled[other] {
                Some(values[other])
            } else {
                None
            };
            if let Some(value) = value {
                for c in 0..4 {
                    sum[c] += value[c];
                }
                count += 1.;
            }
        }
        if count == 0. {
            continue;
        }
        for c in 0..4 {
            values[slot][c] = sum[c] / count;
        }
        filled[slot] = true;
        for j in neighbors(i, w, h) {
            let next = slots[j];
            if next != usize::MAX && !queued[next] {
                queue.push_back(j);
                queued[next] = true;
            }
        }
    }
    if filled.iter().any(|v| !v) {
        return Err(tr("请保留未标记的周围像素用于修补。").into());
    }
    let mut next = values.clone();
    for _ in 0..80 {
        for (slot, &i) in indices.iter().enumerate() {
            let mut sum = [0.; 4];
            let mut count = 0.;
            for j in neighbors(i, w, h) {
                let value = if slots[j] == usize::MAX {
                    premul(*source.get_pixel((j % w) as u32, (j / w) as u32))
                } else {
                    values[slots[j]]
                };
                for c in 0..4 {
                    sum[c] += value[c];
                }
                count += 1.;
            }
            for c in 0..4 {
                next[slot][c] = sum[c] / count;
            }
        }
        std::mem::swap(&mut values, &mut next);
    }
    let mut output = source.clone();
    for (slot, &i) in indices.iter().enumerate() {
        let value = values[slot];
        let a = value[3].clamp(0., 255.);
        let mut p = [0; 4];
        p[3] = a.round() as u8;
        if a > 0. {
            for c in 0..3 {
                p[c] = (value[c] * 255. / a).round().clamp(0., 255.) as u8;
            }
        }
        output.put_pixel((i % w) as u32, (i / w) as u32, image::Rgba(p));
    }
    Ok(output)
}
fn cache_save(image: &RgbaImage, mask: &[u8], folder: &Path) -> Result<PathBuf, String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let _guard = crate::app_cache::mutation_guard();
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let target = folder.join(format!("result_{id:06}.png"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .map_err(|e| e.to_string())?;
    if let Err(error) = image.write_to(&mut file, image::ImageFormat::Png) {
        drop(file);
        let _ = std::fs::remove_file(&target);
        return Err(error.to_string());
    }
    if any(mask) {
        let pixels = (0..image.width() as usize * image.height() as usize)
            .map(|i| if selected(mask, i) { 255 } else { 0 })
            .collect();
        let selection = image::GrayImage::from_raw(image.width(), image.height(), pixels)
            .ok_or("Invalid mask")?;
        if let Err(error) = selection.save(folder.join(format!("mask_{id:06}.png"))) {
            return Err(format!("{}: {error}", target.display()));
        }
    }
    Ok(target)
}
struct Completed {
    image: Option<Arc<RgbaImage>>,
    clear_mask: bool,
    path: Option<PathBuf>,
    mask: Option<Arc<Vec<u8>>>,
    matches: Option<usize>,
}
pub struct Editor {
    pub open: bool,
    base: Option<Arc<RgbaImage>>,
    state: Option<State>,
    history: History<State>,
    stroke: Option<State>,
    last: Option<egui::Pos2>,
    pending: Option<mpsc::Receiver<Result<Completed, String>>>,
    before_job: Option<State>,
    texture: Option<egui::TextureHandle>,
    mask_texture: Option<egui::TextureHandle>,
    dirty: bool,
    mask_dirty: bool,
    similarity: u32,
    brush: u32,
    erase: bool,
    show_mask: bool,
    compare: bool,
    zoom: f32,
    pan: egui::Vec2,
    key: ColorKey,
    picking: bool,
    background: super::ImageBackground,
    background_color: Option<[u8; 3]>,
    folder: Option<PathBuf>,
    message: String,
}
impl Default for Editor {
    fn default() -> Self {
        Self {
            open: false,
            base: None,
            state: None,
            history: Default::default(),
            stroke: None,
            last: None,
            pending: None,
            before_job: None,
            texture: None,
            mask_texture: None,
            dirty: true,
            mask_dirty: true,
            similarity: 95,
            brush: 12,
            erase: false,
            show_mask: true,
            compare: false,
            zoom: 1.,
            pan: egui::Vec2::ZERO,
            key: Default::default(),
            picking: false,
            background: Default::default(),
            background_color: None,
            folder: None,
            message: String::new(),
        }
    }
}
impl Editor {
    pub fn active(&self) -> bool {
        self.open || self.pending.is_some()
    }
    fn record(&mut self, before: State) {
        if let Some(state) = &self.state {
            self.history.record(before, state);
        }
        self.trim_memory();
    }
    fn trim_memory(&mut self) {
        self.history.trim();
        loop {
            let mut seen = HashSet::new();
            let mut bytes = 0;
            for s in self
                .state
                .iter()
                .chain(self.history.undo.iter())
                .chain(self.history.redo.iter())
            {
                if seen.insert((0, Arc::as_ptr(&s.image) as usize)) {
                    bytes += s.image.as_raw().len();
                }
                if seen.insert((1, Arc::as_ptr(&s.mask) as usize)) {
                    bytes += s.mask.len();
                }
            }
            if bytes <= HISTORY_BYTES {
                break;
            }
            if self.history.undo.pop_front().is_none() {
                if self.history.redo.is_empty() {
                    break;
                }
                self.history.redo.remove(0);
            }
        }
    }
    fn restore(&mut self, state: State) {
        self.state = Some(state);
        self.dirty = true;
        self.mask_dirty = true;
        self.picking = false;
        self.compare = false;
    }
    fn finish_stroke(&mut self) {
        if let Some(before) = self.stroke.take() {
            self.record(before);
        }
        self.last = None;
    }
    fn paint(&mut self, point: egui::Pos2) {
        let Some(state) = &mut self.state else { return };
        let w = state.image.width() as usize;
        let h = state.image.height() as usize;
        let radius = self.brush as f32;
        let start = self.last.unwrap_or(point);
        let distance = start.distance(point);
        let steps = (distance / (radius * 0.4).max(1.)).ceil().max(1.) as u32;
        let mask = Arc::make_mut(&mut state.mask);
        for step in 0..=steps {
            let center = start.lerp(point, step as f32 / steps as f32);
            let x0 = (center.x - radius).floor().max(0.) as usize;
            let x1 = (center.x + radius).ceil().max(0.) as usize;
            let y0 = (center.y - radius).floor().max(0.) as usize;
            let y1 = (center.y + radius).ceil().max(0.) as usize;
            for y in y0..=y1.min(h - 1) {
                for x in x0..=x1.min(w - 1) {
                    if egui::pos2(x as f32 + 0.5, y as f32 + 0.5).distance_sq(center)
                        <= radius * radius
                    {
                        mark(mask, y * w + x, !self.erase);
                    }
                }
            }
        }
        self.last = Some(point);
        self.mask_dirty = true;
    }
    fn start_job(&mut self, kind: u8, ctx: &egui::Context) {
        self.finish_stroke();
        let Some(state) = self.state.clone() else {
            return;
        };
        let Some(folder) = self.folder.clone() else {
            self.message = tr("缓存目录不可用。").into();
            return;
        };
        let key = self.key;
        let similarity = self.similarity;
        if kind == 1 && !key.enabled {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.before_job = Some(state.clone());
        let ctx = ctx.clone();
        self.message.clear();
        std::thread::spawn(move || {
            let result = (|| {
                let (matched_mask, matches) = if kind == 3 || kind == 4 {
                    let (mask, count) = matching::find(&state.image, &state.mask, similarity)?;
                    (Some(Arc::new(mask)), Some(count))
                } else {
                    (None, None)
                };
                let used_mask = matched_mask.as_deref().unwrap_or(&state.mask);
                let image = match kind {
                    0 | 4 => Some(Arc::new(repair(&state.image, used_mask)?)),
                    1 => {
                        let mut image = state.image.as_ref().clone();
                        key.apply(&mut image, false);
                        Some(Arc::new(image))
                    }
                    _ => None,
                };
                let path = if kind == 3 {
                    None
                } else {
                    Some(cache_save(
                        image.as_deref().unwrap_or(&state.image),
                        used_mask,
                        &folder,
                    )?)
                };
                Ok(Completed {
                    image,
                    clear_mask: kind == 0 || kind == 4,
                    path,
                    mask: if kind == 3 { matched_mask } else { None },
                    matches,
                })
            })();
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
    pub fn show(&mut self, ctx: &egui::Context, path: &Path, original: Option<&Arc<RgbaImage>>) {
        let Some(original) = original.or(self.base.as_ref()).cloned() else {
            return;
        };
        if let Some(result) = self.pending.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(v) => Some(v),
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err(tr("图片处理任务中断，请重试。").into()))
            }
            Err(_) => None,
        }) {
            self.pending = None;
            match result {
                Ok(done) => {
                    if let Some(image) = done.image {
                        let mask = if done.clear_mask {
                            Arc::new(vec![0; image.as_raw().len().div_ceil(4).div_ceil(8)])
                        } else {
                            self.state.as_ref().unwrap().mask.clone()
                        };
                        self.restore(State { image, mask });
                        if let Some(before) = self.before_job.take() {
                            self.record(before);
                        }
                    }
                    if let Some(mask) = done.mask {
                        let image = self.state.as_ref().unwrap().image.clone();
                        self.restore(State { image, mask });
                        self.show_mask = true;
                        if let Some(before) = self.before_job.take() {
                            self.record(before);
                        }
                    }
                    self.before_job = None;
                    self.message = done.matches.map_or(String::new(), |count| {
                        format!("{}: {count}", tr("匹配位置数量"))
                    });
                    if let Some(path) = done.path {
                        self.message
                            .push_str(&format!("\n{}: {}", tr("已缓存"), path.display()));
                    }
                }
                Err(error) => {
                    self.before_job = None;
                    self.message = error
                }
            }
        }
        if self.pending.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        if !self.open {
            return;
        }
        let count = original.width() as usize * original.height() as usize;
        if self.state.is_none() && count > 0 && count <= MAX_PIXELS {
            self.base = Some(original.clone());
            self.state = Some(State {
                image: original.clone(),
                mask: Arc::new(vec![0; count.div_ceil(8)]),
            });
            let id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            self.folder = crate::app_cache::root().map(|root| {
                root.join("watermark_cache")
                    .join(format!("{}-{id}", std::process::id()))
            });
        }
        let mut open = self.open;
        crate::native_window::Window::new(format!("{} · {}",tr("去水印"),path.file_name().unwrap_or_default().to_string_lossy())).id(egui::Id::new(("watermark",path))).open(&mut open).default_width(1100.).default_height(740.).min_width(740.).scrollable(false).show(ctx,|ui|{
            if self.state.is_none(){ui.label(tr("图片过大或蒙版无效。"));return}
            let busy=self.pending.is_some();let mut action=if busy{0}else{super::image_history::shortcut(ui)};
            ui.horizontal_wrapped(|ui|{
                self.background.controls(ui,&mut self.background_color);
                if ui.add_enabled(!busy&&!self.history.undo.is_empty(),egui::Button::new(tr("撤销"))).on_hover_text("Ctrl+Z").clicked(){action=-1;}
                if ui.add_enabled(!busy&&!self.history.redo.is_empty(),egui::Button::new(tr("重做"))).on_hover_text("Ctrl+Y / Ctrl+Shift+Z").clicked(){action=1;}
                ui.small(format!("{} / {}",self.history.undo.len(),super::image_history::limit()));
                ui.add_enabled_ui(!busy,|ui|{ui.checkbox(&mut self.compare,tr("查看原图"));if ui.button(tr("适应窗口")).clicked(){self.zoom=1.;self.pan=egui::Vec2::ZERO;}});
                if ui.add_enabled(!busy,egui::Button::new(tr("保存到缓存"))).clicked(){self.start_job(2,ui.ctx());}
                if let Some(folder)=&self.folder{if ui.button(tr("打开缓存目录")).clicked(){if let Err(e)=std::fs::create_dir_all(folder).and_then(|_|crate::media_tools::command("explorer.exe").arg(folder).spawn().map(|_|())){self.message=e.to_string();}}}
            });
            if action!=0{self.finish_stroke();let current=self.state.as_ref().unwrap();let restored=if action<0{self.history.undo(current)}else{self.history.redo(current)};if let Some(state)=restored{self.restore(state);}}
            egui::SidePanel::right(ui.id().with("watermark_controls")).default_width(285.).resizable(true).show_inside(ui,|ui|{
                egui::ScrollArea::vertical().show(ui,|ui|{
                ui.add_enabled_ui(self.pending.is_none(),|ui|{
                    ui.heading(tr("标记水印"));ui.small(tr("左键涂抹水印，右键拖动平移，滚轮缩放；红色为待修补区域。"));
                    ui.horizontal(|ui|{ui.label(tr("画笔半径"));ui.add(egui::DragValue::new(&mut self.brush).range(1..=160).suffix(" px"));});ui.checkbox(&mut self.erase,tr("橡皮擦"));ui.checkbox(&mut self.show_mask,tr("显示蒙版"));
                    if ui.button(tr("清除蒙版")).clicked(){self.finish_stroke();let before=self.state.as_ref().unwrap().clone();Arc::make_mut(&mut self.state.as_mut().unwrap().mask).fill(0);self.mask_dirty=true;self.record(before);}
                    if ui.add_enabled(any(&self.state.as_ref().unwrap().mask),egui::Button::new(tr("修补蒙版区域"))).clicked(){self.start_job(0,ui.ctx());}
                    ui.small(tr("按周围像素修补，不自动识别文字或恢复原画；复杂区域可分步处理。"));
                    ui.separator();ui.heading(tr("相似水印"));
                    ui.horizontal(|ui|{ui.label(tr("相似度"));ui.add(egui::DragValue::new(&mut self.similarity).range(70..=99).suffix(" %"));});
                    ui.small(tr("先只涂一个完整水印样本；同尺寸匹配，降低相似度可能误选。"));
                    ui.add_enabled_ui(any(&self.state.as_ref().unwrap().mask),|ui|{
                        if ui.button(tr("查找相似水印")).clicked(){self.start_job(3,ui.ctx());}
                        if ui.button(tr("一键去除相似水印")).clicked(){self.start_job(4,ui.ctx());}
                    });
                    ui.small(tr("查找后检查红色蒙版，用橡皮擦修正，再点修补蒙版区域；一键功能直接从单一样本处理，可撤销。"));

                    ui.separator();ui.checkbox(&mut self.key.enabled,tr("去底色"));
                    ui.add_enabled_ui(self.key.enabled,|ui|{
                        ui.horizontal(|ui|{ui.color_edit_button_srgb(&mut self.key.color);let label=if self.picking{"取消吸色"}else{"吸取底色"};ui.toggle_value(&mut self.picking,tr(label));});
                        if self.picking{ui.colored_label(egui::Color32::LIGHT_BLUE,tr("单击取色 · Esc 取消"));}
                        ui.horizontal(|ui|{ui.label(tr("容差"));ui.add(egui::DragValue::new(&mut self.key.tolerance).range(0..=255));});ui.horizontal(|ui|{ui.label(tr("边缘柔化"));ui.add(egui::DragValue::new(&mut self.key.softness).range(0..=255));});ui.checkbox(&mut self.key.unmix,tr("去除边缘底色混合"));
                        if ui.button(tr("应用去底色")).clicked(){self.start_job(1,ui.ctx());}
                    });
                    ui.small(tr("去底色应用于整图；同色主体也可能被移除。"));
                    ui.separator();
                    ui.small(tr("每次修补或去底色自动保存 PNG 到缓存，原文件不修改。"));
                    ui.small(tr("历史受设置步数和 256 MiB 内存限制；缓存结果保留，清空历史不删除结果。"));
                });
                if self.pending.is_some(){ui.spinner();}if !self.message.is_empty(){ui.label(&self.message);}
                });
            });
            if self.picking&&ui.input(|i|i.key_pressed(egui::Key::Escape)){self.picking=false;}
            if !self.key.enabled{self.picking=false;}
            let image=if self.compare{self.base.as_ref().unwrap().clone()}else{self.state.as_ref().unwrap().image.clone()};
            let display=if self.dirty||self.texture.is_none(){true}else{self.texture.as_ref().unwrap().name()!=if self.compare{"watermark_original"}else{"watermark_result"}};
            if display{let thumb=image::imageops::thumbnail(image.as_ref(),2048,2048);let pixels=thumb.pixels().map(|p|super::preview_color(p.0)).collect();self.texture=Some(ui.ctx().load_texture(if self.compare{"watermark_original"}else{"watermark_result"},egui::ColorImage{size:[thumb.width() as usize,thumb.height() as usize],pixels},egui::TextureOptions::NEAREST));self.dirty=false;}
            let (canvas,response)=ui.allocate_exact_size(ui.available_size().max(egui::vec2(1.,1.)),egui::Sense::click_and_drag());
            if response.dragged_by(egui::PointerButton::Secondary){self.pan+=response.drag_delta();}
            if response.hovered(){let delta=ui.input(|i|i.smooth_scroll_delta.y);if delta!=0.{self.zoom=(self.zoom*(delta*0.002).exp()).clamp(0.1,64.);}}
            let scale=(canvas.width()/image.width() as f32).min(canvas.height()/image.height() as f32)*self.zoom;
            let rect=egui::Rect::from_center_size(canvas.center()+self.pan,egui::vec2(image.width() as f32,image.height() as f32)*scale);
            let pointer=response.interact_pointer_pos().or_else(||response.hover_pos()).filter(|p|rect.contains(*p));
            if !busy&&!self.compare&&action==0{if let Some(pos)=pointer{let point=egui::pos2((pos.x-rect.left())/scale,(pos.y-rect.top())/scale);
                if self.picking{ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);let p=image.get_pixel((point.x as u32).min(image.width()-1),(point.y as u32).min(image.height()-1));if response.clicked(){self.key.color=[p[0],p[1],p[2]];self.picking=false;}}
                else if response.is_pointer_button_down_on()&&ui.input(|i|i.pointer.button_down(egui::PointerButton::Primary)){if self.stroke.is_none(){self.stroke=self.state.clone();}self.paint(point);}
            }}
            if pointer.is_none(){self.last=None;}
            if self.stroke.is_some()&&!ui.input(|i|i.pointer.button_down(egui::PointerButton::Primary)){self.finish_stroke();}
            let painter=ui.painter().with_clip_rect(canvas.intersect(ui.clip_rect()));super::seamless::checker_background(&painter,rect.intersect(canvas),rect.min,egui::Vec2::splat(scale),&self.background,self.background_color);
            painter.image(self.texture.as_ref().unwrap().id(),rect,egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.,1.)),egui::Color32::WHITE);
            if self.mask_dirty||self.mask_texture.is_none(){let size=self.texture.as_ref().unwrap().size();let mut colors=vec![egui::Color32::TRANSPARENT;size[0]*size[1]];let mask=&self.state.as_ref().unwrap().mask;let w=image.width() as usize;let h=image.height() as usize;
                for (byte,&bits) in mask.iter().enumerate(){if bits!=0{for bit in 0..8{let i=byte*8+bit;if i<w*h&&bits&(1<<bit)!=0{let x=(i%w)*size[0]/w;let y=(i/w)*size[1]/h;colors[y*size[0]+x]=egui::Color32::from_rgba_unmultiplied(255,30,30,115);}}}}
                let pixels=egui::ColorImage{size,pixels:colors};if let Some(t)=&mut self.mask_texture{t.set(pixels,egui::TextureOptions::NEAREST);}else{self.mask_texture=Some(ui.ctx().load_texture("watermark_mask",pixels,egui::TextureOptions::NEAREST));}self.mask_dirty=false;}
            if self.show_mask&&!self.compare{painter.image(self.mask_texture.as_ref().unwrap().id(),rect,egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(1.,1.)),egui::Color32::WHITE);}
            if !busy&&!self.compare&&!self.picking{if let Some(pos)=pointer{ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);painter.circle_stroke(pos,self.brush as f32*scale,egui::Stroke::new(1_f32,egui::Color32::WHITE));}}
        });
        self.open = open;
        if !self.open {
            self.finish_stroke();
            self.picking = false;
            self.texture = None;
            self.mask_texture = None;
        }
        self.trim_memory();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repairs_only_masked_pixels_and_preserves_constant_background() {
        let mut image = RgbaImage::from_pixel(7, 5, image::Rgba([25, 50, 75, 128]));
        let mut mask = vec![0; 35_usize.div_ceil(8)];
        for (x, y) in [(2, 2), (3, 2), (5, 3)] {
            image.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
            mark(&mut mask, (y * 7 + x) as usize, true);
        }
        let result = repair(&image, &mask).unwrap();
        for i in 0..35 {
            let p = *result.get_pixel((i % 7) as u32, (i / 7) as u32);
            if selected(&mask, i) {
                assert_eq!(p.0, [25, 50, 75, 128]);
            } else {
                assert_eq!(p, *image.get_pixel((i % 7) as u32, (i / 7) as u32));
            }
        }
    }
    #[test]
    fn rejects_empty_or_full_mask() {
        let image = RgbaImage::from_pixel(3, 3, image::Rgba([10, 20, 30, 255]));
        assert!(repair(&image, &[0, 0]).is_err());
        assert!(repair(&image, &[255, 1]).is_err());
    }
    #[test]
    fn history_restores_both_mask_and_repaired_image() {
        let image = Arc::new(RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 0, 255])));
        let before = State {
            image: image.clone(),
            mask: Arc::new(vec![1]),
        };
        let after = State {
            image: Arc::new(RgbaImage::from_pixel(2, 2, image::Rgba([10, 10, 10, 255]))),
            mask: Arc::new(vec![0]),
        };
        let mut history = History::default();
        history.record(before.clone(), &after);
        assert!(history.undo(&after) == Some(before.clone()));
        assert!(history.redo(&before) == Some(after));
    }
    #[test]
    fn cache_writes_are_unique_and_include_mask() {
        let folder = std::env::temp_dir().join(format!(
            "unity-watermark-cache-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let image = RgbaImage::from_pixel(2, 2, image::Rgba([1, 2, 3, 4]));
        let first = cache_save(&image, &[1], &folder).unwrap();
        let saved = std::fs::read(&first).unwrap();
        let second = cache_save(&image, &[1], &folder).unwrap();
        assert_ne!(first, second);
        assert_eq!(std::fs::read(&first).unwrap(), saved);
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 4);
        std::fs::remove_dir_all(folder).unwrap();
    }
}
