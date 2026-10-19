//! A read-only input graph. Overrides live here, never in an editing session.
use crate::{
    localization::tr,
    render_system::{self, scene as rs},
};
use eframe::egui::{self, Color32, Rect, Sense, Stroke, Vec2};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
};

pub(crate) fn available(path: Option<&Path>) -> bool {
    path.and_then(Path::extension)
        .is_some_and(|e| e.eq_ignore_ascii_case("mat") || e.eq_ignore_ascii_case("shader"))
}

mod geometry;
mod stages;
mod states;
mod probe;
mod mini;
mod helper;
mod flow;
mod operations;
mod tools;
mod interfaces;

/// Horizontal port tangents with bounded, distance-dependent handles.
fn wire_points(from: egui::Pos2, to: egui::Pos2, zoom: f32) -> [egui::Pos2; 4] {
    let dx = to.x - from.x;
    let dy = (to.y - from.y).abs();
    let minimum = 12. * zoom;
    let handle = if dx > 0. {
        (dx * 0.5 + dy * 0.15).min(dx * 0.8).min(220. * zoom).max(minimum.min(dx * 0.5))
    } else {
        (dx.abs() * 0.5 + dy * 0.15 + 40. * zoom).clamp(40. * zoom, 220. * zoom)
    };
    [from, from + egui::vec2(handle, 0.), to - egui::vec2(handle, 0.), to]
}

type Catalog = HashMap<String, PathBuf>;
const IDENTITY: [[f32; 4]; 4] = [
    [1., 0., 0., 0.],
    [0., 1., 0., 0.],
    [0., 0., 1., 0.],
    [0., 0., 0., 1.],
];
fn re(s: &str) -> regex::Regex {
    regex::Regex::new(s).unwrap()
}
fn read(path: &Path) -> Result<String, String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 2 * 1024 * 1024 {
        return Err("Source exceeds 2 MiB".into());
    }
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Float(f32, Option<[f32; 2]>),
    Vector([f32; 4], bool),
    Texture {
        path: Option<PathBuf>,
        default: String,
        st: [f32; 4],
        unresolved: bool,
        kind: String,
    },
}
#[derive(Clone, Debug)]
struct Input {
    name: String,
    label: String,
    value: Value,
    integer: bool,
    shader_type: String,
    bindings: Vec<(String, String)>,
}
struct Loaded {
    render_notes: Vec<(String, String)>,
    shader: PathBuf,
    material: String,
    inputs: Vec<Input>,
    stages: stages::Graph,
    source_warning: Option<String>,
}

// Tokenizing first keeps braces, comments, multiline declarations and Range()
// out of the property boundary logic. Strings are retained as single tokens.
fn tokens(source: &str) -> Vec<String> {
    re(r#"(?s)/\*.*?\*/|//[^\r\n]*|"(?:\\.|[^"\\])*"|(?:2DArray|2D|3D)\b|[A-Za-z_][A-Za-z_0-9]*|[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?|[^\s]"#)
        .find_iter(source).map(|m|m.as_str()).filter(|s|!s.starts_with("//")&&!s.starts_with("/*")).map(str::to_owned).collect()
}
fn block_end(t: &[String], start: usize, open: &str, close: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, s) in t.iter().enumerate().skip(start) {
        if s == open {
            depth += 1;
        }
        if s == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}
fn number(s: &str) -> Result<f32, String> {
    s.parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("Invalid number: {s}"))
}
fn properties(source: &str) -> Result<Vec<Input>, String> {
    let t = tokens(source);
    let Some(start) = t
        .windows(2)
        .position(|w| w[0] == "Properties" && w[1] == "{")
        .map(|i| i + 1)
    else {
        return Ok(Vec::new());
    };
    let end = block_end(&t, start, "{", "}").ok_or("Unterminated Properties")?;
    let mut i = start + 1;
    let mut inputs = Vec::new();
    while i < end {
        if t[i] == "[" {
            i = block_end(&t, i, "[", "]").ok_or("Unterminated property attribute")? + 1;
            continue;
        }
        let name = t[i].clone();
        i += 1;
        if t.get(i).map(String::as_str) != Some("(") {
            return Err(format!("Invalid property: {name}"));
        }
        let close = block_end(&t, i, "(", ")").ok_or("Unterminated property declaration")?;
        let label = t
            .get(i + 1)
            .ok_or("Missing property label")?
            .trim_matches('"')
            .to_owned();
        let kind = t.get(i + 3).ok_or("Missing property type")?.clone();
        let range = if kind.eq_ignore_ascii_case("Range") {
            Some([
                number(t.get(i + 5).ok_or("Missing range minimum")?)?,
                number(t.get(i + 7).ok_or("Missing range maximum")?)?,
            ])
        } else {
            None
        };
        i = close + 1;
        if t.get(i).map(String::as_str) != Some("=") {
            return Err("Missing property default".into());
        }
        i += 1;
        let integer = matches!(kind.to_ascii_lowercase().as_str(), "int" | "integer");
        let value = match kind.to_ascii_lowercase().as_str() {
            "float" | "int" | "integer" | "range" => {
                let v = number(t.get(i).ok_or("Missing number")?)?;
                i += 1;
                Value::Float(v, range)
            }
            "color" | "vector" => {
                if t.get(i).map(String::as_str) != Some("(") {
                    return Err("Missing vector default".into());
                }
                let last = block_end(&t, i, "(", ")").ok_or("Invalid vector default")?;
                let values = t[i + 1..last]
                    .iter()
                    .filter(|s| s.as_str() != ",")
                    .map(|s| number(s))
                    .collect::<Result<Vec<_>, _>>()?;
                if values.len() != 4 {
                    return Err("Expected four vector components".into());
                }
                i = last + 1;
                Value::Vector(
                    [values[0], values[1], values[2], values[3]],
                    kind.eq_ignore_ascii_case("Color"),
                )
            }
            "2d" | "3d" | "cube" | "2darray" | "cubearray" => {
                let default = t
                    .get(i)
                    .ok_or("Missing texture default")?
                    .trim_matches('"')
                    .to_owned();
                i += 1;
                if t.get(i).map(String::as_str) == Some("{") {
                    i = block_end(&t, i, "{", "}").ok_or("Invalid texture default")? + 1;
                }
                Value::Texture {
                    path: None,
                    default,
                    st: [1., 1., 0., 0.],
                    unresolved: false,
                    kind: kind.clone(),
                }
            }
            _ => return Err(format!("Unsupported property type: {kind}")),
        };
        if inputs.iter().any(|p: &Input| p.name == name) {
            return Err(format!("Duplicate property: {name}"));
        }
        inputs.push(Input {
            name,
            label,
            value,
            integer,
            shader_type: kind.clone(),
            bindings: Vec::new(),
        });
        if inputs.len() > 128 {
            return Err("Property limit: 128".into());
        }
    }
    annotate_inputs(source, &mut inputs);
    Ok(inputs)
}
// Precision is recorded, but never used as evidence that a vector is a color.
// Only explicit Color metadata and identifiable use sites supply that meaning.
fn annotate_inputs(source: &str, inputs: &mut [Input]) {
    let mut t = tokens(source);
    if let Some(start) = t
        .windows(2)
        .position(|w| w[0] == "Properties" && w[1] == "{")
        .map(|i| i + 1)
    {
        if let Some(end) = block_end(&t, start, "{", "}") {
            t.drain(start - 1..=end);
        }
    }
    let code = t
        .into_iter()
        .filter(|s| !s.starts_with('"'))
        .collect::<Vec<_>>()
        .join(" ");
    for input in inputs {
        let name = regex::escape(&input.name);
        let declaration = re(&format!(
            r"\b(float4|half4|fixed4|vec4|float|half|fixed|int)\s+{name}\b"
        ));
        if let Some(c) = declaration.captures(&code) {
            input.shader_type = c[1].to_owned();
        }
        let access = re(&format!(r"\b{name}\s*\.\s*([rgbaxyzw]{{1,4}})\b"));
        let channels = access
            .captures_iter(&code)
            .map(|c| c[1].to_owned())
            .collect::<Vec<_>>();
        let sampled = re(r"\b(?:float4|half4|fixed4)\s+(\w+)\s*=\s*tex2D\b[^;]*;")
            .captures_iter(&code)
            .map(|c| c[1].to_owned())
            .collect::<Vec<_>>();
        let texture_tint = re(&format!(
            r"\b{name}\s*\*\s*tex2D\b|tex2D\s*\([^;]+?\)\s*\*\s*{name}\b"
        ))
        .is_match(&code)
            || sampled.iter().any(|sample| {
                re(&format!(
                    r"\b{}\s*(?:\*\s*|\*\s*=\s*){name}\b|\b{name}\s*\*\s*{}\b",
                    regex::escape(sample),
                    regex::escape(sample)
                ))
                .is_match(&code)
            });
        let fragment_color = re(r"\bpragma\s+fragment\s+(\w+)")
            .captures(&code)
            .and_then(|c| {
                let function = re(&format!(
                    r"\b{}\s*\([^)]*\)\s*(?::\s*\w+\s*)?\{{([^{{}}]*)\}}",
                    regex::escape(&c[1])
                ));
                function
                    .captures(&code)
                    .map(|body| re(&format!(r"\breturn\s+{name}\s*;")).is_match(&body[1]))
            })
            .unwrap_or(false);
        if let Value::Vector(_, color) = &mut input.value {
            if !*color
                && (channels
                    .iter()
                    .any(|s| s.chars().all(|c| "rgba".contains(c)))
                    || texture_tint
                    || fragment_color)
                && !channels
                    .iter()
                    .any(|s| s.chars().any(|c| "xyzw".contains(c)))
            {
                *color = true;
            }
        }
        let color = matches!(input.value, Value::Vector(_, true));
        let map_channel = |s: &str| -> String {
            if color {
                s.chars()
                    .map(|c| match c {
                        'x' => 'R',
                        'y' => 'G',
                        'z' => 'B',
                        'w' => 'A',
                        _ => c.to_ascii_uppercase(),
                    })
                    .collect()
            } else {
                s.to_ascii_uppercase()
            }
        };
        for channel in channels {
            let channel = map_channel(&channel);
            if !input.bindings.iter().any(|(_, b)| b == &channel) {
                let output = if [
                    "RGBA", "RGB", "R", "G", "B", "A", "XYZW", "XYZ", "X", "Y", "Z", "W",
                ]
                .contains(&channel.as_str())
                {
                    channel.clone()
                } else if color {
                    "RGBA".into()
                } else {
                    "XYZW".into()
                };
                input.bindings.push((output, channel));
            }
        }
        if matches!(input.value, Value::Float(..)) {
            let assignments = re(
                r"\b\w+\s*\.\s*(rgb|rgba|r|g|b|a|xyz|xyzw|x|y|z|w)\s*(?:[*+/-]\s*)?=\s*([^;]+);",
            );
            for c in assignments.captures_iter(&code) {
                if re(&format!(r"\b{name}\b")).is_match(&c[2]) {
                    let target = c[1].to_ascii_uppercase();
                    if !input.bindings.iter().any(|(_, b)| b == &target) {
                        input.bindings.push(("Out".into(), target));
                    }
                }
            }
        }
        // A declaration alone is not a use. Unused properties keep unconnected ports.
        let uses = re(&format!(r"\b{name}\b")).find_iter(&code).count();
        let without_access = access.replace_all(&code, "");
        let without_declaration = declaration.replace_all(&without_access, "");
        let whole_vector_use = matches!(input.value, Value::Vector(..))
            && re(&format!(r"\b{name}\b")).is_match(&without_declaration);
        if (input.bindings.is_empty() && uses > 1) || whole_vector_use {
            let port = match input.value {
                Value::Vector(_, true) => "RGBA",
                Value::Vector(..) => "XYZW",
                _ => "Out",
            };
            if !input.bindings.iter().any(|(a, b)| a == port && b == port) {
                input.bindings.push((port.into(), port.into()));
            }
        }
    }
}
fn field<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.lines().find_map(|line| {
        line.trim()
            .strip_prefix(name)?
            .strip_prefix(':')
            .map(str::trim)
    })
}
fn guid(value: &str, catalog: &Catalog) -> Option<PathBuf> {
    re(r"\bguid:\s*([a-fA-F0-9]{32})\b")
        .captures(value)
        .and_then(|c| catalog.get(&c[1].to_ascii_lowercase()).cloned())
}
fn material_inputs(inputs: &mut [Input], material: &str, catalog: &Catalog) -> Result<(), String> {
    for input in inputs {
        let scalar = re(&format!(
            r"(?m)^\s*-\s*{}:\s*([-+0-9.eE]+)\s*$",
            regex::escape(&input.name)
        ));
        match &mut input.value {
            Value::Float(v, _) => {
                if let Some(c) = scalar.captures(material) {
                    *v = number(&c[1])?;
                }
            }
            Value::Vector(v, _) => {
                let vector = re(&format!(
                    r"(?m)^\s*-\s*{}:\s*\{{([^}}]+)\}}",
                    regex::escape(&input.name)
                ));
                if let Some(c) = vector.captures(material) {
                    let parts =
                        crate::inline_vector(&format!("{{{}}}", &c[1]), &["r", "g", "b", "a"])
                            .ok_or("Invalid material vector")?;
                    for i in 0..4 {
                        v[i] = parts[i] as f32;
                        if !v[i].is_finite() {
                            return Err("Non-finite material vector".into());
                        }
                    }
                }
            }
            Value::Texture {
                path,
                st,
                unresolved,
                ..
            } => {
                let entry = re(&format!(
                    r"(?m)^([ \t]*)-\s*{}:\s*$",
                    regex::escape(&input.name)
                ));
                if let Some(c) = entry.captures(material) {
                    let tail = &material[c.get(0).unwrap().end()..];
                    let indent = c[1].len();
                    let block = tail
                        .lines()
                        .skip(1)
                        .take_while(|l| {
                            l.trim().is_empty() || l.len() - l.trim_start().len() > indent
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    if let Some(reference) = field(&block, "m_Texture") {
                        let empty = re(r"fileID:\s*0\b").is_match(reference);
                        *path = if empty {
                            None
                        } else {
                            guid(reference, catalog)
                        };
                        *unresolved = path.is_none() && !empty;
                    }
                    for (field_name, offset) in [("m_Scale", 0), ("m_Offset", 2)] {
                        if let Some(v) = field(&block, field_name)
                            .and_then(|v| crate::inline_vector(v, &["x", "y"]))
                        {
                            st[offset] = v[0] as f32;
                            st[offset + 1] = v[1] as f32;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
fn load(path: &Path, text: &str, catalog: &Catalog) -> Result<Loaded, String> {
    if text.len() > 2 * 1024 * 1024 {
        return Err("Source exceeds 2 MiB".into());
    }
    let is_material = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mat"));
    let shader = if is_material {
        crate::static_preview::field(text, "m_Shader")
            .and_then(|v| guid(v, catalog))
            .ok_or("Shader GUID unavailable (built-in shader or missing project resource)")?
    } else {
        path.to_owned()
    };
    let source = if is_material {
        read(&shader)?
    } else {
        text.to_owned()
    };
    let mut inputs = properties(&source)?;
    if is_material {
        material_inputs(&mut inputs, text, catalog)?;
    }
    let (expanded, source_warning) = match render_system::shader::expanded_source_file(&shader) {
        Ok(source) => (source, None),
        Err(error) => (
            source.clone(),
            Some(format!("Function source expansion unavailable: {error}")),
        ),
    };
    let stages = stages::parse(&expanded, &inputs);
    Ok(Loaded {
        render_notes: states::notes(&source),        stages,
        source_warning,
        shader,
        material: if is_material {
            text.to_owned()
        } else {
            String::new()
        },
        inputs,
    })
}
fn texture(path: &Option<PathBuf>, default: &str) -> Result<Arc<rs::Texture>, String> {
    let (size, rgba) = if let Some(path) = path {
        let mut reader = image::ImageReader::open(path)
            .map_err(|e| e.to_string())?
            .with_guessed_format()
            .map_err(|e| e.to_string())?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(4096);
        limits.max_image_height = Some(4096);
        limits.max_alloc = Some(64 * 1024 * 1024);
        reader.limits(limits);
        let mut image = reader.decode().map_err(|e| e.to_string())?.into_rgba8();
        image::imageops::flip_vertical_in_place(&mut image);
        ([image.width(), image.height()], image.into_raw())
    } else {
        (
            [1, 1],
            match default {
                "black" => vec![0, 0, 0, 255],
                "gray" => vec![128, 128, 128, 255],
                "bump" => vec![128, 128, 255, 255],
                "red" => vec![255, 0, 0, 255],
                "white" | "" => vec![255; 4],
                _ => return Err(format!("Unknown default texture: {default}")),
            },
        )
    };
    let repeat = if let Some(path) = path {
        let meta_path = PathBuf::from(format!("{}.meta", path.display()));
        let meta = if meta_path.exists() {
            read(&meta_path)?
        } else {
            String::new()
        };
        let base = field(&meta, "wrapMode")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);
        let modes = ["wrapU", "wrapV"].map(|axis| {
            field(&meta, axis)
                .and_then(|v| v.parse::<i32>().ok())
                .filter(|v| *v >= 0)
                .unwrap_or(base)
        });
        if modes.iter().any(|v| !matches!(v, 0 | 1)) {
            return Err("Mirror texture wrapping is not supported".into());
        }
        modes.map(|v| v == 0)
    } else {
        [true; 2]
    };
    Ok(Arc::new(rs::Texture {
        size,
        rgba: Arc::from(rgba),
        revision: 1,
        repeat,
    }))
}
fn prepare(
    shader: &Path,
    material: &str,
    inputs: &[Input],
    revision: u64,
    probe: Option<&probe::Probe>,
) -> Result<Arc<rs::Scene>, String> {
    let mut overrides = material.to_owned();
    let mut uniforms = BTreeMap::new();
    let mut textures = BTreeMap::new();
    let mut bytes = 0;
    for input in inputs {
        match &input.value {
            Value::Float(v, _) => {
                uniforms.insert(input.name.clone(), rs::Uniform::Float(*v));
                overrides.push_str(&format!("\n- {}: {}\n", input.name, v));
            }
            Value::Vector(v, _) => {
                uniforms.insert(input.name.clone(), rs::Uniform::Vector(*v));
            }
            Value::Texture {
                path,
                default,
                st,
                unresolved,
                kind,
            } => {
                if kind != "2D" {
                    return Err(format!("Unsupported texture dimension: {kind}"));
                }
                if *unresolved {
                    return Err(format!("Texture GUID unavailable: {}", input.name));
                }
                if st.iter().any(|v| !v.is_finite()) {
                    return Err("Non-finite texture transform".into());
                }
                let texture = texture(path, default)?;
                bytes += texture.rgba.len();
                if bytes > 128 * 1024 * 1024 {
                    return Err("Preview textures exceed 128 MiB".into());
                }
                let [w, h] = texture.size.map(|v| v as f32);
                uniforms.insert(
                    format!("{}_TexelSize", input.name),
                    rs::Uniform::Vector([1. / w, 1. / h, w, h]),
                );
                textures.insert(input.name.clone(), texture);
                uniforms.insert(format!("{}_ST", input.name), rs::Uniform::Vector(*st));
            }
        }
    }
    let (program, mut state) = if let Some(probe) = probe {
        let source = render_system::shader::expanded_source_file(shader)?;
        let source = probe::instrument(&source, probe)?;
        render_system::shader::translate_preview_source(&source, &overrides)?
    } else { render_system::shader::translate_preview_file(shader, &overrides)? };
    if probe.is_some() {
        // Diagnostic values must not be attenuated by the material's blend factors.
        state.source = rs::BlendFactor::One;
        state.destination = rs::BlendFactor::Zero;
        state.color_mask = [true; 4];
    }
    // The shared backend binds float uniforms. Reject integer globals until it
    // can upload them as integer uniforms, rather than silently drawing zero.
    if re(r"(?m)^\s*(?:uniform\s+)?(?:int|uint|bool)\s+_\w+\s*;").is_match(&program.fragment) {
        return Err("Integer shader uniforms are not supported in this preview".into());
    }
    for c in
        re(r"uniform\s+(float|vec[234]|sampler2D)\s+(_\w+)\s*;").captures_iter(&program.fragment)
    {
        if let Some(value) = uniforms.get(&c[2]) {
            let valid = matches!(
                (c.get(1).unwrap().as_str(), value),
                ("float", rs::Uniform::Float(_)) | ("vec4", rs::Uniform::Vector(_))
            );
            if !valid {
                return Err(format!(
                    "Unsupported uniform input type: {} ({})",
                    &c[2], &c[1]
                ));
            }
        }
    }
    let main = match textures.get("_MainTex") {
        Some(t) => t.clone(),
        None => texture(&None, "white")?,
    };
    let vertices = [
        ([-1., -1., 0.], [0., 0.]),
        ([1., -1., 0.], [1., 0.]),
        ([1., 1., 0.], [1., 1.]),
        ([-1., 1., 0.], [0., 1.]),
    ]
    .map(|(position, uv)| rs::Vertex {
        position,
        uv,
        color: [1.; 4],
        normal: [0., 0., -1.],
        tangent: [1., 0., 0., 1.],
    });
    let draw = rs::Draw {
        strict_shader: true,
        component: Arc::from("material-graph"),
        vertices: Arc::from(vertices),
        indices: Arc::from([0, 2, 1, 0, 3, 2]),
        object_to_world: IDENTITY,
        material: Arc::new(rs::Material {
            program: Arc::new(program),
            state,
            uniforms,
            textures,
        }),
        main_texture: main,
    };
    Ok(Arc::new(rs::Scene {
        draws: Arc::from([draw]),
        revision,
    }))
}

fn input_color(value: &Value) -> Color32 {
    match value {
        Value::Float(..) => Color32::from_rgb(135, 205, 220),
        Value::Vector(_, true) => Color32::from_rgb(240, 205, 105),
        Value::Vector(..) => Color32::from_rgb(170, 150, 220),
        Value::Texture { .. } => Color32::from_rgb(225, 130, 135),
    }
}
fn output_y(input: &Input, channel: &str) -> f32 {
    if matches!(input.value, Value::Vector(..)) {
        match channel {
            "RGBA" | "XYZW" => 98.,
            "RGB" | "XYZ" => 128.,
            "R" | "X" => 159.,
            "G" | "Y" => 189.,
            "B" | "Z" => 219.,
            "A" | "W" => 249.,
            _ => 98.,
        }
    } else if matches!(input.value, Value::Float(..)) {
        100.
    } else {
        55.
    }
}

fn input_height(value: &Value) -> f32 {
    match value {
        Value::Float(..) => 126.,
        Value::Vector(..) => 306.,
        Value::Texture { .. } => 270.,
    }
}
fn commit_preview_change(pending: &mut bool, changed: bool, down: bool, released: bool) -> bool {
    if changed && down { *pending = true; }
    if (changed && !down) || (*pending && released) { *pending = false; return true; }
    false
}

fn property_input_ui(ui: &egui::Ui, name: &str, body: Rect) -> egui::Ui {
    let mut child=egui::Ui::new(ui.ctx().clone(), ui.id().with(("property_input", name)),
        egui::UiBuilder::new().layer_id(ui.layer_id()).max_rect(body).style(ui.style().clone()));
    child
}

fn edit_input(
    ui: &mut egui::Ui,
    input: &mut Input,
    thumbnail: Option<&egui::TextureHandle>,
    zoom: f32,
) -> bool {
    let before = input.value.clone();
    let mut changed = false;
    ui.add(egui::Label::new(&input.label).truncate())
        .on_hover_text(&input.label);
    ui.add(
        egui::Label::new(
            egui::RichText::new(format!("{} · {}", input.name, input.shader_type)).weak(),
        )
        .truncate(),
    )
    .on_hover_text(&input.name);
    match &mut input.value {
        Value::Float(v, range) => {
            // Use the same local row as the scalar output socket, so controls
            // remain on the module instead of drifting into a parameter panel.
            let row = Rect::from_min_size(
                ui.max_rect().min + egui::vec2(0., 51.) * zoom,
                egui::vec2(ui.available_width(), 26. * zoom),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(row), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Out");
                    changed |= if input.integer {
                        let r = ui.add_sized(
                            [150. * zoom, 26. * zoom],
                            egui::DragValue::new(v).speed(1.).max_decimals(0).update_while_editing(false),
                        );
                        *v = v.round();
                        r.changed()
                    } else if let Some([min, max]) = range.filter(|r| r[0] < r[1]) {
                        ui.spacing_mut().slider_width = 100. * zoom;
                        let sliding = ui.add(egui::Slider::new(&mut *v, min..=max).show_value(false)).changed();
                        let typing = ui.add_sized([70. * zoom, 26. * zoom],
                            egui::DragValue::new(v).range(min..=max).speed(0.01).update_while_editing(false)).changed();
                        sliding | typing
                    } else {
                        ui.add_sized(
                            [150. * zoom, 26. * zoom],
                            egui::DragValue::new(v).speed(0.01).update_while_editing(false),
                        )
                        .changed()
                    };
                });
            });
        }
        Value::Vector(v, color) => {
            let base = ui.max_rect().min;
            let width = ui.available_width();
            let row = |offset: f32| {
                egui::UiBuilder::new().max_rect(Rect::from_min_size(
                    base + egui::vec2(0., offset) * zoom,
                    egui::vec2(width, 26. * zoom),
                ))
            };
            ui.scope_builder(row(50.), |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(color, true, "RGBA");
                    ui.selectable_value(color, false, "XYZW");
                });
            });
            ui.scope_builder(row(80.), |ui| {
                ui.horizontal(|ui| {
                    ui.label(if *color { "RGB" } else { "XYZ" });
                    if *color {
                        // Alpha is a separate shader input: zero alpha must not erase RGB.
                        // The RGBA widget premultiplies and writes back even when untouched.
                        let mut rgb = [v[0], v[1], v[2]];
                        if ui.color_edit_button_rgb(&mut rgb).changed() {
                            v[..3].copy_from_slice(&rgb);
                            changed = true;
                        }
                    }
                });
            });
            for (i, value) in v.iter_mut().enumerate() {
                ui.scope_builder(row(110. + i as f32 * 30.), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(if *color {
                            ["R", "G", "B", "A"][i]
                        } else {
                            ["X", "Y", "Z", "W"][i]
                        });
                        changed |= ui
                            .add_sized(
                                [150. * zoom, 26. * zoom],
                                egui::DragValue::new(value).speed(0.01).update_while_editing(false),
                            )
                            .changed();
                    });
                });
            }
        }
        Value::Texture {
            path,
            default,
            st,
            unresolved,
            ..
        } => {
            let name = if *unresolved {
                tr("纹理引用未找到").to_owned()
            } else {
                path.as_ref()
                    .and_then(|p| p.file_name())
                    .map_or(default.clone(), |p| p.to_string_lossy().into_owned())
            };
            ui.add(egui::Label::new(name).truncate());
            ui.horizontal(|ui| {
                if let Some(image) = thumbnail {
                    ui.add(egui::Image::new(image).fit_to_exact_size(egui::vec2(64., 64.) * zoom));
                } else {
                    ui.allocate_exact_size(egui::vec2(64., 64.) * zoom, Sense::hover());
                }
                ui.vertical(|ui| {
                    if ui.button(tr("替换预览纹理")).clicked() {
                        if let Some(p) = rfd::FileDialog::new()
                            .add_filter(tr("纹理"), &["png", "jpg", "jpeg", "tga", "bmp", "webp"])
                            .pick_file()
                        {
                            *path = Some(p);
                            *unresolved = false;
                            changed = true;
                        }
                    }
                    if ui.button(tr("使用默认值")).clicked() {
                        *path = None;
                        *unresolved = false;
                        changed = true;
                    }
                });
            });
            for (offset, label) in [(0, tr("纹理缩放")), (2, tr("纹理偏移"))] {
                ui.horizontal(|ui| {
                    ui.label(label);
                    for i in 0..2 {
                        ui.label(["X", "Y"][i]);
                        changed |= ui
                            .add_sized(
                                [58. * zoom, 22. * zoom],
                                egui::DragValue::new(&mut st[offset + i]).speed(0.01).update_while_editing(false),
                            )
                            .changed();
                    }
                });
            }
        }
    }
    // TextEdit marks changes while typing even when DragValue has not committed.
    // Render invalidation must follow the actual input, not that UI signal.
    changed && input.value != before
}

fn resized_output_size(size: Vec2, screen_delta: Vec2, zoom: f32) -> Vec2 {
    (size + screen_delta / zoom.max(0.01)).clamp(egui::vec2(280., 300.), egui::vec2(1400., 1400.))
}
fn output_image_rect(output: Rect, zoom: f32, three_d: bool) -> Rect {
    let body = Rect::from_min_max(
        output.min + egui::vec2(12., if three_d { 108. } else { 78. }) * zoom,
        output.max - egui::vec2(12., 26.) * zoom,
    );
    let side = body.width().min(body.height()).max(1.);
    Rect::from_center_size(body.center(), egui::vec2(side, side))
}

pub(crate) struct Preview {
    areas: flow::Areas,
    mini: mini::Previews,
    probe: Option<probe::Probe>,
    pending_drag: bool,
    render_states: states::Editor,
    loaded: Option<Loaded>,
    loading: Option<mpsc::Receiver<Result<Loaded, String>>>,
    job: Option<mpsc::Receiver<(u64, Result<Arc<rs::Scene>, String>)>>,
    initial: Vec<Input>,
    scene: Option<Arc<rs::Scene>>,
    revision: u64,
    dirty: bool,
    error: Option<String>,
    gpu_error: Arc<Mutex<Option<String>>>,
    positions: HashMap<usize, Vec2>,
    thumbnails: HashMap<String, egui::TextureHandle>,
    pan: Vec2,
    zoom: f32,
    fit: bool,
    output_size: Vec2,
    three_d: bool,
    shape: geometry::Shape,
    angles: [f32; 2],
    mesh_scene: Option<geometry::Cached>,
    focus_output: bool,
    checkerboard: bool,
}
impl Preview {
    pub(crate) fn deactivate(&mut self) {
        // Preserve temporary values, but release inactive decoded images and
        // pending render results rather than retaining 128 MiB for every tab.
        self.thumbnails.clear();
        self.render_states.release();
        self.mini.clear();
        self.mesh_scene = None;
        let scene = self.scene.take();
        let job = self.job.take();
        if scene.is_some() || job.is_some() {
            self.dirty = self.loaded.is_some();
        }
    }
    pub(crate) fn new(ctx: &egui::Context, path: &Path, text: &str, catalog: Arc<Catalog>) -> Self {
        let (tx, rx) = mpsc::channel();
        let path = path.to_owned();
        let text = text.to_owned();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(load(&path, &text, &catalog));
            ctx.request_repaint();
        });
        Self {
            areas: Default::default(),
            mini: Default::default(),
            probe: None,
            pending_drag: false,
            render_states: Default::default(),
            loaded: None,
            loading: Some(rx),
            job: None,
            initial: Vec::new(),
            scene: None,
            revision: 0,
            dirty: false,
            error: None,
            gpu_error: Default::default(),
            positions: HashMap::new(),
            thumbnails: HashMap::new(),
            pan: Vec2::ZERO,
            zoom: 1.,
            fit: true,
            output_size: egui::vec2(480., 520.),
            three_d: false,
            shape: geometry::Shape::Sphere,
            angles: [0.6, 0.25],
            mesh_scene: None,
            focus_output: false,
            checkerboard: true,
        }
    }
    fn poll(&mut self, ctx: &egui::Context) {
        if let Some(result) = self.loading.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.loading = None;
            match result {
                Ok(data) => {
                    self.initial = data.inputs.clone();
                    self.loaded = Some(data);
                    self.dirty = true;
                }
                Err(e) => self.error = Some(e),
            }
        }
        if let Some((revision, result)) = self.job.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.job = None;
            if revision == self.revision {
                match result {
                    Ok(scene) => {
                        self.thumbnails.clear();
                        for (name, texture) in &scene.draws[0].material.textures {
                            let mut image = egui::ColorImage::new([64, 64], Color32::TRANSPARENT);
                            let [w, h] = texture.size.map(|v| v as usize);
                            for y in 0..64 {
                                for x in 0..64 {
                                    let index = ((h - 1 - y * h / 64) * w + x * w / 64) * 4;
                                    image.pixels[y * 64 + x] = crate::asset_viewer::preview_color(
                                        texture.rgba[index..index + 4].try_into().unwrap(),
                                    );
                                }
                            }
                            self.thumbnails.insert(
                                name.clone(),
                                ctx.load_texture(
                                    format!("material-node-{name}"),
                                    image,
                                    egui::TextureOptions::LINEAR,
                                ),
                            );
                        }
                        self.scene = Some(scene);
                        self.error = None;
                    }
                    Err(e) => {
                        self.scene = None;
                        self.error = Some(e);
                    }
                }
            }
        }
        if self.dirty && self.job.is_none() && !self.pending_drag && !ctx.input(|input| input.pointer.primary_down()) {
            if let Some(data) = &self.loaded {
                self.dirty = false;
                let shader = data.shader.clone();
                let material = data.material.clone();
                let inputs = data.inputs.clone();
                let revision = self.revision;
                let probe = self.probe.clone();
                let ctx = ctx.clone();
                let (tx, rx) = mpsc::channel();
                self.job = Some(rx);
                std::thread::spawn(move || {
                    let _ = tx.send((revision, prepare(&shader, &material, &inputs, revision, probe.as_ref())));
                    ctx.request_repaint();
                });
            }
        }
    }
    fn changed(&mut self) {
        self.mini.clear();
        self.invalidate_output();
    }
    fn invalidate_output(&mut self) {
        self.revision += 1;
        self.dirty = true;
        self.scene = None;
        self.render_states.release();
        self.mesh_scene = None;
        self.thumbnails.clear();
        *self.gpu_error.lock().unwrap() = None;
    }
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> bool {
        self.poll(ui.ctx());
        self.mini.poll(ui.ctx());
        let mut reload = false;
        ui.horizontal_wrapped(|ui| {
            ui.strong(tr("节点视图"));
            if ui.button(tr("刷新节点快照")).clicked(){self.mini.clear();}
            if self.probe.is_some() && ui.button(tr("最终输出")).clicked() {
                self.probe = None;
                self.invalidate_output();
            }
            if let Some(probe) = &mut self.probe {
                ui.label(format!("{} · {} {}", probe.function, tr("步骤"), probe.step + 1)).on_hover_text(&probe.target);
                let before = probe.alpha;
                ui.selectable_value(&mut probe.alpha, false, tr("数值"));
                ui.selectable_value(&mut probe.alpha, true, "A");
                if before != probe.alpha { self.invalidate_output(); }
            }
            if ui.button(tr("重置输入")).clicked() {
                if let Some(data) = &mut self.loaded {
                    data.inputs = self.initial.clone();
                    self.render_states.reset();
                    self.changed();
                }
            }
            if ui.button(tr("重新载入预览")).clicked() {
                reload = true;
            }
            if ui.button(tr("重置视图")).clicked() {
                self.pan = Vec2::ZERO;
                self.positions.clear();
                self.areas.clear();
                self.fit = true;
            }
            if ui.button(tr("定位输出")).clicked() {
                self.focus_output = true;
            }
            ui.menu_button(tr("预览说明"), |ui| {
                ui.set_max_width(340.);
                ui.label(tr("临时预览输入，不修改文件"));
                ui.label(tr(
                    "按 Vert / Frag / 函数分组展示源码；调用线不代表完整数据流",
                ));
                ui.label(tr("阶段接口按声明展示，TEXCOORD 通道不等同于模型 UV 通道"));
                ui.label(tr("单步：标量显示灰度，二维显示 RG，三维或四维显示 RGB；A 单独查看"));
                ui.label(tr("平面预览不包含场景光照、蒙版及自定义顶点流"));
            });
            ui.add_visible(self.loading.is_some() || self.job.is_some(), egui::Spinner::new());
            ui.weak(format!("{:.0}%", self.zoom * 100.));
        });
        if !render_system::available() {
            ui.colored_label(ui.visuals().warn_fg_color, tr("GPU 渲染不可用"));
        }
        if let Some(error) = &self.error {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                format!("{}: {error}", tr("预览不可用")),
            );
        }
        if let Some(error) = self.gpu_error.lock().unwrap().as_ref() {
            ui.colored_label(ui.visuals().warn_fg_color, error);
        }
        if self.loaded.is_none() {
            return reload;
        }
        if let Some(data) = &self.loaded {
            if let Some(warning) = &data.source_warning {
                ui.colored_label(ui.visuals().warn_fg_color, warning);
            }
            if data.stages.unresolved {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    tr("部分阶段入口无法唯一识别，相关连线未显示"),
                );
            }
            if data.stages.limited {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    tr("函数或语句展示达到上限，未显示部分请查看源码"),
                );
            }
            if data.stages.groups.is_empty() {
                ui.small(tr("阶段入口未解析，保留参数绑定视图"));
            }
        }
        ui.small(tr("左键选择 / 框选，拖动节点移动；右键平移画布；滚轮缩放；Shift 增减选择"));
        let mut changed = false;
        let (canvas, response) = ui.allocate_exact_size(
            egui::vec2(
                ui.available_width().max(1.),
                ui.available_height().max(240.),
            ),
            Sense::click_and_drag(),
        );
        self.areas.tools.begin(ui,canvas);
        let stage_destination=self.loaded.as_ref().filter(|data|!data.stages.groups.is_empty()).map_or(820.,|data|stages::destination_x(&data.stages));
        if self.fit {
            self.zoom = (canvas.width()
                / if self
                    .loaded
                    .as_ref()
                    .is_some_and(|d| !d.stages.groups.is_empty())
                {
                    stage_destination+880.
                } else {
                    1590.
                })
            .clamp(0.10, 1.);
            self.pan = Vec2::ZERO;
            self.fit = false;
        }
        let painter = ui.painter().with_clip_rect(canvas);
        painter.rect_filled(canvas, 0., ui.visuals().extreme_bg_color);
        let pan_delta=self.areas.tools.pan_delta(ui,canvas,&response);
        if pan_delta!=Vec2::ZERO {self.pan+=pan_delta;ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);ui.ctx().request_repaint();}
        if !ui.memory(|m|m.any_popup_open()) && ui.input(|i|i.pointer.hover_pos().is_some_and(|p|canvas.contains(p))) {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0. {
                let old = self.zoom;
                self.zoom = (old * (scroll * 0.002).exp()).clamp(0.10, 1.75);
                ui.input_mut(|i|{i.smooth_scroll_delta.y=0.;i.raw_scroll_delta.y=0.;});
                if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                    self.pan =
                        (self.pan - (pos - canvas.min)) * (self.zoom / old) + (pos - canvas.min);
                }
            }
        }
        // Graph grid remains anchored to graph coordinates while panning.
        let step = 24. * self.zoom;
        let grid_color = if ui.visuals().dark_mode {
            Color32::from_gray(37)
        } else {
            Color32::from_gray(224)
        };
        let offset = self.pan + egui::vec2(24., 24.);
        for x in 0..=(canvas.width() / step).ceil() as usize {
            let x = canvas.left() + offset.x.rem_euclid(step) + x as f32 * step;
            painter.line_segment(
                [egui::pos2(x, canvas.top()), egui::pos2(x, canvas.bottom())],
                Stroke::new(0.5_f32, grid_color),
            );
        }
        for y in 0..=(canvas.height() / step).ceil() as usize {
            let y = canvas.top() + offset.y.rem_euclid(step) + y as f32 * step;
            painter.line_segment(
                [egui::pos2(canvas.left(), y), egui::pos2(canvas.right(), y)],
                Stroke::new(0.5_f32, grid_color),
            );
        }
        if self.focus_output {
            let default = egui::vec2(
                if self
                    .loaded
                    .as_ref()
                    .is_some_and(|d| !d.stages.groups.is_empty())
                {
                    stage_destination+400.
                } else {
                    1220.
                },
                45.,
            );
            let point = self.positions.get(&usize::MAX).copied().unwrap_or(default);
            self.pan = canvas.center()
                - canvas.min
                - egui::vec2(24., 24.)
                - (point + self.output_size * 0.5) * self.zoom;
            self.focus_output = false;
        }
        let origin = canvas.min + self.pan + egui::vec2(24., 24.);
        let data = self.loaded.as_mut().unwrap();
        let grouped = !data.stages.groups.is_empty();
        let mut y = 0.;
        let mut inputs = Vec::new();
        for (index, input) in data.inputs.iter().enumerate() {
            let height = input_height(&input.value);
            let position = *self.positions.entry(index).or_insert(egui::vec2(0., y));
            inputs.push(Rect::from_min_size(
                origin + position * self.zoom,
                egui::vec2(300., height) * self.zoom,
            ));
            y += height + 24.;
        }
        let shader_position = *self
            .positions
            .entry(usize::MAX - 1)
            .or_insert(egui::vec2(420., 45.));
        let output_position = *self
            .positions
            .entry(usize::MAX)
            .or_insert(egui::vec2(stage_destination+400., 45.));
        let state_position = *self.positions.entry(usize::MAX - 2)
            .or_insert(egui::vec2(stage_destination, 45.));
        let state_rect = Rect::from_min_size(origin + state_position * self.zoom, egui::vec2(340., 340.) * self.zoom);
        let connections = data
            .inputs
            .iter()
            .enumerate()
            .flat_map(|(index, input)| {
                input
                    .bindings
                    .iter()
                    .map(move |(output, target)| (index, output.clone(), target.clone()))
            })
            .collect::<Vec<_>>();
        let shader = Rect::from_min_size(
            origin + shader_position * self.zoom,
            egui::vec2(290., (connections.len() as f32 * 28. + 88.).max(150.)) * self.zoom,
        );
        let mut output = Rect::from_min_size(
            origin + output_position * self.zoom,
            self.output_size * self.zoom,
        );
        let handle = Rect::from_min_size(
            output.right_bottom() - egui::vec2(18., 18.),
            egui::vec2(18., 18.),
        );
        if handle.intersects(canvas) {
            let resize = ui
                .interact(
                    handle.intersect(canvas),
                    response.id.with("output_node_resize"),
                    Sense::drag(),
                )
                .on_hover_cursor(egui::CursorIcon::ResizeNwSe);
            if resize.dragged_by(egui::PointerButton::Primary) {
                self.output_size =
                    resized_output_size(self.output_size, resize.drag_delta(), self.zoom);
                output = Rect::from_min_size(output.min, self.output_size * self.zoom);
            }
        }

        let port_y = |i: usize| shader.top() + (76. + i as f32 * 28.) * self.zoom;
        let line = |a: egui::Pos2, b: egui::Pos2, color: Color32| {
            painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
                wire_points(a, b, self.zoom),
                false,
                Color32::TRANSPARENT,
                Stroke::new(1.3_f32, color),
            ));
        };
        let mini_source=self.scene.as_ref().map(|scene|mini::Source{shader:data.shader.clone(),material:data.material.clone(),inputs:data.inputs.clone(),scene:scene.clone(),revision:self.revision});
        let mut step_request = None;
        if grouped {
            step_request = stages::draw(
                ui,
                &data.stages,
                &data.inputs,
                &inputs,
                origin,
                canvas,
                self.zoom,
                &mut self.positions,
                state_rect,
                self.probe.as_ref(),
                &mut self.areas,
                &mut self.mini,
                mini_source.as_ref(),
            );
        } else {
            for (port_index, (index, output, _)) in connections.iter().enumerate() {
                let input = &data.inputs[*index];
                line(
                    egui::pos2(
                        inputs[*index].right(),
                        inputs[*index].top() + output_y(input, output) * self.zoom,
                    ),
                    egui::pos2(shader.left(), port_y(port_index)),
                    input_color(&input.value),
                );
            }
            line(
                shader.right_center(),
                egui::pos2(state_rect.left(), state_rect.top() + 55. * self.zoom),
                Color32::from_rgb(125, 190, 220),
            );
        }
        let body_color = if ui.visuals().dark_mode {
            Color32::from_rgb(47, 48, 52)
        } else {
            Color32::from_rgb(245, 245, 248)
        };
        let header_color = if ui.visuals().dark_mode {
            Color32::from_rgb(34, 35, 39)
        } else {
            Color32::from_rgb(220, 220, 226)
        };
        let drop_position = crate::native_window::file_drop_position(ui.ctx());
        let (dropped_image, hovering_image) = ui.input(|state| {
            let dropped = if state.raw.dropped_files.len() == 1 {
                state.raw.dropped_files[0].path.clone().filter(|path| crate::asset_viewer::is_bitmap(path))
            } else { None };
            let hovering = state.raw.hovered_files.len() == 1 && state.raw.hovered_files[0]
                .path.as_ref().is_some_and(|path| crate::asset_viewer::is_bitmap(path));
            (dropped, hovering)
        });
        // Use this frame's transformed and clipped nodes, never stale screen rectangles.
        let drop_target = drop_position.and_then(|position| {
            if ui.ctx().layer_id_at(position).is_some_and(|layer| layer != ui.layer_id()) { return None; }
            if state_rect.intersect(canvas).contains(position) || output.intersect(canvas).contains(position) || (!grouped && shader.intersect(canvas).contains(position)) { return None; }
            data.inputs.iter().zip(&inputs).enumerate().rev().find_map(|(index, (input, rect))| {
                if rect.intersect(canvas).intersect(ui.clip_rect()).contains(position)
                    && matches!(&input.value, Value::Texture { kind, .. } if kind == "2D") {
                    Some(index)
                } else { None }
            })
        });
        for (index, (input, r)) in data.inputs.iter_mut().zip(&inputs).enumerate() {
            if drop_target == Some(index) {
                if let Some(path) = &dropped_image {
                    if let Value::Texture { path: texture, unresolved, .. } = &mut input.value {
                        *texture = Some(path.clone());
                        *unresolved = false;
                        changed = true;
                        // Consume only the claimed image; the app opens all other drops normally.
                        ui.ctx().input_mut(|state| state.raw.dropped_files.clear());
                    }
                }
            }
            let color = input_color(&input.value);
            painter.rect_filled(*r, 4., body_color);
            let title = Rect::from_min_size(r.min, egui::vec2(r.width(), 28. * self.zoom));
            painter.rect_filled(title, 4., header_color);
            painter.rect_filled(
                Rect::from_min_size(r.min, egui::vec2(4. * self.zoom, r.height())),
                2.,
                color,
            );
            painter.rect_stroke(*r,4.,Stroke::new(1.,Color32::from_gray(72)),egui::StrokeKind::Inside);
            painter.text(
                title.left_center() + egui::vec2(12. * self.zoom, 0.),
                egui::Align2::LEFT_CENTER,
                "Property",
                egui::FontId::proportional(14. * self.zoom),
                ui.visuals().text_color(),
            );
            self.areas.tools.node(ui,tools::Target::Position(index),*r,
                Rect::from_min_max(title.min,title.max-egui::vec2(54.*self.zoom,0.)),canvas,self.zoom);
            let reset = Rect::from_min_max(
                egui::pos2(
                    title.right() - 50. * self.zoom,
                    title.top() + 3. * self.zoom,
                ),
                title.max - egui::vec2(5., 3.) * self.zoom,
            );
            if reset.intersects(canvas) {
                let mut node_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt(("property_reset", index))
                        .max_rect(reset),
                );
                node_ui.set_clip_rect(reset.intersect(canvas));
                node_ui.style_mut().override_font_id =
                    Some(egui::FontId::proportional(11. * self.zoom));
                if node_ui
                    .add_sized(reset.size(), egui::Button::new(tr("重置")).small())
                    .clicked()
                {
                    if let Some(initial) = self.initial.get(index) {
                        input.value = initial.value.clone();
                        changed = true;
                    }
                }
            }
            let body = Rect::from_min_max(
                r.min + egui::vec2(12., 36.) * self.zoom,
                r.max - egui::vec2(10., 8.) * self.zoom,
            );
            if body.intersects(canvas) {
                // A new_child's auto IDs also depend on its parent's widget count.
                // Background status changes must not replace an active DragValue editor.
                let mut child = property_input_ui(ui, &input.name, body);
                child.set_clip_rect(body.intersect(canvas));
                let style = child.style_mut();
                for font in style.text_styles.values_mut() {
                    font.size *= self.zoom;
                }
                style.spacing.item_spacing *= self.zoom;
                style.spacing.interact_size *= self.zoom;
                style.spacing.button_padding *= self.zoom;
                changed |= edit_input(
                    &mut child,
                    input,
                    self.thumbnails.get(&input.name),
                    self.zoom,
                );
            }
            if hovering_image && drop_target == Some(index) {
                painter.rect_stroke(*r, 4., Stroke::new(2.5_f32, Color32::from_rgb(80, 190, 240)), egui::StrokeKind::Inside);
                let hint = Rect::from_min_size(r.left_bottom() - egui::vec2(0., 25. * self.zoom), egui::vec2(r.width(), 25. * self.zoom));
                painter.rect_filled(hint, 2., header_color);
                painter.text(hint.center(), egui::Align2::CENTER_CENTER, tr("松开图片替换预览纹理"), egui::FontId::proportional(12. * self.zoom), Color32::from_rgb(80, 190, 240));
            }
            let ports: Vec<&str> = match input.value {
                Value::Vector(_, true) => vec!["RGBA", "RGB", "R", "G", "B", "A"],
                Value::Vector(..) => vec!["XYZW", "XYZ", "X", "Y", "Z", "W"],
                _ => vec!["Out"],
            };
            for channel in ports {
                let port = egui::pos2(r.right(), r.top() + output_y(input, channel) * self.zoom);
                painter.circle_filled(port, 4. * self.zoom, color);
                painter.circle_stroke(
                    port,
                    4. * self.zoom,
                    Stroke::new(1.0_f32, Color32::from_gray(20)),
                );
                painter.text(
                    port - egui::vec2(9. * self.zoom, 0.),
                    egui::Align2::RIGHT_CENTER,
                    channel,
                    egui::FontId::proportional(11. * self.zoom),
                    color,
                );
            }
        }
        line(egui::pos2(state_rect.right(), state_rect.top() + 55. * self.zoom),
            egui::pos2(output.left(), output.top() + 55. * self.zoom), Color32::from_rgb(125, 190, 220));
        let boxes = if grouped {
            vec![(usize::MAX - 2, state_rect, "Render State"), (usize::MAX, output, tr("预览输出"))]
        } else {
            vec![
                (usize::MAX - 1, shader, "Shader"),
                (usize::MAX - 2, state_rect, "Render State"),
                (usize::MAX, output, tr("预览输出")),
            ]
        };
        for (id, rect, title) in boxes {
            painter.rect_filled(rect, 4., body_color);
            let header = Rect::from_min_size(rect.min, egui::vec2(rect.width(), 28. * self.zoom));
            painter.rect_filled(header, 4., header_color);
            painter.rect_stroke(
                rect,
                4.,
                Stroke::new(1.0_f32, Color32::from_gray(85)),
                egui::StrokeKind::Inside,
            );
            painter.text(
                header.left_center() + egui::vec2(12. * self.zoom, 0.),
                egui::Align2::LEFT_CENTER,
                title,
                egui::FontId::proportional(14. * self.zoom),
                ui.visuals().text_color(),
            );
            self.areas.tools.node(ui,tools::Target::Position(id),rect,header,canvas,self.zoom);

        }
        if state_rect.intersects(canvas) {
            let body = Rect::from_min_max(state_rect.min + egui::vec2(12., 36.) * self.zoom, state_rect.max - egui::vec2(12., 8.) * self.zoom);
            let mut controls = property_input_ui(ui, "render_state_controls", body);
            controls.set_clip_rect(state_rect.intersect(canvas));
            let style = controls.style_mut();
            for font in style.text_styles.values_mut() { font.size *= self.zoom; }
            style.spacing.item_spacing *= self.zoom;
            style.spacing.interact_size *= self.zoom;
            controls.add_enabled_ui(self.probe.is_none(), |ui| {
                self.render_states.show(ui, self.scene.as_ref(), &data.render_notes);
            });
            if self.probe.is_some() { controls.weak(tr("单步数值绕过 Blend，最终输出恢复原状态")); }
        }
        for point in [egui::pos2(state_rect.left(), state_rect.top() + 55. * self.zoom), egui::pos2(state_rect.right(), state_rect.top() + 55. * self.zoom)] {
            painter.circle_filled(point, 4. * self.zoom, Color32::from_rgb(125, 190, 220));
        }
        if !grouped {
            painter.with_clip_rect(shader.intersect(canvas)).text(
                shader.left_top() + egui::vec2(14., 42.) * self.zoom,
                egui::Align2::LEFT_TOP,
                data.shader
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy(),
                egui::FontId::proportional(13. * self.zoom),
                ui.visuals().text_color(),
            );
            for (port_index, (index, _, target)) in connections.iter().enumerate() {
                let input = &data.inputs[*index];
                let port = egui::pos2(shader.left(), port_y(port_index));
                painter.circle_filled(port, 4. * self.zoom, input_color(&input.value));
                painter.with_clip_rect(shader.intersect(canvas)).text(
                    port + egui::vec2(12. * self.zoom, 0.),
                    egui::Align2::LEFT_CENTER,
                    format!("{} · {}", input.name, target),
                    egui::FontId::proportional(12. * self.zoom),
                    ui.visuals().text_color(),
                );
            }
            painter.circle_filled(
                shader.right_center(),
                4. * self.zoom,
                Color32::from_rgb(125, 190, 220),
            );
        }
        painter.circle_filled(
            egui::pos2(output.left(), output.top() + 55. * self.zoom),
            4. * self.zoom,
            Color32::from_rgb(125, 190, 220),
        );
        if let Some(probe) = step_request {
            self.probe = Some(probe);
            self.pending_drag = false;
            self.invalidate_output();
        }
        // Update after all node widgets have finished borrowing their inputs.
        let (down, released) = ui.input(|input| (input.pointer.primary_down(), input.pointer.button_released(egui::PointerButton::Primary)));
        if commit_preview_change(&mut self.pending_drag, changed, down, released) {
            self.changed();
        }
        if !output.intersects(canvas) {
            // Output culling must not skip the graph's input/position commit.
            self.areas.finish_tools(ui,canvas,&response,self.zoom,&mut self.positions);
            return reload;
        }
        let controls = Rect::from_min_size(
            output.min + egui::vec2(12., 36.) * self.zoom,
            egui::vec2(output.width() - 24. * self.zoom, 26. * self.zoom),
        );
        let mut output_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("output_node_controls")
                .max_rect(controls),
        );
        output_ui.set_clip_rect(output.intersect(canvas));
        let style = output_ui.style_mut();
        for font in style.text_styles.values_mut() {
            font.size *= self.zoom;
        }
        style.spacing.item_spacing *= self.zoom;
        style.spacing.interact_size *= self.zoom;
        style.spacing.button_padding *= self.zoom;
        output_ui.horizontal(|ui| {
            crate::theme::preview_space_selector(ui, &mut self.three_d);
            if let Some(probe) = &self.probe {
                ui.label(tr(if probe.vertex {"Vert 顶点值（插值显示）"} else {"单步数值（不透明）"}));
            }
            egui::ComboBox::from_id_salt("output_background")
                .width(100. * self.zoom)
                .selected_text(tr(if self.checkerboard {
                    "棋盘格"
                } else {
                    "纯色"
                }))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.checkerboard, true, tr("棋盘格"));
                    ui.selectable_value(&mut self.checkerboard, false, tr("纯色"));
                });
        });
        if self.three_d {
            let row = Rect::from_min_size(
                controls.min + egui::vec2(0., 32.) * self.zoom,
                controls.size(),
            );
            output_ui.scope_builder(egui::UiBuilder::new().max_rect(row), |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("模型"));
                    let name = match self.shape {
                        geometry::Shape::Sphere => tr("球体"),
                        geometry::Shape::Cylinder => tr("圆柱体"),
                        geometry::Shape::Cube => tr("立方体"),
                    };
                    egui::ComboBox::from_id_salt("output_shape")
                        .width(110. * self.zoom)
                        .selected_text(name)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.shape,
                                geometry::Shape::Sphere,
                                tr("球体"),
                            );
                            ui.selectable_value(
                                &mut self.shape,
                                geometry::Shape::Cylinder,
                                tr("圆柱体"),
                            );
                            ui.selectable_value(
                                &mut self.shape,
                                geometry::Shape::Cube,
                                tr("立方体"),
                            );
                        });
                    if ui
                        .small_button(tr("重置"))
                        .on_hover_text(tr("重置旋转"))
                        .clicked()
                    {
                        self.angles = [0.6, 0.25];
                    }
                });
            });
        }
        let image = output_image_rect(output, self.zoom, self.three_d);
        let painter = ui.painter().with_clip_rect(image.intersect(canvas));
        painter.rect_filled(image, 0., Color32::from_gray(90));
        let cell = 12.;
        if self.checkerboard {
            for y in 0..((image.height() / cell).ceil() as i32) {
                for x in 0..((image.width() / cell).ceil() as i32) {
                    let square = Rect::from_min_size(
                        image.min + egui::vec2(x as f32 * cell, y as f32 * cell),
                        egui::vec2(cell, cell),
                    )
                    .intersect(image);
                    painter.rect_filled(
                        square,
                        0.,
                        if (x + y) % 2 == 0 {
                            Color32::from_gray(130)
                        } else {
                            Color32::from_gray(190)
                        },
                    );
                }
            }
        }
        if image.intersects(canvas) {
            let rotate = ui
                .interact(
                    image.intersect(canvas),
                    response.id.with("output_model_orbit"),
                    Sense::drag(),
                )
                .on_hover_cursor(if self.three_d {
                    egui::CursorIcon::Grab
                } else {
                    egui::CursorIcon::Default
                })
                .on_hover_text(if self.three_d {
                    tr("拖动旋转模型")
                } else {
                    tr("平面")
                });
            if self.three_d && rotate.dragged_by(egui::PointerButton::Primary) {
                self.angles[0] = (self.angles[0] + rotate.drag_delta().x * 0.012)
                    .rem_euclid(std::f32::consts::TAU);
                self.angles[1] = (self.angles[1] + rotate.drag_delta().y * 0.012).clamp(-1.5, 1.5);
            }
        }
        let gpu_failed = self.gpu_error.lock().unwrap().is_some();
        let prepared = self.scene.as_ref().filter(|_| !gpu_failed).map(|base| {
            if self.probe.is_some() { base.clone() } else { self.render_states.scene(base) }
        });
        let display = prepared.as_ref().map(|base| {
            if !self.three_d {
                self.mesh_scene = None;
                return base.clone();
            }
            if self.mesh_scene.as_ref().is_none_or(|cache| {
                !Arc::ptr_eq(&cache.source, base)
                    || cache.shape != self.shape
                    || cache.angles != self.angles
            }) {
                self.mesh_scene = Some(geometry::Cached {
                    source: base.clone(),
                    shape: self.shape,
                    angles: self.angles,
                    scene: geometry::scene(base, self.shape, self.angles),
                });
            }
            self.mesh_scene.as_ref().unwrap().scene.clone()
        });
        if let Some(scene) = display {
            if render_system::available() && image.intersects(canvas) {
                let viewport = image.intersect(canvas);
                let mut projection = IDENTITY;
                if self.three_d {
                    for i in 0..3 {
                        projection[i][i] = 0.78;
                    }
                }
                let projection =
                    render_system::fixed_viewport_projection(projection, image, viewport);
                painter.add(egui::Shape::Callback(render_system::callback(
                    viewport,
                    scene.clone(),
                    projection,
                    self.gpu_error.clone(),
                    true,
                    self.revision,
                )));
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(33));
            }
        } else {
            painter.text(
                image.center(),
                egui::Align2::CENTER_CENTER,
                tr("预览不可用"),
                egui::FontId::proportional(13. * self.zoom),
                ui.visuals().weak_text_color(),
            );
        }
        let grip = ui.painter().with_clip_rect(output.intersect(canvas));
        for step in [4., 9., 14.] {
            grip.line_segment(
                [
                    output.right_bottom() - egui::vec2(step, 2.),
                    output.right_bottom() - egui::vec2(2., step),
                ],
                Stroke::new(1.2_f32, ui.visuals().text_color()),
            );
        }
        self.areas.finish_tools(ui,canvas,&response,self.zoom,&mut self.positions);
        reload
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn wires_keep_horizontal_ports_and_scale_with_canvas() {
        let a = egui::pos2(0., 600.);
        let b = egui::pos2(100., 0.);
        let points = super::wire_points(a, b, 1.);
        assert_eq!(points[1].y, a.y);
        assert_eq!(points[2].y, b.y);
        assert!(points[1].x > a.x && points[1].x < b.x);
        assert!(points[2].x > a.x && points[2].x < b.x);
        let small = super::wire_points(a * 0.5, b * 0.5, 0.5);
        for i in 0..4 { assert_eq!(small[i], points[i] * 0.5); }
    }
    use super::*;
    #[test]
    fn dragging_commits_once_on_release_and_keyboard_commit_is_immediate() {
        let mut pending = false;
        assert!(!commit_preview_change(&mut pending, false, true, false));
        assert!(!commit_preview_change(&mut pending, true, true, false));
        assert!(!commit_preview_change(&mut pending, true, true, false));
        assert!(!commit_preview_change(&mut pending, false, true, false));
        assert!(commit_preview_change(&mut pending, false, false, true));
        assert!(!commit_preview_change(&mut pending, false, false, false));
        assert!(commit_preview_change(&mut pending, true, false, false));
    }
    #[test]
    fn property_editor_ids_survive_background_status_widgets() {
        let ctx = egui::Context::default();
        let mut ids = Vec::new();
        for busy in [false, true, false] {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    if busy { ui.spinner(); ui.label("Preparing preview"); }
                    let mut editor = property_input_ui(ui, "_OverlayColor", ui.available_rect_before_wrap());
                    editor.horizontal(|ui| { ui.label("R"); ids.push(ui.next_auto_id()); });
                });
            });
        }
        assert_eq!(ids[0], ids[1]);
        assert_eq!(ids[1], ids[2]);
    }
    #[test]
    fn displaying_color_nodes_preserves_transparent_rgb_and_hdr_values() {
        for value in [[0.5, 0.5, 0.5, 0.], [0.2, 0.4, 0.8, 0.5], [2., 1.5, 0.25, 0.]] {
            let ctx = egui::Context::default();
            let mut input = Input {
                name: "_OverlayColor".into(), label: "Overlay Color".into(),
                value: Value::Vector(value, true), integer: false,
                shader_type: "Color".into(), bindings: vec![],
            };
            for _ in 0..2 {
                let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500., 500.))), ..Default::default() };
                let _ = ctx.run(raw, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        assert!(!edit_input(ui, &mut input, None, 1.));
                    });
                });
                assert_eq!(input.value, Value::Vector(value, true));
            }
        }
    }
    #[test]
    fn output_resize_tracks_graph_zoom_and_clamps_size() {
        assert_eq!(
            resized_output_size(egui::vec2(480., 520.), egui::vec2(20., 30.), 0.5),
            egui::vec2(520., 580.)
        );
        assert_eq!(
            resized_output_size(egui::vec2(480., 520.), egui::vec2(-10000., -10000.), 1.),
            egui::vec2(280., 300.)
        );
        assert_eq!(
            resized_output_size(egui::vec2(480., 520.), egui::vec2(10000., 10000.), 1.),
            egui::vec2(1400., 1400.)
        );
    }
    #[test]
    fn image_stays_inside_output_node_for_both_modes() {
        for zoom in [0.10, 0.35, 1., 1.75] {
            for three_d in [false, true] {
                let output =
                    Rect::from_min_size(egui::pos2(100., 60.), egui::vec2(480., 520.) * zoom);
                let image = output_image_rect(output, zoom, three_d);
                assert!(output.contains_rect(image));
                assert!((image.width() - image.height()).abs() < 0.001);
            }
        }
    }
    #[test]
    fn vector_semantics_follow_context_not_precision() {
        let source = r#"Properties { _C("C",Vector)=(1,1,1,1) _V("V",Vector)=(1,2,3,4) _Explicit("E",Color)=(1,1,1,1) _Alpha("A",Float)=1 }
        half4 _C; float4 _V; half4 _Explicit; float _Alpha;
        half4 frag(){half4 color=half4(_C.rgb,_C.a); color.a *= _Alpha; return color + _V.xyzw + _Explicit.xyzw;}"#;
        let inputs = properties(source).unwrap();
        assert!(matches!(inputs[0].value, Value::Vector(_, true)));
        assert_eq!(inputs[0].shader_type, "half4");
        assert!(matches!(inputs[1].value, Value::Vector(_, false)));
        assert!(matches!(inputs[2].value, Value::Vector(_, true)));
        assert!(inputs[0].bindings.contains(&("RGB".into(), "RGB".into())));
        assert!(inputs[0].bindings.contains(&("A".into(), "A".into())));
        assert!(inputs[3].bindings.contains(&("Out".into(), "A".into())));
        assert!(output_y(&inputs[0], "R") < output_y(&inputs[0], "G"));
        assert!(output_y(&inputs[0], "G") < output_y(&inputs[0], "B"));
        assert!(output_y(&inputs[0], "B") < output_y(&inputs[0], "A"));
    }
    #[test]
    fn unused_properties_do_not_invent_connections() {
        let inputs =
            properties("Properties { _Unused(\"Unused\",Float)=1 } float _Unused;").unwrap();
        assert!(inputs[0].bindings.is_empty());
    }
    #[test]
    fn preview_entry_is_available_for_materials_and_shaders() {
        for path in ["cloud.mat", "effect.shader", "Cloud.MAT", "Effect.SHADER"] {
            assert!(available(Some(Path::new(path))));
        }
        assert!(!available(None));
        assert!(!available(Some(Path::new("notes.txt"))));
    }
    #[test]
    fn existing_builtin_fixture_properties_are_accepted() {
        let inputs =
            properties(include_str!("../tests/fixtures/pf_sprite_overlay.shader")).unwrap();
        assert_eq!(inputs.len(), 4);
        assert_eq!(inputs[2].value, Value::Float(0., None));
        assert!(matches!(&inputs[0].value,Value::Texture{kind,..} if kind=="2D"));
    }
    #[test]
    fn obsolete_preparation_does_not_replace_current_inputs() {
        let ctx = egui::Context::default();
        let (tx, rx) = mpsc::channel();
        let mut preview = Preview {
            areas: Default::default(),
            mini: Default::default(),
            probe: None,
            pending_drag: false,
            render_states: Default::default(),
            loaded: None,
            loading: None,
            job: Some(rx),
            initial: vec![],
            scene: None,
            revision: 2,
            dirty: false,
            error: Some("current".into()),
            gpu_error: Default::default(),
            positions: HashMap::new(),
            thumbnails: HashMap::new(),
            pan: Vec2::ZERO,
            zoom: 1.,
            fit: true,
            output_size: egui::vec2(480., 520.),
            three_d: false,
            shape: geometry::Shape::Sphere,
            angles: [0.6, 0.25],
            mesh_scene: None,
            focus_output: false,
            checkerboard: true,
        };
        tx.send((1, Err("obsolete".into()))).unwrap();
        preview.poll(&ctx);
        assert_eq!(preview.error.as_deref(), Some("current"));
        assert!(preview.scene.is_none());
    }
    #[test]
    fn defaults_and_material_overrides_are_separate() {
        let source = r#"Shader "X" { Properties { [HDR] _Tint ("Tint", Color) = (1, 0.5, 0.2, 1)
            _Amount ("Amount", Range(-1, 2)) = 0.3
            _MainTex ("Texture", 2D) = "white" {} } }"#;
        let original = properties(source).unwrap();
        let mut preview = original.clone();
        material_inputs(
            &mut preview,
            "  m_Floats:\n  - _Amount: 0.8\n  m_Colors:\n  - _Tint: {r: 0, g: 1, b: 0, a: 1}\n",
            &Catalog::new(),
        )
        .unwrap();
        assert_eq!(original[1].value, Value::Float(0.3, Some([-1., 2.])));
        assert_eq!(preview[1].value, Value::Float(0.8, Some([-1., 2.])));
        assert_eq!(preview[0].value, Value::Vector([0., 1., 0., 1.], true));
    }
    #[test]
    fn comments_braces_and_multiline_properties() {
        let p=properties("// Properties { bad }\nProperties { _X (\"} // label\",\n Float) = 1e-2 /* } */ _V (\"V\", Vector) = (1,2,3,4) }").unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].value, Value::Float(0.01, None));
        assert!(properties("Properties { _X (\"X\", Float) = nan }").is_err());
    }
    #[test]
    fn texture_binding_preserves_transform_and_missing_reference() {
        let mut inputs = properties("Properties { _MainTex (\"T\", 2D) = \"white\" {} }").unwrap();
        material_inputs(&mut inputs,"  m_TexEnvs:\n  - _MainTex:\n      m_Texture: {fileID: 2800000, guid: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, type: 3}\n      m_Scale: {x: 2, y: 3}\n      m_Offset: {x: 0.1, y: 0.2}\n  m_Floats: []\n",&Catalog::new()).unwrap();
        assert!(
            matches!(&inputs[0].value,Value::Texture{unresolved:true,st,..} if *st==[2.,3.,0.1,0.2])
        );
    }
}
