use crate::localization::tr;
use eframe::egui;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

pub(crate) enum Target {
    File(PathBuf),
    Playback(String),
}

pub(crate) enum Outcome {
    Complete(Option<Target>),
    Failed(String),
    Cancelled,
}

pub(crate) struct Progress {
    pub stage: String,
    pub fraction: Option<f32>,
    pub cancellable: bool,
}

struct Running {
    progress: Box<dyn Fn() -> Progress + Send + Sync>,
    cancel: Option<Box<dyn Fn() + Send + Sync>>,
}

enum State {
    Running(Running),
    Finished {
        outcome: Outcome,
    },
}

struct Entry {
    kind: &'static str,
    name: String,
    state: Mutex<State>,
}

type Entries = Arc<Mutex<Vec<Arc<Entry>>>>;

fn entries(ctx: &egui::Context) -> Entries {
    ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<Entries>(egui::Id::new("background_tasks"))
            .clone()
    })
}

pub(crate) struct Task {
    entry: Arc<Entry>,
    ctx: egui::Context,
}

impl Task {
    pub fn start(
        ctx: &egui::Context,
        kind: &'static str,
        name: String,
        progress: impl Fn() -> Progress + Send + Sync + 'static,
        cancel: Option<Box<dyn Fn() + Send + Sync>>,
    ) -> Self {
        let entry = Arc::new(Entry {
            kind,
            name,
            state: Mutex::new(State::Running(Running {
                progress: Box::new(progress),
                cancel,
            })),
        });
        let entries = entries(ctx);
        let mut rows = entries.lock().unwrap();
        if rows.len() >= 100 {
            if let Some(index) = rows
                .iter()
                .position(|entry| matches!(*entry.state.lock().unwrap(), State::Finished { .. }))
            {
                rows.remove(index);
            }
        }
        rows.push(entry.clone());
        ctx.request_repaint_of(egui::ViewportId::ROOT);
        Self {
            entry,
            ctx: ctx.clone(),
        }
    }

    pub fn finish(self, outcome: Outcome) {
        *self.entry.state.lock().unwrap() = State::Finished {
            outcome,
        };
        self.ctx.request_repaint_of(egui::ViewportId::ROOT);
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        let mut state = self.entry.state.lock().unwrap();
        if matches!(*state, State::Running(_)) {
            *state = State::Finished {
                outcome: Outcome::Failed(tr("后台任务已中断").into()),
            };
            self.ctx.request_repaint_of(egui::ViewportId::ROOT);
        }
    }
}

pub(crate) fn active(ctx: &egui::Context) -> bool {
    entries(ctx).lock().unwrap().iter().any(|entry| matches!(*entry.state.lock().unwrap(), State::Running(_)))
}

fn task_kind_color(kind: &str, dark: bool) -> egui::Color32 {
    let (night, day) = match kind {
        "保存工程" => ([106, 180, 255], [20, 95, 175]),
        "打包工程" => ([120, 195, 245], [20, 105, 155]),
        "视频缓存" => ([80, 210, 180], [0, 115, 90]),
        "下载视频" => ([105, 205, 245], [0, 105, 155]),
        "视频编辑" => ([195, 155, 255], [120, 65, 180]),
        "音频编辑" => ([245, 155, 200], [160, 45, 105]),
        "合并导出" => ([245, 185, 95], [155, 90, 0]),
        "队列导出" => ([170, 210, 115], [80, 120, 20]),
        _ => ([185, 195, 210], [80, 95, 115]),
    };
    let [r,g,b] = if dark { night } else { day };
    egui::Color32::from_rgb(r,g,b)
}

fn task_row(ui: &mut egui::Ui, entry: &Entry) -> Option<String> {
    let state = entry.state.lock().unwrap();
    let progress = match &*state { State::Running(job) => Some((job, (job.progress)())), _ => None };
    let (status, detail, color) = match &*state {
        State::Running(_) => {
            let (_, p) = progress.as_ref().unwrap();
            let status = p.fraction.map_or_else(||p.stage.clone(), |fraction|
                format!("{} {:.0}%", p.stage, fraction.clamp(0.0,1.0)*100.0));
            (status, p.stage.clone(), ui.visuals().text_color())
        }
        State::Finished { outcome:Outcome::Complete(_), .. } => (format!("#{}", tr("任务完成")), String::new(), ui.visuals().text_color()),
        State::Finished { outcome:Outcome::Failed(error), .. } => (tr("任务失败").into(), error.clone(), ui.visuals().error_fg_color),
        State::Finished { outcome:Outcome::Cancelled, .. } => (tr("任务已取消").into(), String::new(), ui.visuals().weak_text_color()),
    };
    let target = match &*state { State::Finished {outcome:Outcome::Complete(target),..} => target.as_ref(), _ => None };
    let mut playback = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let width = ui.available_width();
        let kind = format!("[{}]", tr(entry.kind));
        let font = egui::TextStyle::Body.resolve(ui.style());
        let kind_width = (ui.painter().layout_no_wrap(kind.clone(),font.clone(),color).size().x+2.0).min(width*0.28);
        ui.add_sized([kind_width,24.0],egui::Label::new(egui::RichText::new(&kind).strong().color(task_kind_color(entry.kind,ui.visuals().dark_mode))).truncate())
            .on_hover_text(kind);
        let cancel_width = if progress.as_ref().is_some_and(|(job,_)|job.cancel.is_some()) {
            ui.painter().layout_no_wrap(tr("取消").into(),font.clone(),color).size().x+ui.spacing().button_padding.x*2.0+6.0
        } else { 0.0 };
        let status_width = (ui.painter().layout_no_wrap(status.clone(),font.clone(),color).size().x+2.0).min(width*0.36);
        let name_width = (ui.painter().layout_no_wrap(entry.name.clone(),font,color).size().x+2.0)
            .min((ui.available_width()-status_width-cancel_width-6.0).max(1.0));
        let missing = matches!(target, Some(Target::File(path)) if !path.exists());
        let name_color = if missing { ui.visuals().error_fg_color } else if target.is_some() {ui.visuals().hyperlink_color} else {ui.visuals().text_color()};
        let label = egui::RichText::new(&entry.name).color(name_color);
        let name = if target.is_some() && !missing {
            ui.add_sized([name_width,24.0],egui::Label::new(label).truncate().halign(egui::Align::Min).sense(egui::Sense::click()))
                .on_hover_cursor(egui::CursorIcon::PointingHand)
        } else {ui.add_sized([name_width,24.0],egui::Label::new(label).truncate().halign(egui::Align::Min))};
        match target {
            Some(Target::File(path)) => {
                let error_id = ui.id().with("reveal_error");
                let error = ui.ctx().data(|data|data.get_temp::<String>(error_id));
                let mut hint = format!("{}\n{}",tr(if missing {"原路径已失效"}else{"打开文件所在目录"}),path.display());
                if let Some(error) = error {hint.push('\n');hint.push_str(&error);}
                if name.on_hover_text(hint).clicked() && !missing {
                    let result = if path.is_dir() {crate::media_tools::command("explorer.exe").arg(path).spawn().map(|_|())}
                        else {crate::asset_viewer::reveal_file(path)};
                    ui.ctx().data_mut(|data|{data.remove::<String>(error_id);if let Err(error)=result {data.insert_temp(error_id,error.to_string());}});
                }
            }
            Some(Target::Playback(url)) => {
                if name.on_hover_text(format!("{}\n{}",tr("打开播放页"),url)).clicked() {playback=Some(url.clone());}
            }
            None => {name.on_hover_text(&entry.name);}
        }
        ui.add_sized([status_width,24.0],egui::Label::new(egui::RichText::new(&status).color(color)).truncate().halign(egui::Align::Min))
            .on_hover_text(if detail.is_empty(){status.as_str()}else{detail.as_str()});
        if let Some((job,p)) = &progress {
            if let Some(cancel) = &job.cancel {
                if ui.add_enabled(p.cancellable,egui::Button::new(tr("取消"))).clicked() {cancel();}
            }
        }
    });
    playback
}

pub(crate) fn button(ui: &mut egui::Ui) -> Option<String> {
    let entries = entries(ui.ctx());
    let mut rows = entries.lock().unwrap().clone();
    let id = ui.id().with("background_tasks_popup");
    let response =
        crate::theme::toolbar_icon(ui, crate::icons::Icon::BackgroundTasks, tr("后台任务中心"));
    if response.clicked() {
        ui.memory_mut(|memory| memory.toggle_popup(id));
    }
    let mut running = false;
    let mut finished = false;
    let mut problem = false;
    for entry in &rows {
        match &*entry.state.lock().unwrap() {
            State::Running(_) => running = true,
            State::Finished { outcome, .. } => {
                finished = true;
                problem |= matches!(outcome, Outcome::Failed(_) | Outcome::Cancelled);
            }
        }
    }
    if running || finished {
        let color = if running {
            let now = ui.input(|input| input.time);
            const PHASE: f64 = 0.5;
            let phase = (now / PHASE).floor();
            ui.ctx().request_repaint_after(Duration::from_secs_f64(((phase+1.0)*PHASE-now).max(0.01)));
            let yellow = egui::Color32::from_rgb(245,190,40);
            if phase as u64 % 2 == 0 { yellow } else { yellow.gamma_multiply(0.18) }
        } else if problem {
            egui::Color32::from_rgb(231,147,54)
        } else if ui.visuals().dark_mode {
            egui::Color32::from_rgb(70,205,110)
        } else {
            egui::Color32::from_rgb(25,145,65)
        };
        let center = response.rect.right_top() + egui::vec2(-5.0, 5.0);
        ui.painter().circle_filled(center, 4.5, ui.visuals().window_fill);
        ui.painter().circle_filled(center, 3.0, color);
    }

    egui::popup::popup_below_widget(
        ui,
        id,
        &response,
        egui::popup::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            let font = egui::TextStyle::Body.resolve(ui.style());
            let measure = |text: String| ui.painter().layout_no_wrap(text,font.clone(),ui.visuals().text_color()).size().x;
            let has_finished = rows.iter().any(|entry| matches!(*entry.state.lock().unwrap(), State::Finished { .. }));
            let header_width = measure(tr("后台任务中心").into())
                + if has_finished { measure(tr("清除已结束任务").into()) + ui.spacing().button_padding.x*2.0 + ui.spacing().item_spacing.x } else { 0.0 };
            let longest = rows.iter().map(|entry| {
                let state = entry.state.lock().unwrap();
                let (status, cancel) = match &*state {
                    State::Running(job) => {
                        let p = (job.progress)();
                        (p.fraction.map_or_else(||p.stage.clone(),|fraction|format!("{} {:.0}%",p.stage,fraction.clamp(0.0,1.0)*100.0)),job.cancel.is_some())
                    }
                    State::Finished {outcome:Outcome::Complete(_),..} => (format!("#{}",tr("任务完成")),false),
                    State::Finished {outcome:Outcome::Failed(_),..} => (tr("任务失败").into(),false),
                    State::Finished {outcome:Outcome::Cancelled,..} => (tr("任务已取消").into(),false),
                };
                measure(format!("[{}]",tr(entry.kind))) + measure(entry.name.clone()) + measure(status) + 18.0
                    + if cancel {measure(tr("取消").into())+ui.spacing().button_padding.x*2.0+6.0}else{0.0}
            }).fold(0.0_f32,f32::max);
            let maximum = (ui.ctx().screen_rect().width()-24.0).clamp(1.0,620.0);
            let width = longest.max(header_width+4.0).max(160.0).min(maximum);
            ui.set_width(width);
            ui.horizontal_wrapped(|ui| {
                ui.strong(tr("后台任务中心"));
                if rows
                    .iter()
                    .any(|entry| matches!(*entry.state.lock().unwrap(), State::Finished { .. }))
                    && ui.small_button(tr("清除已结束任务")).clicked()
                {
                    entries
                        .lock()
                        .unwrap()
                        .retain(|entry| matches!(*entry.state.lock().unwrap(), State::Running(_)));
                    rows.retain(|entry| matches!(*entry.state.lock().unwrap(), State::Running(_)));
                }
            });
            if rows.is_empty() {
                ui.label(tr("暂无后台任务"));
                return None;
            }
            if running {
                ui.ctx().request_repaint_after(Duration::from_millis(150));
            }
            let mut playback = None;
            egui::ScrollArea::vertical()
                .max_height((ui.ctx().screen_rect().height() * 0.55).min(410.0))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for entry in rows.iter().rev() {
                        ui.push_id(Arc::as_ptr(entry) as usize, |ui| {
                            ui.separator();
                            if let Some(url) = task_row(ui, entry) { playback = Some(url); }
                        });
                    }
                });
            if playback.is_some() {
                ui.memory_mut(|memory| memory.close_popup());
            }
            playback
        },
    )
    .flatten()
}
