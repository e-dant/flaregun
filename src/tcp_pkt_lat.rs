mod skel {
    include!(concat!(env!("OUT_DIR"), "/skel_tcp_pkt_lat.rs"));
}
pub type Value = u64;
pub struct TcpPktLat<'cls> {
    // Need to hold this to keep the attached probes alive
    _skel: skel::TcpPktLatSkel<'cls>,
    ev_buf: libbpf_rs::RingBuffer<'cls>,
    rx: std::sync::mpsc::Receiver<crate::event::Event<Value>>,
}
unsafe impl plain::Plain for skel::tcp_pkt_lat_types::event {}
crate::event::impl_from_bytes_for!(TcpPktLat<'_>, Value, skel::tcp_pkt_lat_types::event);
crate::stream::impl_stream_for!(TcpPktLat<'_>, Value);
crate::tool::impl_tool_for_ring_buf_bpf_prog!(TcpPktLat, skel::TcpPktLatSkelBuilder);
