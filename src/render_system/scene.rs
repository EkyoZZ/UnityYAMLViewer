//! Backend-independent real-time draw data. Asset parsing and shader compilation
//! happen before publishing a scene; a frame changes only uniforms and camera.
// The API is being prepared before the GPU backend and preview migration.
#![allow(dead_code)]
use std::{collections::BTreeMap, sync::Arc, time::Instant};

/// Straight RGBA, deliberately distinct from egui's premultiplied Color32.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub normal: [f32;3],
    pub tangent: [f32;4],
}

#[derive(Clone, Debug)]
pub(crate) struct Texture {
    pub size: [u32; 2],
    pub rgba: Arc<[u8]>,
    pub revision: u64,
    pub repeat: [bool;2],
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Uniform {
    Float(f32),
    Vector([f32; 4]),
    Matrix([[f32; 4]; 4]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlendFactor {
    Zero, One, SourceAlpha, OneMinusSourceAlpha, DestinationColor,
    SourceColor, OneMinusSourceColor, DestinationAlpha, OneMinusDestinationAlpha,
    OneMinusDestinationColor, SourceAlphaSaturate,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RenderState {
    pub source: BlendFactor,
    pub destination: BlendFactor,
    pub color_mask: [bool; 4],
    pub depth_write: bool,
    pub cull_back: bool,
}

/// Compiled programs, not interpreted formulas or shader-name special cases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnityPipeline {
    BuiltIn,
    Universal,
    Lightweight,
    Unknown,
}

#[derive(Clone, Debug)]
pub(crate) struct Program {
    pub pipeline: UnityPipeline,
    /// GLSL after the pipeline-specific Unity compatibility / compilation step.
    pub vertex: Arc<str>,
    pub fragment: Arc<str>,
    pub revision: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct Material {
    pub program: Arc<Program>,
    pub state: RenderState,
    pub uniforms: BTreeMap<String, Uniform>,
    pub textures: BTreeMap<String, Arc<Texture>>,
}

#[derive(Clone, Debug)]
pub(crate) struct Draw {
    /// Input previews must report shader failures instead of drawing a fallback.
    pub strict_shader: bool,
    pub component: Arc<str>,
    pub vertices: Arc<[Vertex]>,
    pub indices: Arc<[u32]>,
    pub object_to_world: [[f32; 4]; 4],
    pub material: Arc<Material>,
    /// SpriteRenderer supplies the texture independently of Material._MainTex.
    pub main_texture: Arc<Texture>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Scene {
    /// Preserve prepared Unity sorting order, including transparent objects.
    pub draws: Arc<[Draw]>,
    pub revision: u64,
}

pub(crate) struct Frame<'a> {
    pub scene: &'a Scene,
    pub viewport: [u32; 2],
    pub view_projection: [[f32; 4]; 4],
    pub clear_color: [f32; 4],
    pub elapsed_seconds: f32,
    pub globals: &'a BTreeMap<String, Uniform>,
}

/// GPU implementation owns targets, textures and compiled-program caches.
/// Rendering must blend against the current scene target, not an isolated
/// transparent sprite image. This is necessary for DstColor blending.
pub(crate) trait Backend {
    type Output;
    fn render(&mut self, frame: &Frame<'_>) -> Result<Self::Output, String>;
}

pub(crate) struct Clock {
    started: Instant,
}
impl Default for Clock {
    fn default() -> Self { Self { started: Instant::now() } }
}
impl Clock {
    pub fn elapsed_seconds(&self) -> f32 { self.started.elapsed().as_secs_f32() }
}

