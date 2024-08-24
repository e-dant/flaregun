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
    /// Format for the duration (since program start) in output
    #[arg(default_value = "usecs", long)]
    duration_format: DurationFormat,
    /// Show the version and exit
    #[arg(long)]
    version: bool,
    /// Verbose debug output
    #[arg(long, short)]
    verbose: bool,
    /// Show a header and exit, precedence after --version
    #[arg(long)]
    just_header: bool,
    /// Show a header (TOOL TIME TASK PID VALUE) as the first time of output
    #[arg(long)]
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

async fn tty_user_pressed_enter() {
    use tokio::io::AsyncReadExt;
    if atty::is(atty::Stream::Stdin) {
        let _ = tokio::io::BufReader::new(tokio::io::stdin())
            .read(&mut [0u8; 0])
            .await;
    } else {
        // Wait forever
        tokio::sync::Semaphore::new(0).acquire().await.ok();
    }
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
    use crate::tool::Tool;
    use futures::StreamExt;
    macro_rules! tool_task {
        ($opt:ident, $prog:ident, $cfg:expr) => {
            tokio::spawn(async move {
                if opts.all || opts.$opt {
                    let mut prog = $prog::try_new(Some($cfg)).unwrap();
                    while let Some(event) = prog.next().await {
                        show_event(
                            stringify!($opt),
                            opts.output_format,
                            opts.duration_format,
                            &event,
                        );
                    }
                }
            })
        };
    }
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
    if opts.header {
        show_header(&opts);
    }
    rlimit::must_bump_memlock_rlimit_once();
    tokio::try_join!(
        tool_task!(bio_lat, BioLat, cfg),
        tool_task!(cpu_pct, CpuPct, cfg),
        tool_task!(fs_lat, FsLat, cfg),
        tool_task!(mem_pct, MemPct, cfg),
        tool_task!(rq_lat, RqLat, cfg)
    )?;
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let opts = Cli::parse();
    if opts.version {
        println!(env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if opts.just_header {
        show_header(&opts);
        return Ok(());
    }
    tokio::select! {
        _ = tty_user_pressed_enter() => Ok(()),
        r = flaregun(opts) => r,
    }
}
