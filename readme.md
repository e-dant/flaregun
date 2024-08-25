# Flaregun

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

This is a library as well as a few command-line tools.

```
Usage: fl [OPTIONS]

Options:
  -a, --all
          Enable all tracing and monitoring tools
      --bio-lat
          Enable block and character device i/o latency tracing
      --rq-lat
          Enable run queue latency tracing
      --fs-lat
          Enable file system latency tracing
      --tcp-pkt-lat
          Enable TCP packet latency tracing
      --cpu-pct
          Enable cpu utilization % monitoring
      --mem-pct
          Enable virtual memory utilization % monitoring
  -p, --pid <PID>
          Process ID to trace, or 0 for everything [default: 0]
      --tgid <TGID>
          Thread ID to trace [default: 0]
  -l, --min-lat-us <MIN_LAT_US>
          Trace latency higher than this value [default: 10000]
  -i, --reporting-interval-ms <REPORTING_INTERVAL_MS>
          For monitoring tools, stats will be reported at this interval [default: 1000]
  -f, --output-format <OUTPUT_FORMAT>
          Some output styles are better for humans (columnar), others for machines [default: columnar] [possible values: columnar, csv, json]
      --duration-format <DURATION_FORMAT>
          Output format for the duration since this program's start [default: usecs] [possible values: hh-mm-ss, hh-mm-ss-mss, usecs]
  -o, --output-file <OUTPUT_FILE>
          Write events to this file, if present, or to standard output if not given
      --no-header
          Omit the header (tool/time/task/pid/value) as the first line of output
      --just-header
          Show a header and exit ('-V, --version' has precedence)
  -h, --help
          Print help (see more with '--help')
  -V, --version
          Print version
```
