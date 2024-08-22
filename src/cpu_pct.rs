#[derive(Clone, Copy)]
pub struct Cfg {
    pub targ_pid: i32,
    pub targ_tgid: i32,
    pub targ_reporting_interval_ms: u64,
}

pub struct CpuPct {
    cfg: Cfg,
    rx: std::sync::mpsc::Receiver<f32>,
    task: String,
    _collector_task: tokio::task::JoinHandle<()>,
}

fn spawn_collector(tx: std::sync::mpsc::Sender<f32>, cfg: Cfg) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        let ms = std::time::Duration::from_millis(cfg.targ_reporting_interval_ms);
        let mut p = psutil::process::Process::new(cfg.targ_pid as u32).unwrap();
        loop {
            let v = p.cpu_percent().unwrap();
            match tx.send(v) {
                Ok(_) => std::thread::sleep(ms),
                Err(_) => break,
            }
        }
    })
}

impl crate::tool::Tool for CpuPct {
    type Cfg = Cfg;
    fn try_new(cfg: Option<Cfg>) -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, rx) = std::sync::mpsc::channel();
        let cfg = match cfg {
            Some(cfg) => cfg,
            None => return Err("Configuration must be provided for CpuPct".into()),
        };
        if cfg.targ_pid == 0 && cfg.targ_tgid == 0 {
            return Err("Either pid or tgid must be specified".into());
        }
        Ok(Self {
            cfg,
            rx,
            task: crate::event::pid_to_name(cfg.targ_pid),
            _collector_task: spawn_collector(tx, cfg),
        })
    }
}

impl futures::Stream for CpuPct {
    type Item = crate::event::Event<String>;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        ctx: &mut std::task::Context,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.rx.try_recv().ok() {
            Some(ev) => {
                let mut task: [u8; crate::bpf_constants::TASK_COMM_LEN as usize] =
                    [' ' as u8; crate::bpf_constants::TASK_COMM_LEN as usize];
                for i in 0..std::cmp::min(16, self.task.len()) {
                    task[i] = self.task.as_bytes()[i];
                }
                let ev = crate::event::Event {
                    time: crate::time::elapsed_since_prog_start(),
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
