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
        for (i, bone) in self.skeleton.bones.iter().enumerate() {
            let parent_world = bone
                .parent
                .and_then(|p| self.matrices.get(p).cloned())
                .unwrap_or(Mat4::IDENTITY);
            let world = parent_world * local[i];
            self.matrices[i] = world * bone.inverse_bind;
        }
    }

    pub fn matrices(&self) -> &[Mat4] {
        &self.matrices
    }
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
}
