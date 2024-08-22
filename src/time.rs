static PROG_START: std::sync::LazyLock<std::time::Instant> = std::sync::LazyLock::new(now);

pub fn now() -> std::time::Instant {
    std::time::Instant::now()
}

pub fn prog_start() -> std::time::Instant {
    *PROG_START
}

pub fn elapsed_since_prog_start() -> std::time::Duration {
    prog_start().elapsed()
}
