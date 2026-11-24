use super::*;
use image::{Rgba, RgbaImage};

fn square() -> RgbaImage {
    RgbaImage::from_fn(15, 15, |x, y| {
        if (4..11).contains(&x) && (4..11).contains(&y) {
            Rgba([126, 97, 85, 255])
        } else {
            Rgba([0; 4])
        }
    })
}

#[test]
fn repeated_rows_keep_the_primary_and_preserve_disabled_instances() {
    let mut fx = Effects::default();
    for kind in Kind::ALL {
        if !kind.repeatable() {
            assert!(fx.add(kind).is_none());
            continue;
        }
        for i in 1..MAX_INSTANCES {
            let page = fx.add(kind).unwrap();
            assert_eq!(page.index, i);
            assert!(fx.enabled(page));
        }
        assert!(fx.add(kind).is_none());
        let first = Page { kind, index: 0 };
        let mut selected = Page { kind, index: 2 };
        fx.remove(first, &mut selected);
        assert_eq!(fx.count(kind), MAX_INSTANCES);
        fx.remove(Page { kind, index: 1 }, &mut selected);
        assert_eq!(selected.index, 1);
        *fx.edit(selected).switch() = false;
    }
    let saved: Effects = serde_json::from_slice(&serde_json::to_vec(&fx).unwrap()).unwrap();
    assert!(saved == fx && saved.valid());
    fx.extra_strokes.resize(MAX_INSTANCES, Stroke::default());
    assert!(!fx.valid());
}

#[test]
fn two_outlines_preserve_order_on_a_transparent_fill() {
    let mut p = Properties {
        fill: 0,
        ..Default::default()
    };
    p.effects.stroke.enabled = true;
    p.effects.stroke.size = 1;
    p.effects.stroke.color = [255, 0, 0];
    p.effects.add(Kind::Stroke).unwrap();
    p.effects.extra_strokes[0].color = [0, 0, 255];
    p.effects.extra_strokes[0].size = 3;
    let out = render(&p, &square(), Light::default());
    assert_eq!(out.get_pixel(3, 7).0, [255, 0, 0, 255]);
    assert_eq!(out.get_pixel(1, 7).0, [0, 0, 255, 255]);
    assert_eq!(out.get_pixel(7, 7)[3], 0);
    p.opacity = 0;
    assert!(
        render(&p, &square(), Light::default())
            .pixels()
            .all(|p| p[3] == 0)
    );
}
fn render(properties: &Properties, pixels: &RgbaImage, light: Light) -> RgbaImage {
    let mut out = RgbaImage::new(pixels.width(), pixels.height());
    properties.composite(&mut out, pixels, 0, light);
    out
}

#[test]
fn fill_controls_content_and_opacity_controls_all_eight_effects() {
    let source = square();
    for kind in Kind::ALL {
        let mut p = Properties {
            fill: 0,
            ..Properties::default()
        };
        *p.effects.edit(Page { kind, index: 0 }).switch() = true;
        let visible = render(&p, &source, Light::default());
        assert!(visible.pixels().any(|v| v[3] > 0));
        p.opacity = 50;
        let faded = render(&p, &source, Light::default());
        for (full, half) in visible.pixels().zip(faded.pixels()) {
            assert_eq!(half[3], full[3].div_ceil(2));
        }
        p.opacity = 0;
        let mut backdrop = RgbaImage::from_pixel(15, 15, Rgba([42, 61, 83, 127]));
        let original = backdrop.clone();
        p.composite(&mut backdrop, &source, 0, Light::default());
        assert_eq!(backdrop, original);
    }
}

#[test]
fn legacy_stroke_and_shadow_survive_zero_fill_and_keep_their_geometry() {
    let source = square();
    for kind in [Kind::Stroke, Kind::Shadow, Kind::Overlay] {
        let mut p = Properties {
            fill: 0,
            ..Default::default()
        };
        p.effects.legacy_appearance = true;
        *p.effects.edit(Page { kind, index: 0 }).switch() = true;
        let image = render(&p, &source, Light::default());
        assert!(image.pixels().any(|v| v[3] > 0));
        if kind == Kind::Stroke {
            assert_eq!(image.get_pixel(3, 3)[3], 255); // square legacy dilation
            assert_eq!(image.get_pixel(7, 7)[3], 0);
        }
        p.opacity = 0;
        assert!(
            render(&p, &source, Light::default())
                .pixels()
                .all(|v| v[3] == 0)
        );
    }
}

#[test]
fn stroke_position_and_global_shadow_direction_change_coverage() {
    let source = square();
    let mut p = Properties {
        fill: 0,
        ..Default::default()
    };
    p.effects.stroke.enabled = true;
    p.effects.stroke.size = 2;
    for (position, outer, inner) in [
        (Position::Outside, 255, 0),
        (Position::Inside, 0, 255),
        (Position::Center, 255, 255),
    ] {
        p.effects.stroke.position = position;
        let out = render(&p, &source, Light::default());
        assert_eq!(out.get_pixel(3, 7)[3], outer);
        assert_eq!(out.get_pixel(4, 7)[3], inner);
        assert_eq!(out.get_pixel(7, 7)[3], 0);
    }
    p.effects.stroke.enabled = false;
    p.effects.shadow.enabled = true;
    p.effects.shadow.opacity = 100;
    p.effects.shadow.radius = 0;
    p.effects.shadow.shape.direction = Some((90, 3));
    p.effects.shadow.shape.global = true;
    let left = render(
        &p,
        &source,
        Light {
            angle: 0,
            altitude: 30,
        },
    );
    let right = render(
        &p,
        &source,
        Light {
            angle: 180,
            altitude: 30,
        },
    );
    assert_eq!(left.get_pixel(1, 7)[3], 255);
    assert_eq!(right.get_pixel(1, 7)[3], 0);
    p.effects.shadow.shape.global = false;
    assert_eq!(
        render(&p, &source, Light::default()).get_pixel(7, 13)[3],
        255
    );
}

#[test]
fn blend_uses_backdrop_and_overlays_apply_shape_alpha_only_once() {
    let source = RgbaImage::from_pixel(1, 1, Rgba([255, 10, 20, 128]));
    let mut p = Properties {
        fill: 0,
        ..Default::default()
    };
    p.effects.overlay.enabled = true;
    p.effects.overlay.color = [128; 3];
    p.effects.overlay.mode = Blend::Multiply;
    let mut backdrop = RgbaImage::from_pixel(1, 1, Rgba([128, 128, 128, 255]));
    p.composite(&mut backdrop, &source, 0, Light::default());
    assert_eq!(backdrop.get_pixel(0, 0).0, [96, 96, 96, 255]);
    let empty = render(&p, &source, Light::default());
    assert_eq!(empty.get_pixel(0, 0).0, [128; 4]);
    p.mask = Some(vec![vec![0]]);
    assert_eq!(
        render(&p, &source, Light::default()).get_pixel(0, 0).0,
        [0; 4]
    );
}

#[test]
fn gradients_lights_and_effects_roundtrip_and_parameters_are_bounded() {
    let mut p: Properties = serde_json::from_value(
        serde_json::json!({"stroke":true,"stroke_size":4,"shadow_offset":[-7,3]}),
    )
    .unwrap();
    assert!(p.effects.stroke.enabled && p.effects.stroke.size == 4);
    assert_eq!(
        p.effects
            .shadow
            .shape
            .offset(p.effects.shadow.offset, Light::default()),
        [-7, 3]
    );
    p.effects.gradient.enabled = true;
    p.effects.gradient.ramp.reverse = true;
    p.effects.inner_shadow.enabled = true;
    p.effects.inner_shadow.shape.global = true;
    p.effects.outer_glow.enabled = true;
    p.effects.bevel.enabled = true;
    let loaded: Properties = serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
    assert!(loaded == p && loaded.valid(1, 225));
    let source = square();
    let mut gradient = Properties::default();
    gradient.effects.gradient.enabled = true;
    gradient.effects.gradient.ramp.angle = 0;
    let forward = render(&gradient, &source, Light::default());
    gradient.effects.gradient.ramp.reverse = true;
    assert_eq!(
        forward.get_pixel(4, 7),
        render(&gradient, &source, Light::default()).get_pixel(10, 7)
    );
    p.effects.inner_shadow.shape.direction = Some((181, 0));
    assert!(!p.valid(1, 225));
    p.effects
        .edit(Page {
            kind: Kind::InnerShadow,
            index: 0,
        })
        .reset();
    assert!(p.effects.inner_shadow.enabled && p.effects.bevel.enabled && p.valid(1, 225));
}
