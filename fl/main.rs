// SPDX-License-Identifier: (LGPL-2.1 OR BSD-2-Clause)
use clap::Parser;
mod outf;
extern crate flaregun;

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
#[clap(
    version,
    long_about = r#"
Tracing and monitoring tools for Linux.

Allows tracing of:
- Block and character device i/o latency
- Run queue scheduling latency
- File system latency
- TCP packet latency

And monitoring of:
- CPU utilization %
- Virtual memory utilization %

These metrics can be exported in a columnar, CSV, or JSON format.

When written as a CSV file, the output may be plotted using `fl-plot`:
```sh
fl --all --output-file /tmp/trace.csv --pid 42
# ...
fl-plot -i /tmp/trace.csv -o /tmp/trace.html
```

The plot is a standalone HTML file which can be opened in a browser.
"#
)]
struct Cli {
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
    #[arg(long, short, default_value = "0", verbatim_doc_comment)]
    pid: i32,
    /// Thread ID to trace
    ///
    /// See '--pid' for more.
    #[arg(long, default_value = "0")]
    tgid: i32,
    /// Trace latency higher than this value
    ///
    /// Affects:
    /// - '--bio-lat'
    /// - '--rq-lat'
    /// - '--fs-lat'
    /// - '--tcp-pkt-lat'
    #[arg(long, short = 'l', default_value = "10000", verbatim_doc_comment)]
    min_lat_us: u64,
    /// Trace block i/o latency higher than this value
    #[arg(long, conflicts_with = "min_lat_us")]
    min_bio_lat_us: Option<u64>,
    /// Trace run queue latency higher than this value
    #[arg(long, conflicts_with = "min_lat_us")]
    min_rq_lat_us: Option<u64>,
    /// Trace file system latency higher than this value
    #[arg(long, conflicts_with = "min_lat_us")]
    min_fs_lat_us: Option<u64>,
    /// Trace TCP packet latency higher than this value
    #[arg(long, conflicts_with = "min_lat_us")]
    min_tcp_pkt_lat_us: Option<u64>,
    /// For monitoring tools, stats will be reported at this interval
    ///
    /// Affects:
    /// - '--cpu-pct'
    /// - '--mem-pct'
    #[arg(long, short = 'i', default_value = "1000", verbatim_doc_comment)]
    reporting_interval_ms: u64,
    /// Some output styles are better for humans (columnar), others for machines
    ///
    /// - columnar
    ///   cpu_pct  101410        systemd              1        0.00
    /// - csv
    ///   cpu_pct,101459,systemd,1,0.00
    /// - json
    ///   {"tool":"cpu_pct","time":"101363","task":"systemd","pid":1,"value":0.00}
    #[arg(long, short = 'f', default_value = "columnar", verbatim_doc_comment)]
    output_format: OutputFormat,
    /// Output format for the duration since this program's start
    ///
    /// This is not the duration since the target process(es) or threads began.
    #[arg(long, default_value = "usecs", verbatim_doc_comment)]
    duration_format: DurationFormat,
    /// Write events to this file, if present, or to standard output if not given
    #[arg(long, short = 'o')]
    output_file: Option<std::path::PathBuf>,
    /// Omit the header (tool/time/task/pid/value) as the first line of output
    ///
    /// Has no effect when the output format ('-f, --output-format') is json.
    /// Formatted according to the output format.
    #[arg(long, verbatim_doc_comment)]
    no_header: bool,
    /// Show a header and exit ('-V, --version' has precedence)
    ///
    /// See '--header' for more.
    #[arg(long, verbatim_doc_comment, conflicts_with = "no_header")]
    just_header: bool,
}

fn duration_to_hh_mm_ss_string(duration: std::time::Duration) -> String {
    let hh = duration.as_secs() / 3600 % 99;
    let mm = (duration.as_secs() / 60) % 60;
    let ss = duration.as_secs() % 60;
    format!("{hh:02}:{mm:02}:{ss:02}")
}

fn duration_to_hh_mm_ss_mss_string(duration: std::time::Duration) -> String {
    let hh_mm_ss = duration_to_hh_mm_ss_string(duration);
    let ms = duration.subsec_millis();
    format!("{hh_mm_ss}.{ms:03}")
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
    use OutputFormat::*;
    match opts.output_format {
        Columnar => outf::outfprintln!(
            "{:<12} {:<13} {:<20} {:<8} {:<14}",
            "tool",
            "time",
            "task",
            "pid",
            "value"
        ),
        Csv => outf::outfprintln!("tool,time,task,pid,value"),
        Json => (),
    }
}

fn show_event<Value>(
    tool: &str,
    output_format: OutputFormat,
    duration_format: DurationFormat,
    event: &flaregun::Event<Value>,
) where
    Value: std::fmt::Display,
{
    use DurationFormat::*;
    use OutputFormat::*;
    let d = match duration_format {
        HhMmSs => duration_to_hh_mm_ss_string(event.time),
        HhMmSsMss => duration_to_hh_mm_ss_mss_string(event.time),
        Usecs => duration_to_usecs_string(event.time),
    };
    let t = bytes_to_str(&event.task);
    let p = event.pid;
    let v = &event.value;
    match output_format {
        Columnar => outf::outfbufprintln!("{tool:<12} {d:<13} {t:<20} {p:<8} {v:<14}"),
        Csv => outf::outfbufprintln!("{tool},{d},{t},{p},{v}"),
        Json => outf::outfbufprintln!(
            r#"{{"tool":"{tool}","time":"{d}","task":"{t}","pid":{p},"value":{v}}}"#
        ),
    }
}

async fn flaregun(opts: Cli) -> Result<(), Box<dyn std::error::Error>> {
    use flaregun::tool::Tool;
    use flaregun::BioLat;
    use flaregun::CpuPct;
    use flaregun::FsLat;
    use flaregun::MemPct;
    use flaregun::RqLat;
    use flaregun::TcpPktLat;
    use futures::StreamExt;
    macro_rules! tool_task {
        ($opt:ident, $opt_mlu:expr, $prog:ident) => {
            tokio::spawn(async move {
                let cfg = flaregun::Cfg {
                    min_lat_us: $opt_mlu.unwrap_or(opts.min_lat_us),
                    targ_reporting_interval_ms: opts.reporting_interval_ms,
                    targ_pid: opts.pid,
                    targ_tgid: opts.tgid,
                    targ_dev: 0,
                    targ_filter_dev: false,
                    targ_filter_cgroup: false,
                    targ_filter_queued: false,
                };
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
                Err(flaregun::tool::Error::Runtime(m))
            })
        };
    }
    if !opts.no_header {
        show_header(&opts);
    }
    if opts.just_header {
        return Ok(());
    }
    flaregun::must_bump_memlock_rlimit_once();
    Ok(tokio::select! {
        r = tool_task!(bio_lat, opts.min_bio_lat_us, BioLat) => r,
        r = tool_task!(fs_lat, opts.min_fs_lat_us, FsLat) => r,
        r = tool_task!(rq_lat, opts.min_rq_lat_us, RqLat) => r,
        r = tool_task!(tcp_pkt_lat, opts.min_tcp_pkt_lat_us, TcpPktLat) => r,
        r = tool_task!(cpu_pct, None, CpuPct) => r,
        r = tool_task!(mem_pct, None, MemPct) => r,
    }??)
}

#[allow(clippy::unit_arg)]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = flaregun::time::prog_start();
    let opts = Cli::parse();
    outf::init(&opts.output_file);
    let r = tokio::select! {
        r = flaregun(opts) => r,
        _ = tokio::signal::ctrl_c() => Ok(()),
    };
    outf::buf_flush();
    r
}
