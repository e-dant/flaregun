// SPDX-License-Identifier: (LGPL-2.1 OR BSD-2-Clause)
use clap::Parser;
mod bio_lat;
mod bpf_constants;
mod cpu_pct;
mod event;
mod fs_lat;
mod mem_pct;
mod rlimit;
mod rq_lat;
mod skel_bio_lat;
mod skel_fs_lat;
mod skel_rq_lat;
mod stream;
mod time;
mod tool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum OutputFormat {
    Columnar,
    Csv,
    Json,
}

#[derive(Debug, Parser)]
struct Cli {
    /// Some output styles are better for humans (columnar), others for machines (csv, json)
    #[arg(default_value = "columnar", long, short = 'f')]
    output_format: OutputFormat,
    /// Trace latency higher than this value (for latency-based tools)
    #[arg(default_value = "10000", long, short = 'l')]
    min_lat_us: u64,
    /// Report at this interval (for relevant tools)
    #[arg(default_value = "1000", long, short = 'i')]
    reporting_interval_ms: u64,
    /// "Process" (thread) ID to trace, or 0 for everything
    #[arg(default_value = "0", long, short)]
    pid: i32,
    /// Thread group ID ("process group") to trace, or 0 for everything
    #[arg(default_value = "0", long)]
    tgid: i32,
    /// Show the version and exit
    #[arg(long)]
    version: bool,
    /// Verbose debug output
    #[arg(long, short)]
    verbose: bool,
    /// Show a header (TOOL TIME TASK PID VALUE) as the first time of output
    #[arg(default_value = "false", long)]
    header: bool,
    /// Enable all tracing and monitoring tools
    #[arg(long, short)]
    all: bool,
    /// Enable block i/o latency tracing
    #[arg(long)]
    bio_lat: bool,
    /// Enable run queue latency tracing
    #[arg(long)]
    rq_lat: bool,
    /// Enable file system latency tracing
    #[arg(long)]
    fs_lat: bool,
    /// Enable CPU utilization monitoring
    #[arg(long)]
    cpu_pct: bool,
    /// Enable (virtual) memory utilization monitoring
    #[arg(long)]
    mem_pct: bool,
}

fn duration_to_string(duration: std::time::Duration) -> String {
    let hh = duration.as_secs() / 3600 % 99;
    let mm = (duration.as_secs() / 60) % 60;
    let ss = duration.as_secs() % 60;
    format!("{hh:02}:{mm:02}:{ss:02}")
}

fn bytes_to_str(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes)
        .unwrap()
        .trim_end_matches('\0')
        .trim()
}

async fn tty_user_pressed_enter() {
    use tokio::io::AsyncReadExt;
    if atty::is(atty::Stream::Stdin) {
        let _ = tokio::io::BufReader::new(tokio::io::stdin())
            .read(&mut [0u8; 0])
            .await;
    }
}

fn show_header(opts: &Cli) {
    match opts.output_format {
        OutputFormat::Columnar => {
            println!(
                "{:<8} {:<8} {:<20} {:<8} {:<14}",
                "TOOL", "TIME", "TASK", "PID", "VALUE"
            );
        }
        OutputFormat::Csv => {
            println!("TOOL,TIME,TASK,PID,VALUE");
        }
        OutputFormat::Json => (),
    }
}

fn show_event<Value>(
    tool_name: &str,
    output_format: OutputFormat,
    event: &crate::event::Event<Value>,
) where
    Value: std::fmt::Display,
{
    let d = duration_to_string(event.time);
    let t = bytes_to_str(&event.task);
    let p = event.pid;
    let v = &event.value;
    match output_format {
        OutputFormat::Columnar => {
            println!("{tool_name:<8} {d:<8} {t:<20} {p:<8} {v:<14}");
        }
        OutputFormat::Csv => {
            println!("{tool_name},{d},{t},{p},{v}",);
        }
        OutputFormat::Json => {
            println!(r#"{{"tool":"{tool_name}","time":"{d}","task":"{t}","pid":{p},"value":{v}}}"#,);
        }
    }
}

async fn flaregun(opts: Cli) -> Result<(), Box<dyn std::error::Error>> {
    use crate::bio_lat::BioLat;
    use crate::cpu_pct::CpuPct;
    use crate::fs_lat::FsLat;
    use crate::mem_pct::MemPct;
    use crate::rq_lat::RqLat;
    use crate::tool::Tool;
    use futures::StreamExt;
    macro_rules! prog_task {
        ($opt:ident, $prog:ident, $cfg:expr) => {
            tokio::spawn(async move {
                if opts.all || opts.$opt {
                    let mut prog = $prog::try_new(Some($cfg)).unwrap();
                    while let Some(event) = prog.next().await {
                        show_event(stringify!($opt), opts.output_format, &event);
                    }
                }
            })
        };
    }
    let bio_lat_cfg = bio_lat::Cfg {
        min_lat_us: opts.min_lat_us,
        targ_pid: opts.pid,
        targ_tgid: opts.tgid,
        targ_dev: 0,
        targ_filter_dev: false,
        targ_filter_cgroup: false,
        targ_filter_queued: false,
    };
    let cpu_pct_cfg = cpu_pct::Cfg {
        targ_pid: opts.pid,
        targ_tgid: opts.tgid,
        targ_reporting_interval_ms: opts.reporting_interval_ms,
    };
    let fs_lat_cfg = fs_lat::Cfg {
        min_lat_us: opts.min_lat_us,
        targ_pid: opts.pid,
        targ_tgid: opts.tgid,
    };
    let mem_pct_cfg = mem_pct::Cfg {
        targ_pid: opts.pid,
        targ_tgid: opts.tgid,
        targ_reporting_interval_ms: opts.reporting_interval_ms,
    };
    let rq_lat_cfg = rq_lat::Cfg {
        min_lat_us: opts.min_lat_us,
        targ_pid: opts.pid,
        targ_tgid: opts.tgid,
    };
    rlimit::must_bump_memlock_rlimit_once();
    if opts.header {
        show_header(&opts);
    }
    tokio::try_join!(
        prog_task!(bio_lat, BioLat, bio_lat_cfg),
        prog_task!(cpu_pct, CpuPct, cpu_pct_cfg),
        prog_task!(fs_lat, FsLat, fs_lat_cfg),
        prog_task!(mem_pct, MemPct, mem_pct_cfg),
        prog_task!(rq_lat, RqLat, rq_lat_cfg)
    )?;
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let opts = Cli::parse();
    if opts.version {
        println!(concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION")));
        return Ok(());
    }
    tokio::select! {
        _ = tty_user_pressed_enter() => Ok(()),
        r = flaregun(opts) => r,
    }
}
