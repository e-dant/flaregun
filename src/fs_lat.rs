pub type Value = u64;
pub type Cfg = crate::skel_fs_lat::fs_lat_types::rodata;
pub struct FsLat<'cls> {
    // Need to hold this to keep the attached probes alive
    _skel: crate::skel_fs_lat::FsLatSkel<'cls>,
    ev_buf: libbpf_rs::PerfBuffer<'cls>,
    rx: std::sync::mpsc::Receiver<crate::event::Event<Value>>,
}
unsafe impl plain::Plain for crate::skel_fs_lat::fs_lat_types::event {}
crate::event::impl_from_bytes_for!(
    FsLat<'_>,
    Value,
    crate::skel_fs_lat::fs_lat_types::event::default
);
crate::stream::impl_stream_for!(FsLat<'_>, Value);
impl crate::tool::Tool for FsLat<'_> {
    type Cfg = Cfg;
    fn try_new(cfg: Option<Cfg>) -> Result<Self, Box<dyn std::error::Error>> {
        use crate::event::FromBytes;
        use libbpf_rs::skel::OpenSkel;
        use libbpf_rs::skel::Skel;
        use libbpf_rs::skel::SkelBuilder;
        let (tx, rx) = std::sync::mpsc::channel();
        let skel_builder = crate::skel_fs_lat::FsLatSkelBuilder::default();
        let mut open_skel = skel_builder.open()?;
        if let Some(rodata) = cfg {
            open_skel.rodata_mut().min_lat_us = rodata.min_lat_us;
            open_skel.rodata_mut().targ_pid = rodata.targ_pid;
            open_skel.rodata_mut().targ_tgid = rodata.targ_tgid;
        }
        let mut skel = open_skel.load()?;
        skel.attach()?;
        let ev_buf = libbpf_rs::PerfBufferBuilder::new(skel.maps().events())
            .sample_cb(move |_cpu, data| {
                tx.send(FsLat::from_bytes(data)).unwrap();
            })
            .build()?;
        Ok(Self {
            _skel: skel,
            ev_buf,
            rx,
        })
    }
}
