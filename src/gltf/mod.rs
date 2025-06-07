use crate::renderer::{SkeletalMesh, SkeletalVertex, StaticMesh, Vertex};
use crate::animation::{Bone, Skeleton};
use glam::{Mat4, Quat, Vec3};
use gltf::{self};

pub enum MeshData {
    Static(StaticMesh),
    Skeletal(SkeletalMesh),
}

pub struct SceneMesh {
    pub mesh: MeshData,
    pub transform: Mat4,
}

pub struct Scene {
    pub meshes: Vec<SceneMesh>,
}

fn mat4_from_node(node: &gltf::Node) -> Mat4 {
    let (t, r, s) = node.transform().decomposed();
    Mat4::from_scale_rotation_translation(Vec3::from(s), Quat::from_array(r), Vec3::from(t))
}

fn load_skin(skin: &gltf::Skin, buffers: &[gltf::buffer::Data]) -> Skeleton {
    let joint_nodes: Vec<_> = skin.joints().collect();
    let reader = skin.reader(|b| Some(&buffers[b.index()].0));
    let mut inverse = vec![Mat4::IDENTITY; joint_nodes.len()];
    if let Some(iter) = reader.read_inverse_bind_matrices() {
        for (i, m) in iter.enumerate() {
            if i < inverse.len() {
                inverse[i] = Mat4::from_cols_array_2d(&m);
            }
        }
    }
    let mut bones = Vec::new();
    for (i, joint) in joint_nodes.iter().enumerate() {
        let mut parent_idx = None;
        for (pi, potential) in joint_nodes.iter().enumerate() {
            if potential.children().any(|c| c.index() == joint.index()) {
                parent_idx = Some(pi);
                break;
            }
        }
        bones.push(Bone {
            name: joint.name().unwrap_or("").into(),
            parent: parent_idx,
            inverse_bind: inverse[i],
        });
    }
    Skeleton { bones }
}

pub fn load_scene(path: &str) -> Result<Scene, gltf::Error> {
    let (doc, buffers, _images) = gltf::import(path)?;
    let mut meshes = Vec::new();
    let default_scene = doc.default_scene().or_else(|| doc.scenes().next());
    if let Some(scene) = default_scene {
        for node in scene.nodes() {
            load_node(&node, Mat4::IDENTITY, &buffers, &mut meshes);
        }
    }
    Ok(Scene { meshes })
}

fn load_node(
    node: &gltf::Node,
    parent: Mat4,
    buffers: &[gltf::buffer::Data],
    meshes: &mut Vec<SceneMesh>,
) {
    let transform = parent * mat4_from_node(node);
    if let Some(mesh) = node.mesh() {
        for prim in mesh.primitives() {
            let reader = prim.reader(|b| Some(&buffers[b.index()].0));
            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .map(|i| i.collect())
                .unwrap_or_default();
            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|i| i.collect())
                .unwrap_or_else(|| vec![[0.0, 0.0, 1.0]; positions.len()]);
            let tex_coords: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|i| i.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);
            let indices = reader.read_indices().map(|i| i.into_u32().collect());
            let joints = reader
                .read_joints(0)
                .map(|i| i.into_u16().collect::<Vec<_>>());
            let weights = reader
                .read_weights(0)
                .map(|i| i.into_f32().collect::<Vec<_>>());
            let mesh = if let (Some(j), Some(w)) = (joints, weights) {
                let verts = positions
                    .into_iter()
                    .zip(normals)
                    .zip(tex_coords)
                    .zip(j)
                    .zip(w)
                    .map(|((((p, n), uv), j), w)| SkeletalVertex {
                        position: p,
                        normal: n,
                        tangent: [0.0, 0.0, 0.0, 1.0],
                        uv,
                        color: [1.0, 1.0, 1.0, 1.0],
                        joint_indices: j,
                        joint_weights: w,
                    })
                    .collect();
                let mut mesh = SkeletalMesh {
                    vertices: verts,
                    indices,
                    vertex_buffer: None,
                    index_buffer: None,
                    index_count: 0,
                    skeleton: Default::default(),
                    bone_buffer: None,
                };
                if let Some(skin) = node.skin() {
                    mesh.skeleton = load_skin(&skin, buffers);
                }
                MeshData::Skeletal(mesh)
            } else {
                let verts = positions
                    .into_iter()
                    .zip(normals)
                    .zip(tex_coords)
                    .map(|((p, n), uv)| Vertex {
                        position: p,
                        normal: n,
                        tangent: [0.0, 0.0, 0.0, 1.0],
                        uv,
                        color: [1.0, 1.0, 1.0, 1.0],
                    })
                    .collect();
                MeshData::Static(StaticMesh {
                    vertices: verts,
                    indices,
                    vertex_buffer: None,
                    index_buffer: None,
                    index_count: 0,
                })
            };
            meshes.push(SceneMesh { mesh, transform });
        }
    }
    for child in node.children() {
        load_node(&child, transform, buffers, meshes);
    }
}
