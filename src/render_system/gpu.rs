//! GLSL program and straight-RGBA texture cache for the real-time Glow backend.
//! All methods, including destruction, require the owning GL context current.
#![allow(dead_code)]
use std::{collections::HashMap, sync::Arc};
use eframe::glow::{self, HasContext};
use super::scene::{Program, Texture, Uniform, Frame, BlendFactor, Draw, Vertex};

struct CachedProgram { source: Arc<Program>, handle: glow::Program }
struct CachedTexture { source: Arc<Texture>, handle: glow::Texture }
struct FailedProgram { source:Arc<Program>, error:String }
struct CachedMesh { vertices:Arc<[Vertex]>, indices:Arc<[u32]>, vao:glow::VertexArray, vbo:glow::Buffer, ebo:glow::Buffer }
struct Snapshot {scene:Arc<super::scene::Scene>,projection:[[f32;4];4],viewport:[i32;4],style:u64,texture:glow::Texture,framebuffer:glow::Framebuffer,vao:glow::VertexArray}
pub(crate) struct Gpu {
    gl: Arc<glow::Context>,
    programs: HashMap<usize, CachedProgram>,
    textures: HashMap<usize, CachedTexture>,
    meshes: HashMap<(usize,usize), CachedMesh>,
    failed:HashMap<usize,FailedProgram>,
    fallback:Arc<Program>,
    copy:Arc<Program>,
    snapshot:Option<Snapshot>,
    pub warnings:Vec<String>,
}

impl Gpu {
    pub fn snapshot_ready(&self,scene:&Arc<super::scene::Scene>)->bool {self.snapshot.as_ref().is_some_and(|cache|Arc::ptr_eq(&cache.scene,scene))}
    pub unsafe fn draw_cached(&mut self,frame:&Frame<'_>,scene:Arc<super::scene::Scene>,viewport:[i32;4],active:bool,style:u64)->Result<(),String> {
        let fresh=self.snapshot.as_ref().is_some_and(|cache|Arc::ptr_eq(&cache.scene,&scene) && cache.style==style);
        unsafe {
            // A clipped viewport can round to zero physical pixels. It is not a shader error.
            if viewport[2] <= 0 || viewport[3] <= 0 { return Ok(()); }
            let max_size = self.gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE);
            if !snapshot_size_supported(viewport, max_size) {
                self.drop_snapshot();
                return self.draw(frame); // Cache is optional; keep rendering the original shader.
            }
            if active || !fresh {
                self.draw(frame)?;
                self.capture(scene,frame.view_projection,viewport,style)?;
                return Ok(());
            }
            let program=self.program(&self.copy.clone())?;
            let cache=self.snapshot.as_ref().unwrap();
            let gl=&self.gl;
            gl.disable(glow::DEPTH_TEST);gl.disable(glow::CULL_FACE);gl.disable(glow::FRAMEBUFFER_SRGB);gl.disable(glow::BLEND);
            gl.color_mask(true,true,true,true);
            gl.use_program(Some(program));gl.bind_vertex_array(Some(cache.vao));gl.active_texture(glow::TEXTURE0);gl.bind_texture(glow::TEXTURE_2D,Some(cache.texture));
            gl.uniform_1_i32(gl.get_uniform_location(program,"preview_image").as_ref(),0);
            let scale=frozen_scale([cache.viewport[2],cache.viewport[3]],[viewport[2],viewport[3]]);
            gl.uniform_2_f32(gl.get_uniform_location(program,"preview_scale").as_ref(),scale[0],scale[1]);
            gl.draw_arrays(glow::TRIANGLES,0,3);gl.bind_vertex_array(None);
            Ok(())
        }
    }
    unsafe fn capture(&mut self,scene:Arc<super::scene::Scene>,projection:[[f32;4];4],viewport:[i32;4],style:u64)->Result<(),String> {
        let [x,y,w,h]=viewport;let gl=self.gl.clone();
        if w<=0 || h<=0 || w>unsafe{gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE)} || h>unsafe{gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE)} {return Err("Preview snapshot size unsupported".into());}
        unsafe {
            let read=gl.get_parameter_framebuffer(glow::READ_FRAMEBUFFER_BINDING);let draw=gl.get_parameter_framebuffer(glow::DRAW_FRAMEBUFFER_BINDING);
            let scissor=gl.is_enabled(glow::SCISSOR_TEST);gl.disable(glow::SCISSOR_TEST);
            let result=(|| {
                if self.snapshot.as_ref().is_none_or(|cache|cache.viewport[2..]!=viewport[2..]) {
                    self.drop_snapshot();
                    let texture=gl.create_texture()?;
                    let previous_texture=gl.get_parameter_texture(glow::TEXTURE_BINDING_2D);
                    gl.bind_texture(glow::TEXTURE_2D,Some(texture));
                    gl.tex_parameter_i32(glow::TEXTURE_2D,glow::TEXTURE_MIN_FILTER,glow::NEAREST as i32);gl.tex_parameter_i32(glow::TEXTURE_2D,glow::TEXTURE_MAG_FILTER,glow::NEAREST as i32);
                    gl.tex_parameter_i32(glow::TEXTURE_2D,glow::TEXTURE_WRAP_S,glow::CLAMP_TO_EDGE as i32);gl.tex_parameter_i32(glow::TEXTURE_2D,glow::TEXTURE_WRAP_T,glow::CLAMP_TO_EDGE as i32);
                    gl.tex_image_2d(glow::TEXTURE_2D,0,glow::RGBA8 as i32,w,h,0,glow::RGBA,glow::UNSIGNED_BYTE,glow::PixelUnpackData::Slice(None));
                    gl.bind_texture(glow::TEXTURE_2D,previous_texture);
                    let framebuffer=match gl.create_framebuffer(){Ok(v)=>v,Err(e)=>{gl.delete_texture(texture);return Err(e);}};
                    gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER,Some(framebuffer));
                    gl.framebuffer_texture_2d(glow::DRAW_FRAMEBUFFER,glow::COLOR_ATTACHMENT0,glow::TEXTURE_2D,Some(texture),0);
                    if gl.check_framebuffer_status(glow::DRAW_FRAMEBUFFER)!=glow::FRAMEBUFFER_COMPLETE {gl.delete_framebuffer(framebuffer);gl.delete_texture(texture);return Err("Preview snapshot framebuffer incomplete".into());}
                    let vao=match gl.create_vertex_array(){Ok(v)=>v,Err(e)=>{gl.delete_framebuffer(framebuffer);gl.delete_texture(texture);return Err(e);}};
                    self.snapshot=Some(Snapshot{scene:scene.clone(),projection,viewport,style,texture,framebuffer,vao});
                }
                let cache=self.snapshot.as_mut().unwrap();
                gl.bind_framebuffer(glow::READ_FRAMEBUFFER,draw);gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER,Some(cache.framebuffer));
                gl.blit_framebuffer(x,y,x+w,y+h,0,0,w,h,glow::COLOR_BUFFER_BIT,glow::NEAREST);
                cache.scene=scene;cache.projection=projection;cache.viewport=viewport;cache.style=style;
                Ok(())
            })();
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER,read);gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER,draw);if scissor {gl.enable(glow::SCISSOR_TEST);}
            result
        }
    }
    unsafe fn drop_snapshot(&mut self) {if let Some(cache)=self.snapshot.take(){unsafe {self.gl.delete_texture(cache.texture);self.gl.delete_framebuffer(cache.framebuffer);self.gl.delete_vertex_array(cache.vao);}}}
    /// Render to the caller's current framebuffer and viewport. Intended for
    /// egui_glow PaintCallback; egui restores its own drawing state afterwards.
    /// The callback must supply a viewport/scissor and background first.
    pub unsafe fn draw(&mut self, frame: &Frame<'_>) -> Result<(), String> {
        let gl = self.gl.clone();
        unsafe {
            let result = (|| {
                self.warnings.clear();let mut warned=std::collections::HashSet::new();
                gl.disable(glow::CULL_FACE); gl.disable(glow::DEPTH_TEST);
                gl.disable(glow::FRAMEBUFFER_SRGB);
                gl.enable(glow::BLEND); gl.blend_equation(glow::FUNC_ADD);
                gl.active_texture(glow::TEXTURE0);
                for draw in frame.scene.draws.iter() {
                    if draw.indices.len()>i32::MAX as usize || draw.indices.iter().any(|i|*i as usize>=draw.vertices.len()) {
                        return Err(format!("Invalid mesh indices: {}",draw.component));
                    }
                    let key=Arc::as_ptr(&draw.material.program) as usize;
                    let compiled=if let Some(failed)=self.failed.get(&key){Err(failed.error.clone())}else{self.program(&draw.material.program)};
                    let (program,fallback)=match compiled {
                        Ok(program)=>(program,false),
                        Err(error)=>{
                            self.failed.entry(key).or_insert_with(||FailedProgram{source:draw.material.program.clone(),error:error.clone()});
                            if draw.strict_shader {return Err(error);}
                            if warned.insert(key){self.warnings.push(format!("Shader GPU · component {}: {error}; ordinary texture fallback",draw.component));}
                            (self.program(&self.fallback.clone())?,true)
                        }
                    };
                    let texture=self.texture(&draw.main_texture)?;
                    let vao=self.mesh(draw)?;
                    gl.bind_vertex_array(Some(vao));
                    gl.use_program(Some(program));
                    gl.bind_texture(glow::TEXTURE_2D,Some(texture));
                    gl.uniform_1_i32(gl.get_uniform_location(program,"_MainTex").as_ref(),0);
                    let max_units=gl.get_parameter_i32(glow::MAX_TEXTURE_IMAGE_UNITS) as usize;
                    if draw.material.textures.len()+1>max_units {return Err("Material exceeds GPU texture unit count".into());}
                    for (unit,(name,source)) in draw.material.textures.iter().filter(|_|!fallback).enumerate() {
                        let texture=self.texture(source)?;
                        gl.active_texture(glow::TEXTURE0+unit as u32+1);
                        gl.bind_texture(glow::TEXTURE_2D,Some(texture));
                        gl.uniform_1_i32(gl.get_uniform_location(program,name).as_ref(),unit as i32+1);
                    }
                    gl.active_texture(glow::TEXTURE0);
                    // Clear uniform values each draw: shared programs must not
                    // inherit another material's property values.
                    for (name,value) in frame.globals { self.uniform(program,name,value); }
                    for (name,value) in &draw.material.uniforms { self.uniform(program,name,value); }
                    let main=draw.material.textures.get("_MainTex").unwrap_or(&draw.main_texture);
                    let [width,height]=main.size.map(|v|v as f32);
                    self.uniform(program,"_MainTex_TexelSize",&Uniform::Vector([1.0/width,1.0/height,width,height]));
                    self.uniform(program,"unity_ObjectToWorld",&Uniform::Matrix(draw.object_to_world));
                    self.uniform(program,"unity_MatrixVP",&Uniform::Matrix(frame.view_projection));
                    self.uniform(program,"_Time",&Uniform::Vector([frame.elapsed_seconds/20.0,frame.elapsed_seconds,frame.elapsed_seconds*2.0,frame.elapsed_seconds*3.0]));
                    let factor=|factor|match factor {
                        BlendFactor::Zero=>glow::ZERO, BlendFactor::One=>glow::ONE,
                        BlendFactor::SourceAlpha=>glow::SRC_ALPHA,
                        BlendFactor::OneMinusSourceAlpha=>glow::ONE_MINUS_SRC_ALPHA,
                        BlendFactor::DestinationColor=>glow::DST_COLOR,
                        BlendFactor::SourceColor=>glow::SRC_COLOR,
                        BlendFactor::OneMinusSourceColor=>glow::ONE_MINUS_SRC_COLOR,
                        BlendFactor::DestinationAlpha=>glow::DST_ALPHA,
                        BlendFactor::OneMinusDestinationAlpha=>glow::ONE_MINUS_DST_ALPHA,
                        BlendFactor::OneMinusDestinationColor=>glow::ONE_MINUS_DST_COLOR,
                        BlendFactor::SourceAlphaSaturate=>glow::SRC_ALPHA_SATURATE,
                    };
                    let state=if fallback {super::scene::RenderState{source:BlendFactor::One,destination:BlendFactor::OneMinusSourceAlpha,color_mask:[true;4],depth_write:false,cull_back:false}}else{draw.material.state};
                    if state.cull_back {gl.enable(glow::CULL_FACE);gl.cull_face(glow::BACK);gl.front_face(glow::CW);}else{gl.disable(glow::CULL_FACE);}
                    gl.blend_func(factor(state.source),factor(state.destination));
                    let [r,g,b,a]=state.color_mask;gl.color_mask(r,g,b,a);
                    gl.depth_mask(state.depth_write);
                    gl.draw_elements(glow::TRIANGLES,draw.indices.len() as i32,glow::UNSIGNED_INT,0);
                }
                Ok(())
            })();
            gl.color_mask(true,true,true,true);gl.depth_mask(true);
            gl.front_face(glow::CCW);
            gl.bind_vertex_array(None);
            // Multiple callbacks may render live output and node snapshots in one frame.
            // Retain resources still owned by another prepared scene, not just this draw.
            let programs:std::collections::HashSet<_>=frame.scene.draws.iter().map(|d|Arc::as_ptr(&d.material.program) as usize).collect();
            self.failed.retain(|key,value|programs.contains(key)||Arc::strong_count(&value.source)>1);
            let mut programs=programs;programs.insert(Arc::as_ptr(&self.fallback) as usize);
            programs.insert(Arc::as_ptr(&self.copy) as usize);
            let textures:std::collections::HashSet<_>=frame.scene.draws.iter().flat_map(|d|std::iter::once(Arc::as_ptr(&d.main_texture) as usize).chain(d.material.textures.values().map(|t|Arc::as_ptr(t) as usize))).collect();
            let meshes:std::collections::HashSet<_>=frame.scene.draws.iter().map(|d|(d.vertices.as_ptr() as usize,d.indices.as_ptr() as usize)).collect();
            self.programs.retain(|key,value|{if programs.contains(key)||Arc::strong_count(&value.source)>1{true}else{gl.delete_program(value.handle);false}});
            self.textures.retain(|key,value|{if textures.contains(key)||Arc::strong_count(&value.source)>1{true}else{gl.delete_texture(value.handle);false}});
            self.meshes.retain(|key,value|{if meshes.contains(key)||(Arc::strong_count(&value.vertices)>1&&Arc::strong_count(&value.indices)>1){true}else{gl.delete_vertex_array(value.vao);gl.delete_buffer(value.vbo);gl.delete_buffer(value.ebo);false}});
            result
        }
    }

    pub fn new(gl: Arc<glow::Context>) -> Self {
        let fallback=Arc::new(Program{pipeline:super::scene::UnityPipeline::BuiltIn,revision:1,
            vertex:Arc::from("#version 330 core\nlayout(location=0) in vec3 a_position;layout(location=1) in vec2 a_uv;layout(location=2) in vec4 a_color;uniform mat4 unity_MatrixVP;uniform mat4 unity_ObjectToWorld;out vec2 v_uv;out vec4 v_color;void main(){gl_Position=unity_MatrixVP*unity_ObjectToWorld*vec4(a_position,1.0);v_uv=a_uv;v_color=a_color;}"),
            fragment:Arc::from("#version 330 core\nin vec2 v_uv;in vec4 v_color;uniform sampler2D _MainTex;out vec4 output_color;void main(){vec4 c=texture(_MainTex,v_uv)*v_color;output_color=vec4(c.rgb*c.a,c.a);}")});
        let copy=Arc::new(Program{pipeline:super::scene::UnityPipeline::BuiltIn,revision:1,
            vertex:Arc::from("#version 330 core\nout vec2 uv;uniform vec2 preview_scale;void main(){vec2 p=vec2((gl_VertexID<<1)&2,gl_VertexID&2);uv=p;gl_Position=vec4((p*2.0-1.0)*preview_scale,0.0,1.0);}"),
            fragment:Arc::from("#version 330 core\nin vec2 uv;uniform sampler2D preview_image;out vec4 color;void main(){if(any(lessThan(uv,vec2(0.0)))||any(greaterThan(uv,vec2(1.0))))discard;color=texture(preview_image,uv);}")});
        Self { gl, programs: HashMap::new(), textures: HashMap::new(), meshes:HashMap::new(),failed:HashMap::new(),fallback,copy,snapshot:None,warnings:Vec::new() }
    }

    unsafe fn mesh(&mut self,draw:&Draw)->Result<glow::VertexArray,String> {
        let key=(draw.vertices.as_ptr() as usize,draw.indices.as_ptr() as usize);
        if let Some(mesh)=self.meshes.get(&key){return Ok(mesh.vao);}
        let gl=&self.gl;
        unsafe {
            let vao=gl.create_vertex_array()?;
            let vbo=match gl.create_buffer(){Ok(v)=>v,Err(e)=>{gl.delete_vertex_array(vao);return Err(e);}};
            let ebo=match gl.create_buffer(){Ok(v)=>v,Err(e)=>{gl.delete_buffer(vbo);gl.delete_vertex_array(vao);return Err(e);}};
            gl.bind_vertex_array(Some(vao));gl.bind_buffer(glow::ARRAY_BUFFER,Some(vbo));gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER,Some(ebo));
            let vertex_bytes:Vec<u8>=draw.vertices.iter().flat_map(|v|v.position.into_iter().chain(v.uv).chain(v.color).chain(v.normal).chain(v.tangent)).flat_map(f32::to_ne_bytes).collect();
            let index_bytes:Vec<u8>=draw.indices.iter().flat_map(|i|i.to_ne_bytes()).collect();
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER,&vertex_bytes,glow::STATIC_DRAW);gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER,&index_bytes,glow::STATIC_DRAW);
            for (slot,count,offset) in [(0,3,0),(1,2,12),(2,4,20),(3,3,36),(4,4,48)]{gl.enable_vertex_attrib_array(slot);gl.vertex_attrib_pointer_f32(slot,count,glow::FLOAT,false,64,offset);}
            self.meshes.insert(key,CachedMesh{vertices:draw.vertices.clone(),indices:draw.indices.clone(),vao,vbo,ebo});
            Ok(vao)
        }
    }

    /// Cache ownership keeps addresses stable; a revision change recompiles.
    pub unsafe fn program(&mut self, source: &Arc<Program>) -> Result<glow::Program, String> {
        let key = Arc::as_ptr(source) as usize;
        if let Some(cached) = self.programs.get(&key) {
            if cached.source.revision == source.revision { return Ok(cached.handle); }
        }
        let gl = &self.gl;
        unsafe {
            let program = gl.create_program()?;
            let mut shaders = Vec::new();
            for (stage, text) in [(glow::VERTEX_SHADER, &source.vertex), (glow::FRAGMENT_SHADER, &source.fragment)] {
                let shader = match gl.create_shader(stage) {
                    Ok(shader) => shader,
                    Err(error) => {
                        for shader in shaders { gl.delete_shader(shader); }
                        gl.delete_program(program);
                        return Err(error);
                    }
                };
                gl.shader_source(shader, text);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    let error = format!("Shader stage {stage}: {}", gl.get_shader_info_log(shader));
                    gl.delete_shader(shader);
                    for shader in shaders { gl.delete_shader(shader); }
                    gl.delete_program(program);
                    return Err(error);
                }
                gl.attach_shader(program, shader);
                shaders.push(shader);
            }
            gl.link_program(program);
            for shader in shaders { gl.detach_shader(program, shader); gl.delete_shader(shader); }
            if !gl.get_program_link_status(program) {
                let error = gl.get_program_info_log(program);
                gl.delete_program(program);
                return Err(error);
            }
            if let Some(old) = self.programs.insert(key, CachedProgram { source: source.clone(), handle: program }) {
                gl.delete_program(old.handle);
            }
            Ok(program)
        }
    }

    pub unsafe fn texture(&mut self, source: &Arc<Texture>) -> Result<glow::Texture, String> {
        let [width, height] = source.size;
        let bytes = (width as usize).checked_mul(height as usize).and_then(|n| n.checked_mul(4));
        if width == 0 || height == 0 || bytes != Some(source.rgba.len()) {
            return Err("Invalid straight RGBA texture size".into());
        }
        let key = Arc::as_ptr(source) as usize;
        if let Some(cached) = self.textures.get(&key) {
            if cached.source.revision == source.revision { return Ok(cached.handle); }
        }
        let gl = &self.gl;
        unsafe {
            if width > gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE) as u32 || height > gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE) as u32 {
                return Err("Texture exceeds GPU dimension limit".into());
            }
            let texture = gl.create_texture()?;
            let previous = gl.get_parameter_i32(glow::UNPACK_ALIGNMENT);
            let previous_texture=gl.get_parameter_texture(glow::TEXTURE_BINDING_2D);
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, if source.repeat[0]{glow::REPEAT}else{glow::CLAMP_TO_EDGE} as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, if source.repeat[1]{glow::REPEAT}else{glow::CLAMP_TO_EDGE} as i32);
            gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, width as i32, height as i32, 0,
                glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(Some(&source.rgba)));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, previous);
            gl.bind_texture(glow::TEXTURE_2D,previous_texture);
            if let Some(old) = self.textures.insert(key, CachedTexture { source: source.clone(), handle: texture }) {
                gl.delete_texture(old.handle);
            }
            Ok(texture)
        }
    }

    /// Caller binds the program first. Matrix inputs use row-major scene layout.
    pub unsafe fn uniform(&self, program: glow::Program, name: &str, value: &Uniform) {
        unsafe {
            let location = self.gl.get_uniform_location(program, name);
            match value {
                Uniform::Float(v) => self.gl.uniform_1_f32(location.as_ref(), *v),
                Uniform::Vector(v) => self.gl.uniform_4_f32_slice(location.as_ref(), v),
                Uniform::Matrix(m) => {
                    let column_major: Vec<f32> = (0..4).flat_map(|column| (0..4).map(move |row| m[row][column])).collect();
                    self.gl.uniform_matrix_4_f32_slice(location.as_ref(), false, &column_major);
                }
            }
        }
    }

    /// Explicit shutdown avoids GL calls after the context is destroyed.
    pub unsafe fn destroy(&mut self) {
        unsafe {
            self.drop_snapshot();
            for (_, entry) in self.programs.drain() { self.gl.delete_program(entry.handle); }
            for (_, entry) in self.textures.drain() { self.gl.delete_texture(entry.handle); }
            for (_,entry) in self.meshes.drain(){self.gl.delete_vertex_array(entry.vao);self.gl.delete_buffer(entry.vbo);self.gl.delete_buffer(entry.ebo);}
        }
    }
}
fn frozen_scale(source:[i32;2],viewport:[i32;2])->[f32;2] {
    let fit=(viewport[0] as f32/source[0] as f32).min(viewport[1] as f32/source[1] as f32);
    [source[0] as f32*fit/viewport[0] as f32,source[1] as f32*fit/viewport[1] as f32]
}
fn snapshot_size_supported(viewport: [i32; 4], max_size: i32) -> bool {
    viewport[2] > 0 && viewport[3] > 0 && viewport[2] <= max_size && viewport[3] <= max_size
}

#[cfg(test)]
mod snapshot_tests {
    #[test]
    fn snapshot_limits_do_not_depend_on_viewport_origin() {
        assert!(super::snapshot_size_supported([-100, 900, 4096, 2048], 4096));
        for size in [[0, 100], [100, 0], [-1, 100], [4097, 100], [100, 4097]] {
            assert!(!super::snapshot_size_supported([0, 0, size[0], size[1]], 4096));
        }
    }
    #[test]
    fn frozen_image_preserves_aspect_on_window_resize() {
        for destination in [[1200,400],[400,1200],[800,600]] {
            let scale=super::frozen_scale([800,600],destination);
            let size=[destination[0] as f32*scale[0],destination[1] as f32*scale[1]];
            assert!((size[0]/size[1]-4.0/3.0).abs()<0.00001);
            assert!(scale[0]<=1.0 && scale[1]<=1.0);
            assert!((scale[0]-1.0).abs()<0.00001 || (scale[1]-1.0).abs()<0.00001);
        }
    }
}
