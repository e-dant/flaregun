type CollectorTask = tokio::task::JoinHandle<Result<(), crate::tool::Error>>;

pub struct MemPct {
    cfg: crate::cfg::Cfg,
    rx: std::sync::mpsc::Receiver<f32>,
    task: String,
    _collector_task: CollectorTask,
}

struct VMemPct {}

impl VMemPct {
    fn memory_percent(&self) -> Result<f32, psutil::Error> {
        Ok(psutil::memory::virtual_memory()?.percent())
    }
}

fn spawn_collector(tx: std::sync::mpsc::Sender<f32>, cfg: crate::cfg::Cfg) -> CollectorTask {
    tokio::task::spawn_blocking(move || {
        let ms = std::time::Duration::from_millis(cfg.targ_reporting_interval_ms);
        macro_rules! monitor {
            ($p:expr) => {
                loop {
                    match $p.memory_percent() {
                        Ok(v) => match tx.send(v) {
                            Ok(_) => std::thread::sleep(ms),
                            Err(_) => break,
                        },
                        Err(_) => {
                            log::error!("Failed to get memory percent");
                            break;
                        }
                    }
                }
            };
        }
        if cfg.targ_pid > 0 {
            let p = psutil::process::Process::new(cfg.targ_pid as u32)
                .map_err(|_| crate::tool::Error::Runtime("Failed to open stats for process"))?;
            monitor!(p);
        } else {
            monitor!(VMemPct {});
        }
        Ok(())
    })
}

impl crate::tool::Tool for MemPct {
    fn try_new(cfg: crate::cfg::Cfg) -> Result<Self, crate::tool::Error> {
        let (tx, rx) = std::sync::mpsc::channel();
        Ok(Self {
            cfg,
            rx,
            task: crate::event::pid_to_name(cfg.targ_pid),
            _collector_task: spawn_collector(tx, cfg),
        })
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
