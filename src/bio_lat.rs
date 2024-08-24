pub type Value = u64;
pub struct BioLat<'cls> {
    // Need to hold this to keep the attached probes alive
    _skel: crate::skel_bio_lat::BioLatSkel<'cls>,
    ev_buf: libbpf_rs::PerfBuffer<'cls>,
    rx: std::sync::mpsc::Receiver<crate::event::Event<Value>>,
}
unsafe impl plain::Plain for crate::skel_bio_lat::bio_lat_types::event {}
crate::event::impl_from_bytes_for!(BioLat<'_>, Value, crate::skel_bio_lat::bio_lat_types::event);
crate::stream::impl_stream_for!(BioLat<'_>, Value);
crate::tool::impl_tool_for_perf_event_bpf_prog!(BioLat, crate::skel_bio_lat::BioLatSkelBuilder);
