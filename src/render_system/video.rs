//! Planar 8-bit YUV420 playback through the shared Glow context.
use std::sync::{Arc,Weak};
use eframe::{egui,glow::{self,HasContext}};
#[derive(Clone,Copy,PartialEq,Eq)]
pub(crate) enum Format {Yuv420,RgbaStraight,RgbaPremultiplied}
pub(crate) struct Frame {pub size:[usize;2],pub bytes:Vec<u8>,pub format:Format}
impl Frame {
    pub fn from_image(image:egui::ColorImage)->Self{
        Self{size:image.size,bytes:image.pixels.into_iter().flat_map(|pixel|pixel.to_array()).collect(),format:Format::RgbaPremultiplied}
    }
    pub fn byte_len(size:[usize;2])->usize{size[0]*size[1]+2*size[0].div_ceil(2)*size[1].div_ceil(2)}
}
pub(crate) struct Renderer {program:glow::Program,vao:glow::VertexArray,textures:[glow::Texture;3],size:[usize;2],format:Format,last:Weak<Frame>}
impl Renderer {
    pub unsafe fn new(gl:&glow::Context)->Result<Self,String>{unsafe{
        let program=gl.create_program()?;
        let mut shaders=Vec::new();
        for (kind,source) in [(glow::VERTEX_SHADER,VERTEX),(glow::FRAGMENT_SHADER,FRAGMENT)] {
            let shader=match gl.create_shader(kind){Ok(shader)=>shader,Err(e)=>{for shader in shaders{gl.delete_shader(shader);}gl.delete_program(program);return Err(e);}};
            gl.shader_source(shader,source);gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader){let error=gl.get_shader_info_log(shader);gl.delete_shader(shader);for shader in shaders{gl.delete_shader(shader);}gl.delete_program(program);return Err(error);}
            gl.attach_shader(program,shader);shaders.push(shader);
        }
        gl.link_program(program);let linked=gl.get_program_link_status(program);let error=gl.get_program_info_log(program);
        for shader in shaders{gl.detach_shader(program,shader);gl.delete_shader(shader);}
        if !linked{gl.delete_program(program);return Err(error);}
        let vao=match gl.create_vertex_array(){Ok(vao)=>vao,Err(e)=>{gl.delete_program(program);return Err(e);}};
        let mut textures=Vec::new();for _ in 0..3 {match gl.create_texture(){Ok(texture)=>textures.push(texture),Err(e)=>{for texture in textures{gl.delete_texture(texture);}gl.delete_vertex_array(vao);gl.delete_program(program);return Err(e);}}}
        Ok(Self{program,vao,textures:textures.try_into().unwrap(),size:[0,0],format:Format::Yuv420,last:Weak::new()})
    }}
    pub unsafe fn draw(&mut self,gl:&glow::Context,frame:&Arc<Frame>,viewport:[i32;4],flip:[bool;2],crop:[f32;4])->Result<(),String>{unsafe{
        if viewport[2]<=0||viewport[3]<=0{return Ok(());}
        let [w,h]=frame.size;
        let rgba=frame.format!=Format::Yuv420;
        let expected=if rgba{w*h*4}else{Frame::byte_len(frame.size)};
        if w==0||h==0||frame.bytes.len()!=expected{return Err("Invalid YUV420 frame".into());}
        let max=gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE) as usize;if w>max||h>max{return Err("YUV frame exceeds GPU texture limit".into());}
        let fresh=!self.last.upgrade().is_some_and(|last|Arc::ptr_eq(&last,frame));
        let alignment=gl.get_parameter_i32(glow::UNPACK_ALIGNMENT);
        let row_length=gl.get_parameter_i32(glow::UNPACK_ROW_LENGTH);
        let skip_pixels=gl.get_parameter_i32(glow::UNPACK_SKIP_PIXELS);
        let skip_rows=gl.get_parameter_i32(glow::UNPACK_SKIP_ROWS);
        gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER,None);gl.pixel_store_i32(glow::UNPACK_ALIGNMENT,1);gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH,0);gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS,0);gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS,0);
        let mut offset=0;
        for (index,[width,height]) in [[w,h],[w.div_ceil(2),h.div_ceil(2)],[w.div_ceil(2),h.div_ceil(2)]].into_iter().take(if rgba{1}else{3}).enumerate(){
            gl.active_texture(glow::TEXTURE0+index as u32);gl.bind_texture(glow::TEXTURE_2D,Some(self.textures[index]));
            gl.bind_sampler(index as u32,None);
            if fresh {
                let count=width*height*if rgba{4}else{1};let data=&frame.bytes[offset..offset+count];
                if self.size!=frame.size||self.format!=frame.format {
                    gl.tex_image_2d(glow::TEXTURE_2D,0,if rgba{glow::RGBA8}else{glow::R8} as i32,width as i32,height as i32,0,if rgba{glow::RGBA}else{glow::RED},glow::UNSIGNED_BYTE,glow::PixelUnpackData::Slice(Some(data)));
                    gl.tex_parameter_i32(glow::TEXTURE_2D,glow::TEXTURE_MIN_FILTER,glow::LINEAR as i32);gl.tex_parameter_i32(glow::TEXTURE_2D,glow::TEXTURE_MAG_FILTER,glow::LINEAR as i32);
                    gl.tex_parameter_i32(glow::TEXTURE_2D,glow::TEXTURE_WRAP_S,glow::CLAMP_TO_EDGE as i32);gl.tex_parameter_i32(glow::TEXTURE_2D,glow::TEXTURE_WRAP_T,glow::CLAMP_TO_EDGE as i32);
                }else{gl.tex_sub_image_2d(glow::TEXTURE_2D,0,0,0,width as i32,height as i32,if rgba{glow::RGBA}else{glow::RED},glow::UNSIGNED_BYTE,glow::PixelUnpackData::Slice(Some(data)));}
            }
            offset+=width*height*if rgba{4}else{1};
        }
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT,alignment);gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH,row_length);gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS,skip_pixels);gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS,skip_rows);
        self.size=frame.size;self.format=frame.format;self.last=Arc::downgrade(frame);
        gl.viewport(viewport[0],viewport[1],viewport[2],viewport[3]);
        gl.disable(glow::DEPTH_TEST);gl.disable(glow::CULL_FACE);gl.disable(glow::FRAMEBUFFER_SRGB);
        if rgba{gl.enable(glow::BLEND);gl.blend_equation(glow::FUNC_ADD);gl.blend_func_separate(glow::ONE,glow::ONE_MINUS_SRC_ALPHA,glow::ONE,glow::ONE_MINUS_SRC_ALPHA);}else{gl.disable(glow::BLEND);}
        gl.use_program(Some(self.program));gl.bind_vertex_array(Some(self.vao));
        for (name,index) in [("plane_y",0),("plane_u",1),("plane_v",2)]{gl.uniform_1_i32(gl.get_uniform_location(self.program,name).as_ref(),index);}
        gl.uniform_2_f32(gl.get_uniform_location(self.program,"flip").as_ref(),if flip[0]{1.0}else{0.0},if flip[1]{1.0}else{0.0});
        gl.uniform_1_i32(gl.get_uniform_location(self.program,"rgba_mode").as_ref(),match frame.format{Format::Yuv420=>0,Format::RgbaStraight=>1,Format::RgbaPremultiplied=>2});
        gl.uniform_4_f32(gl.get_uniform_location(self.program,"crop").as_ref(),crop[0],crop[1],crop[2],crop[3]);
        gl.draw_arrays(glow::TRIANGLES,0,3);gl.active_texture(glow::TEXTURE0);
        let error=gl.get_error();if error!=glow::NO_ERROR{return Err(format!("YUV rendering OpenGL error: 0x{error:X}"));}
        Ok(())
    }}
    pub unsafe fn destroy(self,gl:&glow::Context){unsafe{for texture in self.textures{gl.delete_texture(texture);}gl.delete_vertex_array(self.vao);gl.delete_program(self.program);}}
}
const VERTEX:&str=r#"#version 330 core
out vec2 uv;
uniform vec2 flip;
uniform vec4 crop;
void main(){vec2 p=vec2((gl_VertexID<<1)&2,gl_VertexID&2);gl_Position=vec4(p*2.0-1.0,0,1);uv=mix(crop.xy,crop.zw,vec2(p.x,1.0-p.y));uv=mix(uv,1.0-uv,flip);}
"#;
const FRAGMENT:&str=r#"#version 330 core
in vec2 uv;
out vec4 color;
uniform sampler2D plane_y;
uniform sampler2D plane_u;
uniform sampler2D plane_v;
uniform int rgba_mode;
vec4 rgba_pixel(ivec2 pixel){
    vec4 value=texelFetch(plane_y,clamp(pixel,ivec2(0),textureSize(plane_y,0)-1),0);
    if(rgba_mode==1){value.rgb*=value.a;}
    return value;
}
vec4 rgba_sample(vec2 coordinate){
    vec2 p=coordinate*vec2(textureSize(plane_y,0))-0.5;
    ivec2 base=ivec2(floor(p));vec2 f=fract(p);
    return mix(mix(rgba_pixel(base),rgba_pixel(base+ivec2(1,0)),f.x),mix(rgba_pixel(base+ivec2(0,1)),rgba_pixel(base+ivec2(1,1)),f.x),f.y);
}
// Match the regular egui_glow path's sub-LSB output dithering. Stable spatial
// noise breaks RGB quantization bands without blurring or temporal flicker.
float output_noise(vec2 position){return (fract(52.9829189*fract(dot(position,vec2(0.06711056,0.00583715))))-0.5)*0.95/255.0;}
void main(){
    if(rgba_mode!=0){color=rgba_sample(uv);color.rgb=clamp(color.rgb+vec3(output_noise(gl_FragCoord.xy))*color.a,vec3(0.0),vec3(color.a));return;}
    vec2 chroma_uv=uv*vec2(textureSize(plane_y,0))/(2.0*vec2(textureSize(plane_u,0)));
    float y=(texture(plane_y,uv).r-16.0/255.0)*(255.0/219.0);
    float u=(texture(plane_u,chroma_uv).r-128.0/255.0)*(255.0/224.0);
    float v=(texture(plane_v,chroma_uv).r-128.0/255.0)*(255.0/224.0);
    vec3 rgb=vec3(y+1.5748*v,y-0.187324*u-0.468124*v,y+1.8556*u);
    color=vec4(clamp(rgb+vec3(output_noise(gl_FragCoord.xy)),0.0,1.0),1.0);
}
"#;

pub(crate) fn callback(image:egui::Rect,rect:egui::Rect,frame:Arc<Frame>,flip:[bool;2],error:Arc<std::sync::Mutex<Option<String>>>)->egui::PaintCallback{
    let crop=[(rect.left()-image.left())/image.width(),(rect.top()-image.top())/image.height(),(rect.right()-image.left())/image.width(),(rect.bottom()-image.top())/image.height()];
    egui::PaintCallback{rect,callback:Arc::new(eframe::egui_glow::CallbackFn::new(move|info,_painter|{
        let viewport=info.viewport_in_pixels();
        if let Some(gpu)=super::GPU.lock().unwrap().as_mut(){
            let result=unsafe{gpu.draw_video(&frame,[viewport.left_px,viewport.from_bottom_px,viewport.width_px,viewport.height_px],flip,crop)};
            if let Err(message)=result{*error.lock().unwrap()=Some(message);}
        }
    }))}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_premultiplied_pixels_keep_alpha_without_reapplying_it(){
        let image=egui::ColorImage{size:[2,1],pixels:vec![egui::Color32::from_rgba_premultiplied(64,32,16,128),egui::Color32::TRANSPARENT]};
        let frame=Frame::from_image(image);
        assert!(frame.format==Format::RgbaPremultiplied);assert_eq!(frame.bytes,vec![64,32,16,128,0,0,0,0]);
    }
    #[test]
    fn planar_sizes_include_odd_chroma_edges(){
        assert_eq!(Frame::byte_len([3840,2160]),12441600);
        assert_eq!(Frame::byte_len([3,3]),17);
        assert_eq!(Frame::byte_len([1,1]),3);
        assert_eq!(Frame::byte_len([1920,1080])*4,Frame::byte_len([3840,2160]));
    }
}
