pub trait Tool: futures::Stream {
    fn try_new(cfg: Option<crate::cfg::Cfg>) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized;
}

#[allow(clippy::crate_in_macro_def)]
macro_rules! impl_tool_for_perf_event_bpf_prog {
    ($Tool:ident, $Skel:ty) => {
        impl crate::tool::Tool for $Tool<'_> {
            fn try_new(cfg: Option<crate::cfg::Cfg>) -> Result<Self, Box<dyn std::error::Error>> {
                use crate::event::FromBytes;
                use libbpf_rs::skel::OpenSkel;
                use libbpf_rs::skel::Skel;
                use libbpf_rs::skel::SkelBuilder;
                let (tx, rx) = std::sync::mpsc::channel();
                let skel_builder = <$Skel>::default();
                let mut open_skel = skel_builder.open()?;
                if let Some(cfg) = cfg {
                    open_skel.rodata_mut().min_lat_us = cfg.min_lat_us;
                    open_skel.rodata_mut().targ_pid = cfg.targ_pid;
                    open_skel.rodata_mut().targ_tgid = cfg.targ_tgid;
                }
                let mut skel = open_skel.load()?;
                skel.attach()?;
                let ev_buf = libbpf_rs::PerfBufferBuilder::new(skel.maps().events())
                    .sample_cb(move |_cpu, data| {
                        tx.send($Tool::from_bytes(data)).unwrap();
                    })
                    .build()?;
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
