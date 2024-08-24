static PROG_START: std::sync::LazyLock<std::time::Instant> =
    std::sync::LazyLock::new(std::time::Instant::now);

pub fn prog_start() -> std::time::Instant {
    *PROG_START
}
