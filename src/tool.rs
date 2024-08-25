#[derive(Debug, Clone, Copy)]
pub enum Error {
    Libbpf,
    Misconfig(&'static str),
    Runtime(&'static str),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use Error::*;
        match self {
            Libbpf => write!(f, "Libbpf"),
            Misconfig(m) => write!(f, "Misconfig: {m}"),
            Runtime(m) => write!(f, "Runtime: {m}"),
        }
    }
}

pub trait Tool: futures::Stream {
    fn try_new(cfg: crate::cfg::Cfg) -> Result<Self, Error>
    where
        Self: Sized;
}

#[allow(clippy::crate_in_macro_def)]
macro_rules! impl_tool_for_perf_event_bpf_prog {
    ($Tool:ident, $Skel:ty) => {
        impl crate::tool::Tool for $Tool<'_> {
            fn try_new(cfg: crate::cfg::Cfg) -> Result<Self, crate::tool::Error> {
                use crate::event::FromBytes;
                use crate::tool::Error;
                use libbpf_rs::skel::OpenSkel;
                use libbpf_rs::skel::Skel;
                use libbpf_rs::skel::SkelBuilder;
                let (tx, rx) = std::sync::mpsc::channel();
                let skel_builder = <$Skel>::default();
                let mut open_skel = skel_builder.open().map_err(|_| Error::Libbpf)?;
                open_skel.rodata_mut().min_lat_us = cfg.min_lat_us;
                open_skel.rodata_mut().targ_pid = cfg.targ_pid;
                open_skel.rodata_mut().targ_tgid = cfg.targ_tgid;
                let mut skel = open_skel.load().map_err(|_| Error::Libbpf)?;
                skel.attach().map_err(|_| Error::Libbpf)?;
                let ev_buf = libbpf_rs::PerfBufferBuilder::new(skel.maps().events())
                    .sample_cb(move |_cpu, data| {
                        tx.send($Tool::from_bytes(data)).unwrap();
                    })
                    .build()
                    .map_err(|_| Error::Libbpf)?;
                Ok(Self {
                    _skel: skel,
                    ev_buf,
                    rx,
                })
            }
        }
    };
}

#[allow(clippy::crate_in_macro_def)]
macro_rules! impl_tool_for_ring_buf_bpf_prog {
    ($Tool:ident, $Skel:ty) => {
        impl crate::tool::Tool for $Tool<'_> {
            fn try_new(cfg: crate::cfg::Cfg) -> Result<Self, crate::tool::Error> {
                use crate::event::FromBytes;
                use crate::tool::Error;
                use libbpf_rs::skel::OpenSkel;
                use libbpf_rs::skel::Skel;
                use libbpf_rs::skel::SkelBuilder;
                let (tx, rx) = std::sync::mpsc::channel();
                let skel_builder = <$Skel>::default();
                let mut open_skel = skel_builder.open().map_err(|_| Error::Libbpf)?;
                open_skel.rodata_mut().min_lat_us = cfg.min_lat_us;
                open_skel.rodata_mut().targ_pid = cfg.targ_pid;
                open_skel.rodata_mut().targ_tgid = cfg.targ_tgid;
                let mut skel = open_skel.load().map_err(|_| Error::Libbpf)?;
                skel.attach().map_err(|_| Error::Libbpf)?;
                let mut rb = libbpf_rs::RingBufferBuilder::new();
                let maps = skel.maps();
                rb.add(maps.events(), move |data| {
                    tx.send($Tool::from_bytes(data)).unwrap();
                    return 0;
                })
                .map_err(|_| Error::Libbpf)?;
                let ev_buf = rb.build().map_err(|_| Error::Libbpf)?;
                Ok(Self {
                    _skel: skel,
                    ev_buf,
                    rx,
                })
            }
        }
    };
}

pub(crate) use impl_tool_for_perf_event_bpf_prog;
pub(crate) use impl_tool_for_ring_buf_bpf_prog;
