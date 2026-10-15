//! Temporary pass-state controls; unsupported depth and fixed-function lighting stay explicit.
use super::{rs, egui, Arc, tr};
#[derive(Clone, Copy, PartialEq, Eq)]
struct Settings { source: rs::BlendFactor, destination: rs::BlendFactor, cull_back: bool }
impl Settings {
    fn from(state: rs::RenderState) -> Self { Self { source: state.source, destination: state.destination, cull_back: state.cull_back } }
}
#[derive(Default)]
pub(super) struct Editor {
    selected: Option<Settings>,
    cache: Option<(Arc<rs::Scene>, Settings, Arc<rs::Scene>)>,
}
impl Editor {
    pub fn release(&mut self) { self.cache = None; }
    pub fn reset(&mut self) { self.selected = None; self.cache = None; }
    pub fn scene(&mut self, source: &Arc<rs::Scene>) -> Arc<rs::Scene> {
        let Some(selected) = self.selected else { return source.clone(); };
        if let Some((old, settings, result)) = &self.cache {
            if Arc::ptr_eq(old, source) && *settings == selected { return result.clone(); }
        }
        let draws = source.draws.iter().map(|draw| {
            let mut draw = draw.clone();
            let mut material = (*draw.material).clone();
            material.state.source = selected.source;
            material.state.destination = selected.destination;
            material.state.cull_back = selected.cull_back;
            draw.material = Arc::new(material);
            draw
        }).collect::<Vec<_>>();
        let result = Arc::new(rs::Scene { draws: Arc::from(draws), revision: source.revision });
        self.cache = Some((source.clone(), selected, result.clone()));
        result
    }
    pub fn show(&mut self, ui: &mut egui::Ui, source: Option<&Arc<rs::Scene>>, notes: &[(String, String)]) {
        ui.small(tr("临时预览输入，不修改文件"));
        if let Some(state) = source.and_then(|scene|scene.draws.first()).map(|draw|draw.material.state) {
            let mut selected = self.selected.unwrap_or_else(||Settings::from(state));
            let before = selected;
            ui.horizontal(|ui| {
                ui.label("Cull");
                egui::ComboBox::from_id_salt("cull").width(ui.available_width() * 0.65).selected_text(if selected.cull_back {"Back"} else {"Off"}).show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected.cull_back, false, "Off");
                    ui.selectable_value(&mut selected.cull_back, true, "Back");
                });
            });
            for (name, value) in [("Blend Src", &mut selected.source), ("Blend Dst", &mut selected.destination)] {
                ui.horizontal(|ui| {
                    ui.label(name);
                    egui::ComboBox::from_id_salt(name).width(ui.available_width() * 0.65).selected_text(label(*value)).show_ui(ui, |ui| {
                        for factor in factors() { ui.selectable_value(value, factor, label(factor)); }
                    });
                });
            }
            if before != selected { self.selected = Some(selected); self.cache = None; }
            ui.small("Off = One / Zero");
            if ui.button(tr("恢复 Shader 状态")).clicked() { self.reset(); }
        } else {
            ui.weak(tr("预览不可用"));
        }
        ui.separator();
        for (name, value) in notes { ui.label(format!("{name} {value}")); }
        ui.weak(tr("Lighting / 深度状态仅展示，Front 剔除暂不支持"));
    }
}
fn factors() -> [rs::BlendFactor; 11] {
    use rs::BlendFactor::*;
    [Zero, One, SourceAlpha, OneMinusSourceAlpha, DestinationColor, SourceColor, OneMinusSourceColor, DestinationAlpha, OneMinusDestinationAlpha, OneMinusDestinationColor, SourceAlphaSaturate]
}
fn label(factor: rs::BlendFactor) -> &'static str {
    use rs::BlendFactor::*;
    match factor { Zero=>"Zero", One=>"One", SourceAlpha=>"SrcAlpha", OneMinusSourceAlpha=>"OneMinusSrcAlpha", DestinationColor=>"DstColor", SourceColor=>"SrcColor", OneMinusSourceColor=>"OneMinusSrcColor", DestinationAlpha=>"DstAlpha", OneMinusDestinationAlpha=>"OneMinusDstAlpha", OneMinusDestinationColor=>"OneMinusDstColor", SourceAlphaSaturate=>"SrcAlphaSaturate" }
}
pub(super) fn notes(source: &str) -> Vec<(String, String)> {
    let source = super::re(r"(?s)/\*.*?\*/|//[^\r\n]*").replace_all(source, "");
    let prefix = source.split("CGPROGRAM").next().unwrap_or(&source);
    [("Lighting", "Off"), ("ZWrite", "On"), ("ZTest", "LEqual")].into_iter().map(|(name, default)| {
        let pattern = super::re(&format!(r"(?i)\b{name}\s+(\w+|\[[^\]]+\])"));
        (name.to_owned(), pattern.captures(prefix).map_or(default.to_owned(), |c|c[1].to_owned()))
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn state_override_keeps_shader_textures_and_source_unchanged() {
        let state = rs::RenderState { source: rs::BlendFactor::One, destination: rs::BlendFactor::OneMinusSourceAlpha, color_mask: [true;4], depth_write: false, cull_back: false };
        let program = Arc::new(rs::Program { pipeline: rs::UnityPipeline::BuiltIn, vertex: Arc::from("vertex"), fragment: Arc::from("fragment"), revision: 1 });
        let texture = Arc::new(rs::Texture { size: [1,1], rgba: Arc::from([255;4]), revision: 1, repeat: [false;2] });
        let source = Arc::new(rs::Scene { revision: 2, draws: Arc::from([rs::Draw {
            strict_shader: true, component: Arc::from("test"), vertices: Arc::from([]), indices: Arc::from([]), object_to_world: super::super::IDENTITY,
            material: Arc::new(rs::Material { program: program.clone(), state, uniforms: Default::default(), textures: Default::default() }), main_texture: texture.clone(),
        }]) });
        let mut editor = Editor::default();
        editor.selected = Some(Settings { source: rs::BlendFactor::SourceAlpha, destination: rs::BlendFactor::One, cull_back: true });
        let changed = editor.scene(&source);
        assert_eq!(source.draws[0].material.state.source, rs::BlendFactor::One);
        assert_eq!(changed.draws[0].material.state.source, rs::BlendFactor::SourceAlpha);
        assert_eq!(changed.draws[0].material.state.destination, rs::BlendFactor::One);
        assert!(changed.draws[0].material.state.cull_back);
        assert!(!changed.draws[0].material.state.depth_write);
        assert!(Arc::ptr_eq(&changed.draws[0].material.program, &program));
        assert!(Arc::ptr_eq(&changed.draws[0].main_texture, &texture));
        assert!(Arc::ptr_eq(&editor.scene(&source), &changed));
        editor.reset(); assert!(Arc::ptr_eq(&editor.scene(&source), &source));
    }
}
