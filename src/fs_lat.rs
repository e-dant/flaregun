mod skel {
    include!(concat!(env!("OUT_DIR"), "/skel_fs_lat.rs"));
}
pub type Value = u64;
pub struct FsLat<'cls> {
    // Need to hold this to keep the attached probes alive
    _skel: skel::FsLatSkel<'cls>,
    ev_buf: libbpf_rs::PerfBuffer<'cls>,
    rx: std::sync::mpsc::Receiver<crate::event::Event<Value>>,
}
unsafe impl plain::Plain for skel::fs_lat_types::event {}
crate::event::impl_from_bytes_for!(FsLat<'_>, Value, skel::fs_lat_types::event);
crate::stream::impl_stream_for!(FsLat<'_>, Value);
crate::tool::impl_tool_for_perf_event_bpf_prog!(FsLat, skel::FsLatSkelBuilder);
