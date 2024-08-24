// SPDX-License-Identifier: (LGPL-2.1 OR BSD-2-Clause)
use clap::Parser;
mod bio_lat;
mod bpf_constants;
mod cfg;
mod cpu_pct;
mod event;
mod fs_lat;
mod mem_pct;
mod rlimit;
mod rq_lat;
mod tcp_pkt_lat;
mod stream;
mod time;
mod tool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum OutputFormat {
    Columnar,
    Csv,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum DurationFormat {
    HhMmSs,
    HhMmSsMss,
    Usecs,
}

#[derive(Debug, Parser)]
#[clap(version, long_about = "Tracing and monitoring tools for Linux")]
struct Cli {
    /// Process ID to trace, or 0 for everything
    ///
    /// +--process-A-+ --(fork)-> +--process-B-+ --(thread)-> +--process-B-+
    /// |  pid  43   |         !> |  pid  42   |              |  pid  42   |
    /// |  tgid 43   |         !> |  tgid 42   |           !> |  tgid 44   |
    /// +-thread-1/1-+            +-thread-1/2-+              +-thread-2/2-+
    ///
    /// - `$ fl --pid 42` would monitor process B and all of its threads.
    /// - `$ fl --tgid 44` would monitor process B's second thread.
    ///
    /// This diagram represents the common meaning of pid and tgid to the user.
    /// (The meaning of pid and tgid is reversed in kernel-land.)
    #[arg(default_value = "0", long, short, verbatim_doc_comment)]
    pid: i32,
    /// Thread ID to trace
    ///
    /// See '--pid' for more.
    #[arg(default_value = "0", long)]
    tgid: i32,
    /// Trace latency higher than this value
    ///
    /// Affects:
    /// - '--bio-lat'
    /// - '--rq-lat'
    /// - '--fs-lat'
    /// - '--tcp-pkt-lat'
    #[arg(default_value = "10000", long, short = 'l', verbatim_doc_comment)]
    min_lat_us: u64,
    /// For monitoring tools, stats will be reported at this interval
    ///
    /// Affects:
    /// - '--cpu-pct'
    /// - '--mem-pct'
    #[arg(default_value = "1000", long, short = 'i', verbatim_doc_comment)]
    reporting_interval_ms: u64,
    /// Enable all tracing and monitoring tools.
    #[arg(long, short)]
    all: bool,
    /// Enable block and character device i/o latency tracing.
    #[arg(long)]
    bio_lat: bool,
    /// Enable run queue latency tracing
    #[arg(long)]
    rq_lat: bool,
    /// Enable file system latency tracing
    #[arg(long)]
    fs_lat: bool,
    /// Enable TCP packet latency tracing
    #[arg(long)]
    tcp_pkt_lat: bool,
    /// Enable cpu utilization % monitoring
    #[arg(long)]
    cpu_pct: bool,
    /// Enable virtual memory utilization % monitoring
    #[arg(long)]
    mem_pct: bool,
    /// Some output styles are better for humans (columnar), others for machines
    ///
    /// - columnar
    ///   cpu_pct  101410        systemd              1        0.00
    /// - csv
    ///   cpu_pct,101459,systemd,1,0.00
    /// - json
    ///   {"tool":"cpu_pct","time":"101363","task":"systemd","pid":1,"value":0.00}
    #[arg(default_value = "columnar", long, short = 'f', verbatim_doc_comment)]
    output_format: OutputFormat,
    /// Output format for the duration since this program's start
    ///
    /// This is not the duration since the target process(es) or threads began.
    #[arg(default_value = "usecs", long, verbatim_doc_comment)]
    duration_format: DurationFormat,
    /// Show a header (tool/time/task/pid/value) as the first time of output
    ///
    /// Has no effect when the output format ('-f, --output-format') is json.
    /// Formatted according to the output format.
    #[arg(long, verbatim_doc_comment)]
    header: bool,
    /// Show a header and exit ('-V, --version' has precedence)
    ///
    /// See '--header' for more.
    #[arg(long, verbatim_doc_comment)]
    just_header: bool,
}

fn duration_to_hh_mm_ss_string(duration: std::time::Duration) -> String {
    let hh = duration.as_secs() / 3600 % 99;
    let mm = (duration.as_secs() / 60) % 60;
    let ss = duration.as_secs() % 60;
    format!("{hh:02}:{mm:02}:{ss:02}")
}

fn duration_to_hh_mm_ss_mss_string(duration: std::time::Duration) -> String {
    let hh = duration.as_secs() / 3600 % 99;
    let mm: u8 = ((duration.as_secs() / 60) % 60).try_into().unwrap();
    let ss: u8 = (duration.as_secs() % 60).try_into().unwrap();
    let us = duration.subsec_millis();
    format!("{hh:02}:{mm:02}:{ss:02}.{us:03}")
}

fn duration_to_usecs_string(duration: std::time::Duration) -> String {
    let us = duration.as_micros();
    format!("{us}")
}

fn bytes_to_str(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes)
        .unwrap()
        .trim_end_matches('\0')
        .trim()
}

async fn forever() {
    tokio::sync::Semaphore::new(0).acquire().await.ok();
}

fn show_header(opts: &Cli) {
    match opts.output_format {
        OutputFormat::Columnar => {
            println!(
                "{:<8} {:<13} {:<20} {:<8} {:<14}",
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
    duration_format: DurationFormat,
    event: &crate::event::Event<Value>,
) where
    Value: std::fmt::Display,
{
    let d = match duration_format {
        DurationFormat::HhMmSs => duration_to_hh_mm_ss_string(event.time),
        DurationFormat::HhMmSsMss => duration_to_hh_mm_ss_mss_string(event.time),
        DurationFormat::Usecs => duration_to_usecs_string(event.time),
    };
    let t = bytes_to_str(&event.task);
    let p = event.pid;
    let v = &event.value;
    match output_format {
        OutputFormat::Columnar => {
            println!("{tool_name:<8} {d:<13} {t:<20} {p:<8} {v:<14}");
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
    use crate::tcp_pkt_lat::TcpPktLat;
    use crate::tool::Tool;
    use futures::StreamExt;
    let cfg = crate::cfg::Cfg {
        min_lat_us: opts.min_lat_us,
        targ_reporting_interval_ms: opts.reporting_interval_ms,
        targ_pid: opts.pid,
        targ_tgid: opts.tgid,
        targ_dev: 0,
        targ_filter_dev: false,
        targ_filter_cgroup: false,
        targ_filter_queued: false,
    };
    macro_rules! tool_task {
        ($opt:ident, $prog:ident) => {
            tokio::spawn(async move {
                if opts.all || opts.$opt {
                    let mut prog = $prog::try_new(cfg)?;
                    while let Some(event) = prog.next().await {
                        show_event(
                            stringify!($opt),
                            opts.output_format,
                            opts.duration_format,
                            &event,
                        );
                    }
                } else {
                    forever().await;
                }
                let m = "Task ended, but not because of the user";
                Err(crate::tool::Error::Runtime(m))
            })
        };
    }
    if opts.header {
        show_header(&opts);
    }
    if opts.just_header {
        return Ok(());
    }
    rlimit::must_bump_memlock_rlimit_once();
    Ok(tokio::select! {
        r = tool_task!(bio_lat, BioLat) => r,
        r = tool_task!(cpu_pct, CpuPct) => r,
        r = tool_task!(fs_lat, FsLat) => r,
        r = tool_task!(mem_pct, MemPct) => r,
        r = tool_task!(rq_lat, RqLat) => r,
        r = tool_task!(tcp_pkt_lat, TcpPktLat) => r,
    }??)
}

#[allow(clippy::unit_arg)]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let opts = Cli::parse();
    let _ = crate::time::prog_start();
    Ok(tokio::select! {
        r = flaregun(opts) => r?,
        _ = tokio::signal::ctrl_c() => (),
    })
}
