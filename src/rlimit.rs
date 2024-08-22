pub fn must_bump_memlock_rlimit_once() {
    static WAS_SET: std::sync::Once = std::sync::Once::new();
    WAS_SET.call_once(|| {
        let rlimit = libc::rlimit {
            rlim_cur: 128 << 20,
            rlim_max: 128 << 20,
        };
        if unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlimit) } != 0 {
            panic!("Failed to increase rlimit");
        }
    });
}
