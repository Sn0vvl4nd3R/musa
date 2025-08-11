pub fn seconds_to_mmss(secs: f32) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "--:--".to_string();
    }
    let s = secs as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}
