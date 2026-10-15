//! Convex preview meshes. Keep the shader, textures and values from the 2D
//! scene; only the supporting mesh and its object transform change.
use crate::render_system::scene as rs;
use std::sync::{Arc, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shape {
    Sphere,
    Cylinder,
    Cube,
}
type Mesh = (Arc<[rs::Vertex]>, Arc<[u32]>);

fn vertex(position: [f32; 3], normal: [f32; 3], uv: [f32; 2], tangent: [f32; 3]) -> rs::Vertex {
    rs::Vertex {
        position,
        normal,
        uv,
        tangent: [tangent[0], tangent[1], tangent[2], 1.],
        color: [1.; 4],
    }
}
fn sphere() -> Mesh {
    let (slices, stacks) = (64, 32);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for y in 0..=stacks {
        let v = y as f32 / stacks as f32;
        let phi = v * std::f32::consts::PI;
        for x in 0..=slices {
            let u = x as f32 / slices as f32;
            let theta = u * std::f32::consts::TAU;
            let normal = [phi.sin() * theta.sin(), phi.cos(), phi.sin() * theta.cos()];
            vertices.push(vertex(
                normal.map(|v| v * 0.95),
                normal,
                [u, 1. - v],
                [theta.cos(), 0., -theta.sin()],
            ));
        }
    }
    for y in 0..stacks {
        for x in 0..slices {
            let a = (y * (slices + 1) + x) as u32;
            let b = a + (slices + 1) as u32;
            let c = a + 1;
            let d = b + 1;
            if y > 0 {
                indices.extend([a, b, c]);
            }
            if y + 1 < stacks {
                indices.extend([c, b, d]);
            }
        }
    }
    (Arc::from(vertices), Arc::from(indices))
}
fn cylinder() -> Mesh {
    let slices = 64;
    let radius = 0.75;
    let half_height = 0.85;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for y in 0..2 {
        for x in 0..=slices {
            let u = x as f32 / slices as f32;
            let theta = u * std::f32::consts::TAU;
            let normal = [theta.sin(), 0., theta.cos()];
            vertices.push(vertex(
                [
                    normal[0] * radius,
                    if y == 0 { half_height } else { -half_height },
                    normal[2] * radius,
                ],
                normal,
                [u, 1. - y as f32],
                [theta.cos(), 0., -theta.sin()],
            ));
        }
    }
    for x in 0..slices {
        let a = x as u32;
        let b = a + (slices + 1) as u32;
        indices.extend([a, b, a + 1, a + 1, b, b + 1]);
    }
    for top in [true, false] {
        let start = vertices.len() as u32;
        let y = if top { half_height } else { -half_height };
        let normal = [0., if top { 1. } else { -1. }, 0.];
        vertices.push(vertex([0., y, 0.], normal, [0.5, 0.5], [1., 0., 0.]));
        for x in 0..=slices {
            let theta = x as f32 / slices as f32 * std::f32::consts::TAU;
            vertices.push(vertex(
                [theta.sin() * radius, y, theta.cos() * radius],
                normal,
                [0.5 + theta.sin() * 0.5, 0.5 + theta.cos() * 0.5],
                [1., 0., 0.],
            ));
        }
        for x in 0..slices {
            let a = start + 1 + x as u32;
            if top {
                indices.extend([start, a, a + 1]);
            } else {
                indices.extend([start, a + 1, a]);
            }
        }
    }
    (Arc::from(vertices), Arc::from(indices))
}
fn cube() -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for (n, t, b) in [
        ([0., 0., -1.], [1., 0., 0.], [0., -1., 0.]),
        ([0., 0., 1.], [-1., 0., 0.], [0., -1., 0.]),
        ([1., 0., 0.], [0., 0., 1.], [0., -1., 0.]),
        ([-1., 0., 0.], [0., 0., -1.], [0., -1., 0.]),
        ([0., 1., 0.], [1., 0., 0.], [0., 0., -1.]),
        ([0., -1., 0.], [1., 0., 0.], [0., 0., 1.]),
    ] {
        let start = vertices.len() as u32;
        for (u, v) in [(0., 0.), (1., 0.), (1., 1.), (0., 1.)] {
            let position = std::array::from_fn(|i| {
                (n[i] + (u * 2. - 1.) * t[i] + (v * 2. - 1.) * b[i]) * 0.65
            });
            vertices.push(vertex(position, n, [u, v], t));
        }
        indices.extend([start, start + 1, start + 2, start, start + 2, start + 3]);
    }
    (Arc::from(vertices), Arc::from(indices))
}
fn mesh(shape: Shape) -> &'static Mesh {
    static SPHERE: OnceLock<Mesh> = OnceLock::new();
    static CYLINDER: OnceLock<Mesh> = OnceLock::new();
    static CUBE: OnceLock<Mesh> = OnceLock::new();
    match shape {
        Shape::Sphere => SPHERE.get_or_init(sphere),
        Shape::Cylinder => CYLINDER.get_or_init(cylinder),
        Shape::Cube => CUBE.get_or_init(cube),
    }
}
fn rotation([yaw, pitch]: [f32; 2]) -> [[f32; 4]; 4] {
    let (sy, cy) = yaw.sin_cos();
    let (sx, cx) = pitch.sin_cos();
    [
        [cy, 0., sy, 0.],
        [sx * sy, cx, -sx * cy, 0.],
        [-cx * sy, sx, cx * cy, 0.],
        [0., 0., 0., 1.],
    ]
}

pub(super) fn scene(base: &Arc<rs::Scene>, shape: Shape, angles: [f32; 2]) -> Arc<rs::Scene> {
    let (vertices, indices) = mesh(shape);
    let transform = rotation(angles);
    // The supported shaders include transparent passes with ZWrite Off. For
    // these convex meshes, order undeformed triangles far-to-near rather than
    // changing the shader's blend/depth state. Complex displaced/intersecting
    // surfaces still need a full depth-aware renderer.
    let depth = |triangle: &[u32; 3]| {
        triangle
            .iter()
            .map(|i| {
                let p = vertices[*i as usize].position;
                (0..3).map(|j| transform[2][j] * p[j]).sum::<f32>()
            })
            .sum::<f32>()
            / 3.
    };
    let mut triangles = indices
        .chunks_exact(3)
        .map(|t| [t[0], t[1], t[2]])
        .collect::<Vec<_>>();
    triangles.sort_by(|a, b| depth(b).total_cmp(&depth(a)));
    let mut draw = base.draws[0].clone();
    draw.vertices = vertices.clone();
    draw.indices = Arc::from(triangles.into_iter().flatten().collect::<Vec<_>>());
    draw.object_to_world = transform;
    Arc::new(rs::Scene {
        draws: Arc::from([draw]),
        revision: base.revision,
    })
}

pub(super) struct Cached {
    pub source: Arc<rs::Scene>,
    pub shape: Shape,
    pub angles: [f32; 2],
    pub scene: Arc<rs::Scene>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn switching_mesh_keeps_material_values_textures_and_original_scene() {
        let texture = Arc::new(rs::Texture {
            size: [1, 1],
            rgba: Arc::from([255u8; 4]),
            revision: 1,
            repeat: [false; 2],
        });
        let material = Arc::new(rs::Material {
            program: Arc::new(rs::Program {
                pipeline: rs::UnityPipeline::BuiltIn,
                vertex: Arc::from(""),
                fragment: Arc::from(""),
                revision: 1,
            }),
            state: rs::RenderState {
                source: rs::BlendFactor::One,
                destination: rs::BlendFactor::Zero,
                color_mask: [true; 4],
                depth_write: false,
                cull_back: false,
            },
            uniforms: std::collections::BTreeMap::from([(
                "_Amount".into(),
                rs::Uniform::Float(0.37),
            )]),
            textures: std::collections::BTreeMap::from([("_MainTex".into(), texture.clone())]),
        });
        let draw = rs::Draw {
            strict_shader: true,
            component: Arc::from("material-graph"),
            vertices: Arc::from([vertex([0.; 3], [0., 0., -1.], [0.; 2], [1., 0., 0.])]),
            indices: Arc::from([0u32; 3]),
            object_to_world: super::super::IDENTITY,
            material: material.clone(),
            main_texture: texture.clone(),
        };
        let base = Arc::new(rs::Scene {
            draws: Arc::from([draw]),
            revision: 7,
        });
        for shape in [Shape::Sphere, Shape::Cylinder, Shape::Cube] {
            let model = scene(&base, shape, [0.6, 0.25]);
            assert!(Arc::ptr_eq(&model.draws[0].material, &material));
            assert!(Arc::ptr_eq(&model.draws[0].main_texture, &texture));
            assert_eq!(model.revision, 7);
            assert!(model.draws[0].vertices.len() > 4);
        }
        assert_eq!(base.draws[0].vertices.len(), 1);
        assert_eq!(base.draws[0].object_to_world, super::super::IDENTITY);
        assert_eq!(material.uniforms["_Amount"], rs::Uniform::Float(0.37));
    }
    #[test]
    fn presets_have_finite_vertices_valid_uvs_and_outward_faces() {
        for shape in [Shape::Sphere, Shape::Cylinder, Shape::Cube] {
            let (vertices, indices) = mesh(shape);
            assert!(vertices.len() > 4 && indices.len() > 6);
            for v in vertices.iter() {
                assert!(
                    v.position
                        .iter()
                        .chain(&v.normal)
                        .chain(&v.uv)
                        .all(|v| v.is_finite())
                );
                assert!(v.uv.iter().all(|v| *v >= 0. && *v <= 1.));
            }
            for triangle in indices.chunks_exact(3) {
                assert!(triangle.iter().all(|i| (*i as usize) < vertices.len()));
                let a = vertices[triangle[0] as usize];
                let b = vertices[triangle[1] as usize];
                let c = vertices[triangle[2] as usize];
                let u: [f32; 3] = std::array::from_fn(|i| b.position[i] - a.position[i]);
                let v: [f32; 3] = std::array::from_fn(|i| c.position[i] - a.position[i]);
                let cross = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                let dot = (0..3)
                    .map(|i| cross[i] * (a.normal[i] + b.normal[i] + c.normal[i]))
                    .sum::<f32>();
                assert!(dot >= -1e-6, "inward face in {shape:?}");
            }
        }
    }
    #[test]
    fn rotations_preserve_distance_and_have_no_translation() {
        let r = rotation([0.6, 0.3]);
        let p = [0.2, -0.4, 0.7];
        let q: [f32; 3] = std::array::from_fn(|i| (0..3).map(|j| r[i][j] * p[j]).sum());
        assert!(
            (q.iter().map(|v| v * v).sum::<f32>() - p.iter().map(|v| v * v).sum::<f32>()).abs()
                < 1e-5
        );
        assert_eq!(r[3], [0., 0., 0., 1.]);
    }
}
