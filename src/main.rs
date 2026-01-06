mod csv_reader;
mod graph;
mod parser;
mod runtime;

use anyhow::{Context, Result};
use clap::Parser;
use std::io::{self, Write};

#[derive(Parser, Debug)]
#[command(name = "gramgraph")]
#[command(about = "Generate graphs from CSV data using PlotPipe DSL", long_about = None)]
struct Args {
    /// PlotPipe DSL string (e.g., 'chart(x: time, y: temp) | layer_line(color: "red")')
    dsl: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Read CSV from stdin
    let csv_data = csv_reader::read_csv_from_stdin()
        .context("Failed to read CSV from stdin")?;

    // Parse the DSL string
    let pipeline = match parser::parse_pipeline(&args.dsl) {
        Ok((remaining, pipeline)) => {
            if !remaining.trim().is_empty() {
                eprintln!("Warning: unparsed input: '{}'", remaining);
            }
            pipeline
        }
        Err(e) => {
            eprintln!("Parse error: {:?}", e);
            std::process::exit(1);
        }
    };

    // Execute the pipeline
    let png_bytes = runtime::execute_pipeline(pipeline, csv_data)
        .context("Failed to execute pipeline")?;

    // Write PNG to stdout
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_all(&png_bytes)
        .context("Failed to write PNG to stdout")?;
    handle.flush().context("Failed to flush stdout")?;

    Ok(())
}
