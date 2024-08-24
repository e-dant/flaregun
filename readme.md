# Flaregun

```
Tracing and monitoring tools for Linux

Usage: fl [OPTIONS]

Options:
  -p, --pid <PID>
          Process ID to trace, or 0 for everything
          
          +--process-A-+ --(fork)-> +--process-B-+ --(thread)-> +--process-B-+
          |  pid  43   |         !> |  pid  42   |              |  pid  42   |
          |  tgid 43   |         !> |  tgid 42   |           !> |  tgid 44   |
          +-thread-1/1-+            +-thread-1/2-+              +-thread-2/2-+
          
          - `$ fl --pid 42` would monitor process B and all of its threads.
          - `$ fl --tgid 44` would monitor process B's second thread.
          
          This diagram represents the common meaning of pid and tgid to the user.
          (The meaning of pid and tgid is reversed in kernel-land.)
          
          [default: 0]

      --tgid <TGID>
          Thread ID to trace
          
          See '--pid' for more.
          
          [default: 0]

  -l, --min-lat-us <MIN_LAT_US>
          Trace latency higher than this value
          
          Affects '--bio_lat', '--rq_lat' and '--fs-lat'
          
          [default: 10000]

  -i, --reporting-interval-ms <REPORTING_INTERVAL_MS>
          For monitoring tools, stats will be reported at this interval
          
          Affects '--cpu-pct' and '--mem-pct'
          
          [default: 1000]

  -a, --all
          Enable all tracing and monitoring tools

      --bio-lat
          Enable block and character device i/o latency tracing

      --rq-lat
          Enable run queue latency tracing

      --fs-lat
          Enable file system latency tracing

      --cpu-pct
          Enable cpu utilization % monitoring

      --mem-pct
          Enable virtual memory utilization % monitoring

  -f, --output-format <OUTPUT_FORMAT>
          Some output styles are better for humans (columnar), others for machines
          
          - columnar
            cpu_pct  101410        systemd              1        0.00
          - csv
            cpu_pct,101459,systemd,1,0.00
          - json
            {"tool":"cpu_pct","time":"101363","task":"systemd","pid":1,"value":0.00}
          
          [default: columnar]
          [possible values: columnar, csv, json]

      --duration-format <DURATION_FORMAT>
          Output format for the duration since this program's start
          
          This is not the duration since the target process(es) or threads began.
          
          [default: usecs]
          [possible values: hh-mm-ss, hh-mm-ss-mss, usecs]

      --header
          Show a header (tool/time/task/pid/value) as the first time of output
          
          Has no effect when the output format ('-f, --output-format') is json.
          Formatted according to the output format.

      --just-header
          Show a header and exit ('-V, --version' has precedence)
          
          See '--header' for more.

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
