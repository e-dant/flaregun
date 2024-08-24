mod skel {
    include!(concat!(env!("OUT_DIR"), "/skel_bio_lat.rs"));
}
pub type Value = u64;
pub struct BioLat<'cls> {
    // Need to hold this to keep the attached probes alive
    _skel: skel::BioLatSkel<'cls>,
    ev_buf: libbpf_rs::PerfBuffer<'cls>,
    rx: std::sync::mpsc::Receiver<crate::event::Event<Value>>,
}
unsafe impl plain::Plain for skel::bio_lat_types::event {}
crate::event::impl_from_bytes_for!(BioLat<'_>, Value, skel::bio_lat_types::event);
crate::stream::impl_stream_for!(BioLat<'_>, Value);
crate::tool::impl_tool_for_perf_event_bpf_prog!(BioLat, skel::BioLatSkelBuilder);
