use koji::renderer::TimeStats;
use std::time::Duration;

#[test]
fn update_tracks_elapsed_and_delta() {
    let mut stats = TimeStats::new();
    std::thread::sleep(Duration::from_millis(5));
    stats.update();
    assert!(stats.total_time > 0.0);
    let first_total = stats.total_time;
    let first_delta = stats.delta_time;
    assert!(first_delta > 0.0);
    std::thread::sleep(Duration::from_millis(5));
    stats.update();
    assert!(stats.total_time > first_total);
    assert!(stats.delta_time > 0.0);
    assert!(stats.delta_time <= stats.total_time);
}
