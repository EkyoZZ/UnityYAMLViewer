//! Shared CPU rendering primitives. Colors use egui's gamma-space,
//! premultiplied RGBA convention; this is not Unity Linear rendering.
use eframe::egui::Color32;

pub(crate) mod scene;
pub(crate) mod gpu;
pub(crate) mod shader;
pub(crate) mod mesh;

static GPU: std::sync::Mutex<Option<gpu::Gpu>> = std::sync::Mutex::new(None);
static STARTED: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

pub(crate) fn initialize(gl: Option<&std::sync::Arc<eframe::glow::Context>>) {
    if let Some(gl) = gl { *GPU.lock().unwrap() = Some(gpu::Gpu::new(gl.clone())); STARTED.get_or_init(std::time::Instant::now); }
}
pub(crate) fn available() -> bool { GPU.lock().unwrap().is_some() }
pub(crate) fn warnings()->Vec<String>{GPU.lock().unwrap().as_ref().map_or_else(Vec::new,|gpu|gpu.warnings.clone())}
pub(crate) fn snapshot_ready(scene:&std::sync::Arc<scene::Scene>)->bool {GPU.lock().unwrap().as_ref().is_some_and(|gpu|gpu.snapshot_ready(scene))}
pub(crate) fn shutdown() {
    if let Some(mut gpu) = GPU.lock().unwrap().take() { unsafe { gpu.destroy(); } }
}
/// Transform a scene expressed in a movable image rectangle into a fixed
/// preview viewport. Clipping must not change the scale of the projection.
pub(crate) fn fixed_viewport_projection(mut projection:[[f32;4];4],image:eframe::egui::Rect,viewport:eframe::egui::Rect)->[[f32;4];4] {
    let scale=[image.width()/viewport.width(),image.height()/viewport.height()];
    let offset=image.center()-viewport.center();
    let translation=[2.0*offset.x/viewport.width(),-2.0*offset.y/viewport.height()];
    let homogeneous=projection[3];
    for row in 0..2 {for column in 0..4 {
        projection[row][column]=projection[row][column]*scale[row]+homogeneous[column]*translation[row];
    }}
    projection
}
pub(crate) fn callback(rect: eframe::egui::Rect, scene: std::sync::Arc<scene::Scene>, projection: [[f32;4];4], error: std::sync::Arc<std::sync::Mutex<Option<String>>>,active:bool,style:u64) -> eframe::egui::PaintCallback {
    callback_timed(rect,scene,projection,error,active,style,None)
}
pub(crate) fn callback_timed(rect: eframe::egui::Rect, scene: std::sync::Arc<scene::Scene>, projection: [[f32;4];4], error: std::sync::Arc<std::sync::Mutex<Option<String>>>,active:bool,style:u64,time:Option<f32>) -> eframe::egui::PaintCallback {
    eframe::egui::PaintCallback { rect, callback: std::sync::Arc::new(eframe::egui_glow::CallbackFn::new(move |info,_painter| {
        let globals = std::collections::BTreeMap::new();
        let viewport=info.viewport_in_pixels();
        let frame=scene::Frame { scene:&scene, viewport:[viewport.width_px as u32,viewport.height_px as u32],view_projection:projection,clear_color:[0.0;4],elapsed_seconds:time.unwrap_or_else(||STARTED.get().map_or(0.0,|t|t.elapsed().as_secs_f32())),globals:&globals };
        if let Some(gpu)=GPU.lock().unwrap().as_mut() {
            let result=unsafe { gpu.draw_cached(&frame,scene.clone(),[viewport.left_px,viewport.from_bottom_px,viewport.width_px,viewport.height_px],active,style) };
            *error.lock().unwrap()=result.err();
        }
    })) }
}

/// Texture and vertex colors are both premultiplied. Tinting therefore
/// multiplies RGB as well as alpha, without premultiplying a second time.
#[inline]
pub(crate) fn source_over(source: Color32, tint: Color32, destination: Color32) -> Color32 {
    let alpha = (source.a() as u32 * tint.a() as u32 + 127) / 255;
    let channel = |s: u8, t: u8, d: u8| {
        ((s as u32 * t as u32 + 127) / 255
            + (d as u32 * (255 - alpha) + 127) / 255).min(255) as u8
    };
    Color32::from_rgba_premultiplied(
        channel(source.r(), tint.r(), destination.r()),
        channel(source.g(), tint.g(), destination.g()),
        channel(source.b(), tint.b(), destination.b()),
        (alpha + (destination.a() as u32 * (255 - alpha) + 127) / 255).min(255) as u8,
    )
}

/// Keep picking independent of rounded display alpha. Existing scene previews
/// pick coverage strictly above 8/255, before the alpha product is rounded.
#[inline]
pub(crate) fn pickable(source: Color32, tint: Color32) -> bool {
    source.a() as u32 * tint.a() as u32 > 8 * 255
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragging_past_window_edge_keeps_pixel_scale() {
        use eframe::egui::{Rect,pos2,vec2};
        let identity=[[1.0,0.0,0.0,0.0],[0.0,1.0,0.0,0.0],[0.0,0.0,1.0,0.0],[0.0,0.0,0.0,1.0]];
        let viewport=Rect::from_min_size(pos2(100.0,100.0),vec2(800.0,600.0));
        let image=viewport.translate(vec2(-500.0,200.0));
        let projection=fixed_viewport_projection(identity,image,viewport);
        // A 0.5 NDC separation remains 200 pixels horizontally, even though
        // the image's left edge is outside the native window.
        assert!((projection[0][0]*0.5*viewport.width()/2.0-200.0).abs()<0.001);
        let center=viewport.center()+vec2(projection[0][3]*400.0,-projection[1][3]*300.0);
        assert!((center-image.center()).length()<0.001);
    }

    #[test]
    fn zoomed_offscreen_image_preserves_both_axes() {
        use eframe::egui::{Rect,pos2,vec2};
        let identity=[[1.0,0.0,0.0,0.0],[0.0,1.0,0.0,0.0],[0.0,0.0,1.0,0.0],[0.0,0.0,0.0,1.0]];
        let viewport=Rect::from_min_size(pos2(100.0,100.0),vec2(800.0,600.0));
        let image=Rect::from_center_size(pos2(-200.0,250.0),vec2(1600.0,1200.0));
        let projection=fixed_viewport_projection(identity,image,viewport);
        assert_eq!(projection[0][0],2.0);
        assert_eq!(projection[1][1],2.0);
        let center=viewport.center()+vec2(projection[0][3]*400.0,-projection[1][3]*300.0);
        assert!((center-image.center()).length()<0.001);
    }

    #[test]
    fn transparent_and_opaque_endpoints() {
        assert_eq!(source_over(Color32::TRANSPARENT, Color32::WHITE, Color32::RED), Color32::RED);
        assert_eq!(source_over(Color32::BLUE, Color32::WHITE, Color32::RED), Color32::BLUE);
    }

    #[test]
    fn translucent_tint_and_destination_use_premultiplied_channels() {
        let source = Color32::from_rgba_premultiplied(128, 64, 0, 128);
        let tint = Color32::from_rgba_premultiplied(64, 128, 128, 128);
        let destination = Color32::from_rgba_premultiplied(0, 0, 128, 128);
        assert_eq!(source_over(source, tint, destination).to_array(), [32, 32, 96, 160]);
    }

    #[test]
    fn picking_uses_unrounded_coverage() {
        let alpha = |a| Color32::from_rgba_premultiplied(0, 0, 0, a);
        assert!(!pickable(alpha(8), Color32::WHITE));
        assert!(pickable(alpha(9), Color32::WHITE));
        assert!(pickable(alpha(16), alpha(128)));
        assert!(!pickable(alpha(16), alpha(127)));
    }
}

/// Small offscreen diagnostic snapshots, independent of the visible callback viewport.
pub(crate) fn elapsed_seconds() -> f32 { STARTED.get().map_or(0.,|time|time.elapsed().as_secs_f32()) }
pub(crate) type SnapshotResult = std::sync::Arc<std::sync::Mutex<Option<Result<eframe::egui::ColorImage,String>>>>;
pub(crate) fn snapshot_callback(rect:eframe::egui::Rect, scene:std::sync::Arc<scene::Scene>, time:f32, result:SnapshotResult, ctx:eframe::egui::Context)->eframe::egui::PaintCallback {
    eframe::egui::PaintCallback {rect, callback:std::sync::Arc::new(eframe::egui_glow::CallbackFn::new(move |_,painter| {
        use eframe::glow::{self,HasContext};
        let gl=painter.gl();
        let output=unsafe {
            let draw=gl.get_parameter_framebuffer(glow::DRAW_FRAMEBUFFER_BINDING);
            let read=gl.get_parameter_framebuffer(glow::READ_FRAMEBUFFER_BINDING);
            let old_active=gl.get_parameter_i32(glow::ACTIVE_TEXTURE) as u32;
            gl.active_texture(glow::TEXTURE0);
            let old_texture=gl.get_parameter_texture(glow::TEXTURE_BINDING_2D);
            let mut viewport=[0;4];gl.get_parameter_i32_slice(glow::VIEWPORT,&mut viewport);
            let scissor=gl.is_enabled(glow::SCISSOR_TEST);
            let mut clear=[0.;4];gl.get_parameter_f32_slice(glow::COLOR_CLEAR_VALUE,&mut clear);
            let capture=(|| {
                let texture=gl.create_texture()?;
                gl.bind_texture(glow::TEXTURE_2D,Some(texture));
                gl.tex_parameter_i32(glow::TEXTURE_2D,glow::TEXTURE_MIN_FILTER,glow::LINEAR as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D,glow::TEXTURE_MAG_FILTER,glow::LINEAR as i32);
                gl.tex_image_2d(glow::TEXTURE_2D,0,glow::RGBA8 as i32,192,192,0,glow::RGBA,glow::UNSIGNED_BYTE,glow::PixelUnpackData::Slice(None));
                let framebuffer=match gl.create_framebuffer(){Ok(value)=>value,Err(error)=>{gl.delete_texture(texture);return Err(error);}};
                let capture=(|| {
                    gl.bind_framebuffer(glow::FRAMEBUFFER,Some(framebuffer));
                    gl.framebuffer_texture_2d(glow::FRAMEBUFFER,glow::COLOR_ATTACHMENT0,glow::TEXTURE_2D,Some(texture),0);
                    if gl.check_framebuffer_status(glow::FRAMEBUFFER)!=glow::FRAMEBUFFER_COMPLETE {return Err("Node snapshot framebuffer incomplete".into());}
                    gl.disable(glow::SCISSOR_TEST);gl.viewport(0,0,192,192);
                    gl.color_mask(true,true,true,true);gl.clear_color(0.,0.,0.,0.);gl.clear(glow::COLOR_BUFFER_BIT);
                    let globals=std::collections::BTreeMap::new();
                    let frame=scene::Frame {scene:&scene,viewport:[192,192],view_projection:[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]],clear_color:[0.;4],elapsed_seconds:time,globals:&globals};
                    GPU.lock().unwrap().as_mut().ok_or("GPU unavailable")?.draw(&frame)?;
                    let mut rgba=vec![0u8;192*192*4];
                    gl.read_pixels(0,0,192,192,glow::RGBA,glow::UNSIGNED_BYTE,glow::PixelPackData::Slice(Some(&mut rgba)));
                    let flipped=rgba.chunks_exact(192*4).rev().flatten().copied().collect::<Vec<_>>();
                    Ok(eframe::egui::ColorImage::from_rgba_unmultiplied([192,192],&flipped))
                })();
                gl.delete_framebuffer(framebuffer);gl.delete_texture(texture);capture
            })();
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER,draw);gl.bind_framebuffer(glow::READ_FRAMEBUFFER,read);
            gl.active_texture(glow::TEXTURE0);gl.bind_texture(glow::TEXTURE_2D,old_texture);gl.active_texture(old_active);gl.viewport(viewport[0],viewport[1],viewport[2],viewport[3]);
            if scissor {gl.enable(glow::SCISSOR_TEST);} else {gl.disable(glow::SCISSOR_TEST);}
            gl.clear_color(clear[0],clear[1],clear[2],clear[3]);
            capture
        };
        *result.lock().unwrap()=Some(output);ctx.request_repaint();
    }))}
}
