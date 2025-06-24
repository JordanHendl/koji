use glam::Mat4;

pub mod clip;

#[derive(Clone, Debug, Default)]
pub struct Bone {
    pub name: String,
    pub parent: Option<usize>,
    pub inverse_bind: Mat4,
    pub node_index: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
}

impl Skeleton {
    pub fn bone_count(&self) -> usize {
        self.bones.len()
    }
}

pub struct Animator {
    pub skeleton: Skeleton,
    pub matrices: Vec<Mat4>,
    order: Vec<usize>,
}

impl Animator {
    pub fn new(skeleton: Skeleton) -> Self {
        let matrices = vec![Mat4::IDENTITY; skeleton.bone_count()];
        let order = compute_topo_order(&skeleton);
        Self { skeleton, matrices, order }
    }

    pub fn update(&mut self, local: &[Mat4]) {
        assert_eq!(local.len(), self.skeleton.bone_count());
        let mut worlds = vec![Mat4::IDENTITY; self.skeleton.bone_count()];
        for &i in &self.order {
            let bone = &self.skeleton.bones[i];
            let parent_world = if let Some(p) = bone.parent {
                worlds[p]
            } else {
                Mat4::IDENTITY
            };
            worlds[i] = parent_world * local[i];
            self.matrices[i] = worlds[i] * bone.inverse_bind;
        }
    }

    /// Update bone matrices using the node-local transforms for the joint nodes.
    pub fn update_from_nodes(&mut self, nodes: &[Mat4]) {
        let mut local = Vec::with_capacity(self.skeleton.bone_count());
        for bone in &self.skeleton.bones {
            let mat = nodes
                .get(bone.node_index)
                .copied()
                .unwrap_or(Mat4::IDENTITY);
            local.push(mat);
        }
        self.update(&local);
    }

    pub fn matrices(&self) -> &[Mat4] {
        &self.matrices
    }
}

fn compute_topo_order(skeleton: &Skeleton) -> Vec<usize> {
    fn visit(idx: usize, skeleton: &Skeleton, visited: &mut [bool], order: &mut Vec<usize>) {
        if visited[idx] {
            return;
        }
        visited[idx] = true;
        order.push(idx);
        for (i, bone) in skeleton.bones.iter().enumerate() {
            if bone.parent == Some(idx) {
                visit(i, skeleton, visited, order);
            }
        }
    }

    let mut order = Vec::new();
    let mut visited = vec![false; skeleton.bone_count()];
    for (i, bone) in skeleton.bones.iter().enumerate() {
        if bone.parent.is_none() {
            visit(i, skeleton, &mut visited, &mut order);
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Mat4, Vec3};

    #[test]
    fn simple_hierarchy_updates() {
        let bones = vec![
            Bone {
                name: "root".into(),
                parent: None,
                inverse_bind: Mat4::IDENTITY,
                node_index: 0,
            },
            Bone {
                name: "child".into(),
                parent: Some(0),
                inverse_bind: Mat4::IDENTITY,
                node_index: 1,
            },
        ];
        let skeleton = Skeleton { bones };
        let mut animator = Animator::new(skeleton);
        let local = vec![
            Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0)),
            Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0)),
        ];
        animator.update(&local);
        let mats = animator.matrices();
        let root_pos = mats[0].transform_point3(Vec3::ZERO);
        let child_pos = mats[1].transform_point3(Vec3::ZERO);
        assert_eq!(root_pos, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(child_pos, Vec3::new(1.0, 0.0, 1.0));
    }

    #[test]
    fn out_of_order_bones() {
        let bones = vec![
            Bone {
                name: "child".into(),
                parent: Some(1),
                inverse_bind: Mat4::IDENTITY,
                node_index: 1,
            },
            Bone {
                name: "root".into(),
                parent: None,
                inverse_bind: Mat4::IDENTITY,
                node_index: 0,
            },
        ];
        let skeleton = Skeleton { bones };
        let mut animator = Animator::new(skeleton);
        let local = vec![
            Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0)),
        ];
        animator.update(&local);
        let mats = animator.matrices();
        let root_pos = mats[1].transform_point3(Vec3::ZERO);
        let child_pos = mats[0].transform_point3(Vec3::ZERO);
        assert_eq!(root_pos, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(child_pos, Vec3::new(1.0, 0.0, 1.0));
    }

    #[test]
    fn empty_skeleton_bone_count() {
        let skeleton = Skeleton { bones: Vec::new() };
        assert_eq!(skeleton.bone_count(), 0);
    }

    #[test]
    #[should_panic]
    fn animator_update_wrong_slice_length() {
        let bones = vec![Bone { name: "root".into(), parent: None, inverse_bind: Mat4::IDENTITY, node_index: 0 }];
        let skeleton = Skeleton { bones };
        let mut animator = Animator::new(skeleton);
        let locals = vec![Mat4::IDENTITY, Mat4::IDENTITY];
        animator.update(&locals);
    }

    #[test]
    fn compute_order_deep_hierarchy() {
        // Two roots each with a long chain
        let bones = vec![
            Bone { name: "r1".into(), parent: None, inverse_bind: Mat4::IDENTITY, node_index: 0 }, //0
            Bone { name: "r2".into(), parent: None, inverse_bind: Mat4::IDENTITY, node_index: 1 }, //1
            Bone { name: "r1c1".into(), parent: Some(0), inverse_bind: Mat4::IDENTITY, node_index: 2 }, //2
            Bone { name: "r1c2".into(), parent: Some(2), inverse_bind: Mat4::IDENTITY, node_index: 3 }, //3
            Bone { name: "r2c1".into(), parent: Some(1), inverse_bind: Mat4::IDENTITY, node_index: 4 }, //4
            Bone { name: "r2c2".into(), parent: Some(4), inverse_bind: Mat4::IDENTITY, node_index: 5 }, //5
        ];
        let skeleton = Skeleton { bones };
        let order = super::compute_topo_order(&skeleton);
        // parents should always appear before children
        for (i, b) in skeleton.bones.iter().enumerate() {
            if let Some(p) = b.parent {
                let pos_parent = order.iter().position(|&x| x == p).unwrap();
                let pos_child = order.iter().position(|&x| x == i).unwrap();
                assert!(pos_parent < pos_child);
            }
        }
    }

    #[test]
    fn update_from_nodes_matches_update() {
        use crate::gltf::{load_scene, MeshData};
        use crate::animation::clip::AnimationPlayer;

        let scene = load_scene("tests/data/simple_skin.gltf").expect("load");
        let mesh = match &scene.meshes[0].mesh {
            MeshData::Skeletal(m) => m.clone(),
            _ => panic!("expected skeletal mesh"),
        };
        let clip = scene.animations[0].clone();
        let mut player = AnimationPlayer::new(clip);
        let nodes = player.advance(0.0);

        let mut anim_a = Animator::new(mesh.skeleton.clone());
        let mut anim_b = Animator::new(mesh.skeleton.clone());

        let locals: Vec<Mat4> = mesh
            .skeleton
            .bones
            .iter()
            .map(|b| nodes[b.node_index])
            .collect();
        anim_a.update(&locals);
        anim_b.update_from_nodes(&nodes);

        assert_eq!(anim_a.matrices, anim_b.matrices);
    }
}
