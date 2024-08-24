#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct Cfg {
    pub min_lat_us: u64,
    pub targ_reporting_interval_ms: u64,
    pub targ_pid: i32,
    pub targ_tgid: i32,
    pub targ_dev: u64,
    pub targ_filter_dev: bool,
    pub targ_filter_cgroup: bool,
    pub targ_filter_queued: bool,
}
