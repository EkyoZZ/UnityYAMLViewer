#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use eframe::egui;
use ps_tile_installer as installer;
use std::{path::PathBuf, sync::mpsc};

struct App {
    appdata: Option<PathBuf>,
    status: String,
    status_error: bool,
    installed: String,
    pending: Option<mpsc::Receiver<installer::Result<String>>>,
    confirm_uninstall: bool,
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        for path in [r"C:\Windows\Fonts\msyh.ttc", r"C:\Windows\Fonts\simhei.ttf"] {
            if let Ok(bytes) = std::fs::read(path) {
                fonts.font_data.insert(
                    "chinese".into(),
                    std::sync::Arc::new(egui::FontData::from_owned(bytes)),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "chinese".into());
                break;
            }
        }
        cc.egui_ctx.set_fonts(fonts);
        let mut app = Self {
            appdata: std::env::var_os("APPDATA").map(PathBuf::from),
            status: "安装到当前用户目录；无需管理员权限。".into(),
            status_error: false,
            installed: String::new(),
            pending: None,
            confirm_uninstall: false,
        };
        app.refresh();
        app
    }
    fn refresh(&mut self) {
        self.installed = match &self.appdata {
            Some(root) => match installer::installed(root) {
                Ok(Some(v)) => format!("目录中的插件版本：{v}（PS 加载状态未检测）"),
                Ok(None) => "尚未安装".into(),
                Err(e) => format!("无法读取安装状态：{e}"),
            },
            None => "无法读取 APPDATA，安装与卸载已禁用。".into(),
        };
    }
    fn action(&mut self, ctx: &egui::Context, uninstall: bool) {
        let Some(root) = self.appdata.clone() else {
            return;
        };
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.confirm_uninstall = false;
        self.status_error = false;
        self.status = if uninstall {
            "正在卸载…"
        } else {
            "正在安装并校验…"
        }
        .into();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = if uninstall {
                installer::uninstall(&root)
            } else {
                installer::install(&root)
            };
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(result) = self.pending.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(v) => Some(v),
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("安装任务中断；请检查目标目录后重试。".into()))
            }
            Err(_) => None,
        }) {
            self.pending = None;
            self.status_error = result.is_err();
            self.status = result.unwrap_or_else(|e| e);
            self.refresh();
        }
        egui::CentralPanel::default().show(ctx,|ui|{
            ui.heading("PS Tile 插件安装器");
            ui.label(format!("Isometric Tile Preview · 插件 {}",installer::version()));
            ui.label(&self.installed);ui.separator();
            ui.label("安装位置");
            if let Some(root)=&self.appdata{let mut path=installer::target(root).display().to_string();ui.add(egui::TextEdit::multiline(&mut path).desired_rows(2).interactive(false).desired_width(f32::INFINITY));}
            ui.label("安装前请关闭 Photoshop，完成后重新打开。仅部署此插件，不修改 PS 安装目录或文件关联。");
            ui.add_space(12.);
            let enabled=self.pending.is_none()&&self.appdata.is_some();
            ui.add_enabled_ui(enabled,|ui|{
                ui.horizontal(|ui|{
                    if ui.button("安装 / 更新").clicked(){self.action(ctx,false);}
                    if ui.button("卸载插件").clicked(){self.confirm_uninstall=true;}
                    if ui.button("刷新状态").clicked(){self.refresh();}
                });
                if self.confirm_uninstall {
                    ui.label("确认移除本插件的四个文件？其他文件会保留。");
                    ui.horizontal(|ui|{if ui.button("确认卸载").clicked(){self.action(ctx,true);}if ui.button("取消").clicked(){self.confirm_uninstall=false;}});
                }
            });
            if self.pending.is_some(){ui.spinner();}
            ui.add_space(12.);
            if self.status_error {
                ui.add(egui::Label::new(egui::RichText::new(&self.status).color(egui::Color32::from_rgb(255, 110, 110)).strong()).wrap());
            } else if self.status.starts_with("已部署并校验 ") {
                let (success, detail) = self.status.split_once('；').unwrap_or((&self.status, ""));
                let font = egui::TextStyle::Body.resolve(ui.style());
                let mut text = egui::text::LayoutJob::default();
                text.append(success, 0.0, egui::TextFormat {
                    font_id: font.clone(), color: egui::Color32::from_rgb(100, 210, 135), ..Default::default()
                });
                if !detail.is_empty() {
                    text.append(&format!("；{detail}"), 0.0, egui::TextFormat {
                        font_id: font, color: ui.visuals().text_color(), ..Default::default()
                    });
                }
                ui.add(egui::Label::new(text).wrap());
            } else {
                ui.label(&self.status);
            }
            ui.separator();ui.small("适用于 Windows 当前用户；插件需要 Photoshop 24.4 或更高版本。");
            if self.pending.is_some(){ui.small("请等待操作完成后关闭安装器。");}
        });
        if self.pending.is_some() && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
    }
}
fn main() -> eframe::Result {
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../../../../assets/app_icon_256.png"))
            .expect("embedded icon");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([620., 420.])
            .with_min_inner_size([520., 380.])
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native(
        "PS Tile 插件安装器",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
