mod skel {
    include!(concat!(env!("OUT_DIR"), "/skel_rq_lat.rs"));
}
pub type Value = u64;
pub struct RqLat<'cls> {
    // Need to hold this to keep the attached probes alive
    _skel: skel::RqLatSkel<'cls>,
    ev_buf: libbpf_rs::PerfBuffer<'cls>,
    rx: std::sync::mpsc::Receiver<crate::event::Event<Value>>,
}
unsafe impl plain::Plain for skel::rq_lat_types::event {}
crate::event::impl_from_bytes_for!(RqLat<'_>, Value, skel::rq_lat_types::event);
crate::stream::impl_stream_for!(RqLat<'_>, Value);
crate::tool::impl_tool_for_perf_event_bpf_prog!(RqLat, skel::RqLatSkelBuilder);
