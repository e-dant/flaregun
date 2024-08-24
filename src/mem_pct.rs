pub struct MemPct {
    cfg: crate::cfg::Cfg,
    rx: std::sync::mpsc::Receiver<f32>,
    task: String,
    _collector_task: tokio::task::JoinHandle<()>,
}

fn spawn_collector(
    tx: std::sync::mpsc::Sender<f32>,
    cfg: crate::cfg::Cfg,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        let ms = std::time::Duration::from_millis(cfg.targ_reporting_interval_ms);
        let p = psutil::process::Process::new(cfg.targ_pid as u32).unwrap();
        loop {
            let v = p.memory_percent().unwrap();
            match tx.send(v) {
                Ok(_) => std::thread::sleep(ms),
                Err(_) => break,
            }
        }
    })
}

impl crate::tool::Tool for MemPct {
    fn try_new(cfg: crate::cfg::Cfg) -> Result<Self, crate::tool::Error> {
        let (tx, rx) = std::sync::mpsc::channel();
        if cfg.targ_pid == 0 && cfg.targ_tgid == 0 {
            let m = "Either pid or tgid must be specified for tool MemPct";
            Err(crate::tool::Error::Misconfig(m))
        } else {
            Ok(Self {
                cfg,
                rx,
                task: crate::event::pid_to_name(cfg.targ_pid),
                _collector_task: spawn_collector(tx, cfg),
            })
        }
    }
}

impl futures::Stream for MemPct {
    type Item = crate::event::Event<String>;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        ctx: &mut std::task::Context,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.rx.try_recv().ok() {
            Some(ev) => {
                let mut task: [u8; crate::bpf_constants::TASK_COMM_LEN as usize] =
                    [b' '; crate::bpf_constants::TASK_COMM_LEN as usize];
                task[..std::cmp::min(16, self.task.len())]
                    .copy_from_slice(&self.task.as_bytes()[..std::cmp::min(16, self.task.len())]);
                let ev = crate::event::Event {
                    time: crate::time::prog_start().elapsed(),
                    task,
                    pid: self.cfg.targ_pid as u32,
                    value: format!("{:00.02}", ev),
                };
                std::task::Poll::Ready(Some(ev))
            }
            None => {
                let waker = ctx.waker().clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    waker.wake();
                });
                std::task::Poll::Pending
            }
        }
    }
}
