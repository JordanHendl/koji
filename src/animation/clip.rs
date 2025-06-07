// Animation clip module with keyframes and interpolation
use glam::{Vec3, Quat, Mat4};

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn to_mat4(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            translation: self.translation.lerp(other.translation, t),
            rotation: self.rotation.slerp(other.rotation, t),
            scale: self.scale.lerp(other.scale, t),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Keyframe {
    pub time: f32,
    pub transform: Transform,
}

pub struct AnimationClip {
    pub length: f32,
    pub tracks: Vec<Vec<Keyframe>>, // per-bone keyframes
}

impl AnimationClip {
    pub fn sample(&self, time: f32) -> Vec<Mat4> {
        self.tracks
            .iter()
            .map(|track| sample_track(track, time))
            .collect()
    }
}

fn sample_track(frames: &[Keyframe], time: f32) -> Mat4 {
    if frames.is_empty() {
        return Mat4::IDENTITY;
    }
    if time <= frames[0].time {
        return frames[0].transform.to_mat4();
    }
    for pair in frames.windows(2) {
        let a = pair[0];
        let b = pair[1];
        if time >= a.time && time <= b.time {
            let t = (time - a.time) / (b.time - a.time);
            return a.transform.lerp(&b.transform, t).to_mat4();
        }
    }
    frames.last().unwrap().transform.to_mat4()
}

pub struct AnimationPlayer {
    pub clip: AnimationClip,
    pub time: f32,
    pub speed: f32,
    pub looping: bool,
}

impl AnimationPlayer {
    pub fn new(clip: AnimationClip) -> Self {
        Self {
            clip,
            time: 0.0,
            speed: 1.0,
            looping: true,
        }
    }

    pub fn advance(&mut self, dt: f32) -> Vec<Mat4> {
        self.time += dt * self.speed;
        if self.looping {
            self.time = self.time % self.clip.length;
        } else if self.time > self.clip.length {
            self.time = self.clip.length;
        }
        self.clip.sample(self.time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Vec3, Quat};

    #[test]
    fn interpolate_midpoint() {
        let track = vec![
            Keyframe {
                time: 0.0,
                transform: Transform {
                    translation: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE,
                },
            },
            Keyframe {
                time: 1.0,
                transform: Transform {
                    translation: Vec3::new(1.0, 0.0, 0.0),
                    rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                    scale: Vec3::ONE,
                },
            },
        ];
        let clip = AnimationClip { length: 1.0, tracks: vec![track] };
        let mut player = AnimationPlayer::new(clip);
        let mats = player.advance(0.5);
        let (_, _, t) = mats[0].to_scale_rotation_translation();
        assert!((t - Vec3::new(0.5, 0.0, 0.0)).length() < 0.0001);
    }
}
