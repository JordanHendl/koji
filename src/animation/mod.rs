use glam::Mat4;

#[derive(Clone, Debug, Default)]
pub struct Bone {
    pub name: String,
    pub parent: Option<usize>,
    pub inverse_bind: Mat4,
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
}

impl Animator {
    pub fn new(skeleton: Skeleton) -> Self {
        let matrices = vec![Mat4::IDENTITY; skeleton.bone_count()];
        Self { skeleton, matrices }
    }

    pub fn update(&mut self, local: &[Mat4]) {
        assert_eq!(local.len(), self.skeleton.bone_count());
        let mut world_cache: Vec<Option<Mat4>> = vec![None; self.skeleton.bone_count()];
        for i in 0..self.skeleton.bone_count() {
            let world = compute_world_recursive(&self.skeleton, i, local, &mut world_cache);
            self.matrices[i] = world * self.skeleton.bones[i].inverse_bind;
        }
    }

    pub fn matrices(&self) -> &[Mat4] {
        &self.matrices
    }
}

fn compute_world_recursive(
    skeleton: &Skeleton,
    index: usize,
    local: &[Mat4],
    cache: &mut [Option<Mat4>],
) -> Mat4 {
    if let Some(m) = cache[index] {
        return m;
    }
    let bone = &skeleton.bones[index];
    let parent_world = if let Some(p) = bone.parent {
        compute_world_recursive(skeleton, p, local, cache)
    } else {
        Mat4::IDENTITY
    };
    let world = parent_world * local[index];
    cache[index] = Some(world);
    world
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
            },
            Bone {
                name: "child".into(),
                parent: Some(0),
                inverse_bind: Mat4::IDENTITY,
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
            },
            Bone {
                name: "root".into(),
                parent: None,
                inverse_bind: Mat4::IDENTITY,
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
}
