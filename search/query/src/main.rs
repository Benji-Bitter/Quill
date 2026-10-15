use quill_search_common::{Result, invalid};
use quill_search_index::Index;
use std::{path::Path, time::Instant};

fn writer_lock(path: &Path) -> Result<std::fs::File> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path.with_extension("lock"))?;
    file.try_lock()
        .map_err(|_| invalid("another crawl is writing this index"))?;
    Ok(file)
}

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("crawl") if args.len() == 3 => {
            let _lock = writer_lock(Path::new(&args[1]))?;
            let (index, report) = quill_search_crawler::crawl(&args[2], 30)?;
            index.save(Path::new(&args[1]))?;
            println!(
                "Fetched {}, indexed {}, denied {}, skipped {}",
                report.fetched, report.indexed, report.denied, report.skipped
            );
        }
        Some("collect") if args.len() >= 4 => {
            let path = Path::new(&args[1]);
            let _lock = writer_lock(path)?;
            let pages: usize = args[2].parse().map_err(|_| invalid("invalid page limit"))?;
            let mut index = if path.exists() {
                Index::load(path)?
            } else {
                Index::default()
            };
            for seed in &args[3..] {
                match quill_search_crawler::crawl(seed, pages) {
                    Ok((crawled, report)) => {
                        let added = index.merge(crawled)?;
                        index.save(path)?;
                        println!(
                            "{}",
                            serde_json::json!({"seed":seed,"added":added,"total":index.len(),"fetched":report.fetched,"denied":report.denied,"skipped":report.skipped})
                        );
                    }
                    Err(error) => println!(
                        "{}",
                        serde_json::json!({"seed":seed,"error":error.to_string()})
                    ),
                }
            }
        }
        Some("stats") if args.len() == 2 => {
            let index = if Path::new(&args[1]).exists() {
                Index::load(Path::new(&args[1]))?
            } else {
                Index::default()
            };
            let mut origins = std::collections::BTreeMap::<String, usize>::new();
            for document in index.documents() {
                *origins
                    .entry(quill_search_crawler::seed_origin(&document.url)?)
                    .or_default() += 1;
            }
            println!(
                "{}",
                serde_json::json!({"documents":index.len(),"origins":origins})
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
                "usage: quill-search crawl INDEX URL | query INDEX WORDS | bench INDEX WORDS | collect INDEX PAGES URL... | stats INDEX",
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn writers_are_exclusive_and_release_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.json");
        let first = writer_lock(&path).unwrap();
        assert!(writer_lock(&path).is_err());
        drop(first);
        assert!(writer_lock(&path).is_ok());
    }
}
