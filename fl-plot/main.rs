use clap::Parser;
use std::collections::HashMap;

#[derive(Debug, Parser)]
#[clap(version)]
struct Cli {
    /// Path to a csv input file (with a header) for plotting
    #[arg(short, long)]
    input_file: String,
    /// Path to an (html) output file for the plot
    #[arg(short, long, default_value = "fl.html")]
    output_file: String,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct Event {
    tool: String,
    time: u64,
    task: String,
    pid: i32,
    value: f64,
}

type PlottableByTool = HashMap<String, Vec<(u64, f64)>>;

fn many_plottable_from_csv_file_by_tool(
    file_path: &str,
) -> Result<PlottableByTool, Box<dyn std::error::Error>> {
    let mut rdr = csv::ReaderBuilder::new().from_reader(std::fs::File::open(file_path)?);
    let mut evs = PlottableByTool::new();
    for r in rdr.deserialize::<Event>() {
        match r {
            Ok(event) => {
                let entry = evs.entry(event.tool.clone()).or_default();
                entry.push((event.time, event.value));
            }
            Err(e) => log::error!("Error parsing CSV: {e} in {file_path}"),
        }
    }
    Ok(evs)
}

fn plot_from_csv_file(file_path: &str) -> Result<plotly::Plot, Box<dyn std::error::Error>> {
    let mut p = plotly::Plot::new();
    p.set_layout(plotly::layout::Layout::new());
    for (tool, values) in many_plottable_from_csv_file_by_tool(file_path)? {
        let t = plotly::Scatter::new(
            values.iter().map(|(x, _)| *x).collect(),
            values.iter().map(|(_, y)| *y).collect(),
        )
        .mode(plotly::common::Mode::LinesMarkers)
        .name(tool);
        p.add_trace(t);
    }
    Ok(p)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Cli::parse();
    plot_from_csv_file(&opts.input_file)?.write_html(&opts.output_file);
    Ok(())
}
