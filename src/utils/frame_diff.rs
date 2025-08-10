/// Utilities for comparing image frames.
///
/// The primary entry point is [`diff_rgba8`], which computes the mean absolute
/// difference between two RGBA8 images.

/// Compute the mean absolute per-channel difference between two RGBA8 images.
///
/// The slices must contain `width * height * 4` bytes. The result is normalized
/// to the range `0.0..=1.0`, where `0.0` indicates identical images.
pub fn diff_rgba8(a: &[u8], b: &[u8]) -> f32 {
    assert_eq!(a.len(), b.len());
    assert_eq!(a.len() % 4, 0);
    let sum: u64 = a
        .iter()
        .zip(b)
        .map(|(x, y)| (*x as i32 - *y as i32).abs() as u64)
        .sum();
    sum as f32 / (a.len() as f32 * 255.0)
}

#[cfg(test)]
mod tests {
    use super::diff_rgba8;

    #[test]
    fn identical_buffers_have_zero_difference() {
        let img = vec![10u8; 8];
        assert_eq!(diff_rgba8(&img, &img), 0.0);
    }

    #[test]
    fn max_difference_is_one() {
        let a = vec![0u8; 4];
        let b = vec![255u8; 4];
        assert!((diff_rgba8(&a, &b) - 1.0).abs() < f32::EPSILON);
    }
}
