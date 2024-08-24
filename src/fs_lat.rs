pub type Value = u64;
pub struct FsLat<'cls> {
    // Need to hold this to keep the attached probes alive
    _skel: crate::skel_fs_lat::FsLatSkel<'cls>,
    ev_buf: libbpf_rs::PerfBuffer<'cls>,
    rx: std::sync::mpsc::Receiver<crate::event::Event<Value>>,
}
unsafe impl plain::Plain for crate::skel_fs_lat::fs_lat_types::event {}
crate::event::impl_from_bytes_for!(FsLat<'_>, Value, crate::skel_fs_lat::fs_lat_types::event);
crate::stream::impl_stream_for!(FsLat<'_>, Value);
crate::tool::impl_tool_for_perf_event_bpf_prog!(FsLat, crate::skel_fs_lat::FsLatSkelBuilder);
