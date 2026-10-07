use quill_search_common::{Result, invalid};
use quill_search_index::Index;
use std::{path::Path, time::Instant};

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("crawl") if args.len() == 3 => {
            let (index, report) = quill_search_crawler::crawl(&args[2], 30)?;
            index.save(Path::new(&args[1]))?;
            println!(
                "Fetched {}, indexed {}, denied {}, skipped {}",
                report.fetched, report.indexed, report.denied, report.skipped
            );
        }
        Some("query") if args.len() >= 3 => {
            let index = Index::load(Path::new(&args[1]))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&index.query(&args[2..].join(" "), 10))?
            );
        }
        Some("bench") if args.len() >= 3 => {
            let start = Instant::now();
            let index = Index::load(Path::new(&args[1]))?;
            let load = start.elapsed();
            let query = args[2..].join(" ");
            let mut samples = Vec::new();
            for _ in 0..1000 {
                let start = Instant::now();
                std::hint::black_box(index.query(&query, 10));
                samples.push(start.elapsed().as_nanos());
            }
            samples.sort_unstable();
            println!(
                "documents={} load_us={} query_p50_ns={} query_p95_ns={} samples=1000",
                index.len(),
                load.as_micros(),
                samples[500],
                samples[950]
            );
        }
        _ => {
            return Err(invalid(
                "usage: quill-search crawl INDEX URL | query INDEX WORDS | bench INDEX WORDS",
            ));
        }
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
