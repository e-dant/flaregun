static PROG_START: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

pub fn prog_start() -> std::time::Instant {
    let mut start = PROG_START.lock().unwrap();
    if start.is_none() {
        *start = Some(std::time::Instant::now());
    }
    start.unwrap()
}
