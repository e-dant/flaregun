pub type Value = u64;
pub struct RqLat<'cls> {
    // Need to hold this to keep the attached probes alive
    _skel: crate::skel_rq_lat::RqLatSkel<'cls>,
    ev_buf: libbpf_rs::PerfBuffer<'cls>,
    rx: std::sync::mpsc::Receiver<crate::event::Event<Value>>,
}
unsafe impl plain::Plain for crate::skel_rq_lat::rq_lat_types::event {}
crate::event::impl_from_bytes_for!(RqLat<'_>, Value, crate::skel_rq_lat::rq_lat_types::event);
crate::stream::impl_stream_for!(RqLat<'_>, Value);
crate::tool::impl_tool_for_perf_event_bpf_prog!(RqLat, crate::skel_rq_lat::RqLatSkelBuilder);
