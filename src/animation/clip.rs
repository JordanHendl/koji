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

#[derive(Clone)]
pub struct AnimationClip {
    pub length: f32,
    pub tracks: Vec<Vec<Keyframe>>, // per-bone keyframes
}

impl AnimationClip {
    pub fn sample_into(&self, time: f32, out: &mut [Mat4]) {
        assert_eq!(out.len(), self.tracks.len());
        for (track, m) in self.tracks.iter().zip(out.iter_mut()) {
            *m = sample_track(track, time);
        }
    }

    pub fn sample(&self, time: f32) -> Vec<Mat4> {
        let mut out = vec![Mat4::IDENTITY; self.tracks.len()];
        self.sample_into(time, &mut out);
        out
    }
}

fn sample_track(frames: &[Keyframe], time: f32) -> Mat4 {
    if frames.is_empty() {
        return Mat4::IDENTITY;
    }
    if time <= frames[0].time {
        return frames[0].transform.to_mat4();
    }
    if time >= frames.last().unwrap().time {
        return frames.last().unwrap().transform.to_mat4();
    }

    use std::cmp::Ordering;

    let idx = match frames
        .binary_search_by(|f| f.time.partial_cmp(&time).unwrap_or(Ordering::Less))
    {
        Ok(i) => return frames[i].transform.to_mat4(),
        Err(i) => i,
    };
    let a = &frames[idx - 1];
    let b = &frames[idx];
    let t = (time - a.time) / (b.time - a.time);
    a.transform.lerp(&b.transform, t).to_mat4()
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
        let mut mats = vec![Mat4::IDENTITY; clip.tracks.len()];
        clip.sample_into(0.5, &mut mats);
        let (_, _, t) = mats[0].to_scale_rotation_translation();
        assert!((t - Vec3::new(0.5, 0.0, 0.0)).length() < 0.0001);
    }

    #[test]
    fn sample_track_boundaries() {
        let frames = vec![
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
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE,
                },
            },
        ];

        // before first frame
        let m = sample_track(&frames, -1.0);
        let (_, _, t) = m.to_scale_rotation_translation();
        assert_eq!(t, Vec3::ZERO);

        // exactly at first frame
        let m = sample_track(&frames, 0.0);
        let (_, _, t) = m.to_scale_rotation_translation();
        assert_eq!(t, Vec3::ZERO);

        // midway
        let m = sample_track(&frames, 0.5);
        let (_, _, t) = m.to_scale_rotation_translation();
        assert!((t - Vec3::new(0.5, 0.0, 0.0)).length() < 0.0001);

        // after last frame
        let m = sample_track(&frames, 2.0);
        let (_, _, t) = m.to_scale_rotation_translation();
        assert_eq!(t, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn transform_to_mat4_and_lerp() {
        let a = Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            scale: Vec3::new(2.0, 2.0, 2.0),
        };
        let b = Transform {
            translation: Vec3::new(3.0, 4.0, 5.0),
            rotation: Quat::from_rotation_x(std::f32::consts::PI),
            scale: Vec3::new(4.0, 4.0, 4.0),
        };
        let m = a.to_mat4();
        let (s, r, t) = m.to_scale_rotation_translation();
        assert_eq!(t, a.translation);
        assert!((s - a.scale).length() < 0.0001);
        assert!((r.x - a.rotation.x).abs() < 0.0001);

        let l = a.lerp(&b, 0.5);
        assert_eq!(l.translation, a.translation.lerp(b.translation, 0.5));
        assert_eq!(l.scale, a.scale.lerp(b.scale, 0.5));
        let lr = a.rotation.slerp(b.rotation, 0.5);
        assert!((l.rotation.x - lr.x).abs() < 0.0001);
    }

    #[test]
    fn sample_into_empty_and_multi_tracks() {
        let clip_empty = AnimationClip { length: 1.0, tracks: vec![] };
        let mut out = Vec::<Mat4>::new();
        clip_empty.sample_into(0.5, &mut out);

        let track1 = vec![Keyframe { time: 0.0, transform: Transform::default() }];
        let track2 = vec![Keyframe { time: 0.0, transform: Transform { translation: Vec3::X, rotation: Quat::IDENTITY, scale: Vec3::ONE } }];
        let clip = AnimationClip { length: 1.0, tracks: vec![track1.clone(), track2.clone()] };
        let mut mats = vec![Mat4::IDENTITY; 2];
        clip.sample_into(0.0, &mut mats);
        let (_, _, t1) = mats[0].to_scale_rotation_translation();
        let (_, _, t2) = mats[1].to_scale_rotation_translation();
        assert_eq!(t1, Vec3::ZERO);
        assert_eq!(t2, Vec3::X);
    }

    #[test]
    fn animation_player_advance_looping_and_clamp() {
        let track = vec![
            Keyframe { time: 0.0, transform: Transform::default() },
            Keyframe { time: 1.0, transform: Transform { translation: Vec3::X, rotation: Quat::IDENTITY, scale: Vec3::ONE } },
        ];
        let clip = AnimationClip { length: 1.0, tracks: vec![track] };
        let mut player = AnimationPlayer::new(clip);

        // looping
        let _ = player.advance(0.6); // time = 0.6
        let _ = player.advance(0.6); // time wraps to 0.2
        assert!(player.time > 0.19 && player.time < 0.21);

        // disable looping and advance beyond length
        player.looping = false;
        let _ = player.advance(2.0);
        assert!((player.time - 1.0).abs() < 0.0001);
        let _ = player.advance(1.0);
        assert!((player.time - 1.0).abs() < 0.0001);
    }
}
