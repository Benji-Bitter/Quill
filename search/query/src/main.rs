use quill_search_common::{Result, exclusions, invalid};
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

fn load_index(path: &Path) -> Result<Index> {
    let mut index = Index::load(path)?;
    let domains = exclusions::read(path)?;
    let remove: Vec<_> = index
        .documents()
        .iter()
        .filter(|d| exclusions::matches(&d.url, &domains))
        .map(|d| d.url.clone())
        .collect();
    if !remove.is_empty() {
        index.remove_urls(&remove)?;
    }
    Ok(index)
}

fn crawl_allowed(
    path: &Path,
    seed: &str,
    pages: usize,
) -> Result<(Index, quill_search_crawler::Report)> {
    if exclusions::matches(seed, &exclusions::read(path)?) {
        return Err(invalid("source excluded by your local domain settings"));
    }
    quill_search_crawler::crawl(seed, pages)
}

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("crawl") if args.len() == 3 => {
            let _lock = writer_lock(Path::new(&args[1]))?;
            let (index, report) = crawl_allowed(Path::new(&args[1]), &args[2], 30)?;
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
                load_index(path)?
            } else {
                Index::default()
            };
            for seed in &args[3..] {
                match crawl_allowed(path, seed, pages) {
                    Ok((crawled, report)) => {
                        let added = index.merge(crawled)?;
                        index.save(path)?;
                        println!(
                            "{}",
                            serde_json::json!({"seed":seed,"added":added,"total":index.len(),"fetched":report.fetched,"denied":report.denied,"skipped":report.skipped,"time_limited":report.time_limited})
                        );
                    }
                    Err(error) => println!(
                        "{}",
                        serde_json::json!({"seed":seed,"error":error.to_string()})
                    ),
                }
            }
        }
        Some("remove") if args.len() >= 3 => {
            let path = Path::new(&args[1]);
            let _lock = writer_lock(path)?;
            let mut index = load_index(path)?;
            let removed = index.remove_urls(&args[2..])?;
            index.save(path)?;
            println!(
                "{}",
                serde_json::json!({"removed":removed,"total":index.len()})
            );
        }
        Some("stats") if args.len() == 2 => {
            let index = if Path::new(&args[1]).exists() {
                load_index(Path::new(&args[1]))?
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
            let index = load_index(Path::new(&args[1]))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&index.query(&args[2..].join(" "), 10))?
            );
        }
        Some("bench") if args.len() >= 3 => {
            let start = Instant::now();
            let index = load_index(Path::new(&args[1]))?;
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
                "usage: quill-search crawl INDEX URL | query INDEX WORDS | bench INDEX WORDS | collect INDEX PAGES URL... | stats INDEX | remove INDEX URL...",
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
    fn exclusions_filter_saved_results_and_stop_crawls_before_network() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.json");
        let mut index = Index::default();
        index
            .insert(quill_search_common::Document {
                url: "https://example.org/story".into(),
                title: "Story".into(),
                text: "sample reporting".into(),
                fetched_at: Some(123),
                published_at: None,
            })
            .unwrap();
        index.save(&path).unwrap();
        std::fs::write(path.with_file_name("excluded-domains.txt"), "example.org\n").unwrap();
        assert!(load_index(&path).unwrap().query("reporting", 10).is_empty());
        let result = crawl_allowed(&path, "https://www.example.org/", 1);
        assert!(matches!(result, Err(error) if error.to_string().contains("source excluded")));
    }
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
