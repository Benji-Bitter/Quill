use quill_search_common::{
    Document, MAX_DOCUMENTS, MAX_INDEX_BYTES, MAX_TEXT_CHARS, Result, canonicalize, content_hash,
    invalid, tokens,
};
use quill_search_ranking::bm25;
use quill_search_spam::repetition_penalty;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Read, Write},
    path::Path,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Posting {
    document: usize,
    title_frequency: u32,
    positions: Vec<u32>,
}

#[derive(Default)]
pub struct Index {
    documents: Vec<Document>,
    postings: BTreeMap<String, Vec<Posting>>,
    lengths: Vec<usize>,
    quality: Vec<f64>,
    hashes: BTreeSet<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Segment {
    version: u32,
    documents: Vec<Document>,
    postings: BTreeMap<String, Vec<Posting>>,
}

#[derive(Debug, Serialize)]
pub struct Hit {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub score: f64,
    pub matched_terms: Vec<String>,
    pub repetition_factor: f64,
}

impl Index {
    pub fn documents(&self) -> &[Document] {
        &self.documents
    }
    pub fn merge(&mut self, other: Self) -> Result<usize> {
        let before = self.len();
        let replaced: BTreeSet<_> = other.documents.iter().map(|d| d.url.as_str()).collect();
        let mut updated = Self::default();
        for document in &self.documents {
            if !replaced.contains(document.url.as_str()) {
                updated.insert(document.clone())?;
            }
        }
        for document in &other.documents {
            updated.insert(document.clone())?;
        }
        let added = updated.len().saturating_sub(before);
        *self = updated;
        Ok(added)
    }
    pub fn len(&self) -> usize {
        self.documents.len()
    }
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
    pub fn insert(&mut self, mut document: Document) -> Result<bool> {
        if self.len() >= MAX_DOCUMENTS {
            return Err(invalid("document limit reached"));
        }
        if document.text.chars().count() > MAX_TEXT_CHARS
            || document.title.chars().count() > 512
            || document.url.len() > 8192
        {
            return Err(invalid("document exceeds limits"));
        }
        document.url = canonicalize(&document.url)?.into();
        let body = tokens(&document.text);
        let hash = content_hash(&body.join(" "));
        if body.is_empty()
            || self.hashes.contains(&hash)
            || self.documents.iter().any(|d| d.url == document.url)
        {
            return Ok(false);
        }
        let title = tokens(&document.title);
        let id = self.len();
        let mut terms: BTreeMap<String, Posting> = BTreeMap::new();
        for (position, term) in body.iter().enumerate() {
            terms
                .entry(term.clone())
                .or_insert_with(|| Posting {
                    document: id,
                    title_frequency: 0,
                    positions: vec![],
                })
                .positions
                .push(position as u32);
        }
        for term in title {
            terms
                .entry(term)
                .or_insert_with(|| Posting {
                    document: id,
                    title_frequency: 0,
                    positions: vec![],
                })
                .title_frequency += 1;
        }
        for (term, posting) in terms {
            self.postings.entry(term).or_default().push(posting);
        }
        self.lengths.push(body.len());
        self.quality.push(repetition_penalty(&body));
        self.hashes.insert(hash);
        self.documents.push(document);
        Ok(true)
    }
    pub fn query(&self, query: &str, limit: usize) -> Vec<Hit> {
        if query.len() > 4096 || self.is_empty() {
            return vec![];
        }
        let terms: BTreeSet<_> = tokens(query).into_iter().take(32).collect();
        let average = self.lengths.iter().sum::<usize>() as f64 / self.len() as f64;
        let mut scores = BTreeMap::<usize, f64>::new();
        for term in &terms {
            if let Some(postings) = self.postings.get(term) {
                for posting in postings {
                    let tf = posting.positions.len() as u32 + 3 * posting.title_frequency;
                    *scores.entry(posting.document).or_default() += bm25(
                        tf,
                        postings.len(),
                        self.len(),
                        self.lengths[posting.document],
                        average,
                    ) * self.quality
                        [posting.document];
                }
            }
        }
        let mut scored: Vec<_> = scores.into_iter().collect();
        scored.sort_by(|a, b| {
            b.1.total_cmp(&a.1)
                .then(self.documents[a.0].url.cmp(&self.documents[b.0].url))
        });
        scored
            .into_iter()
            .take(limit.min(100))
            .map(|(id, score)| {
                let d = &self.documents[id];
                let words: Vec<_> = d.text.split_whitespace().collect();
                let start = words
                    .iter()
                    .position(|w| tokens(w).iter().any(|t| terms.contains(t)))
                    .unwrap_or(0)
                    .saturating_sub(8);
                Hit {
                    title: d.title.clone(),
                    url: d.url.clone(),
                    snippet: words
                        .iter()
                        .skip(start)
                        .take(40)
                        .copied()
                        .collect::<Vec<_>>()
                        .join(" "),
                    score,
                    matched_terms: terms
                        .iter()
                        .filter(|term| {
                            self.postings
                                .get(*term)
                                .is_some_and(|ps| ps.iter().any(|p| p.document == id))
                        })
                        .cloned()
                        .collect(),
                    repetition_factor: self.quality[id],
                }
            })
            .collect()
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        let segment = Segment {
            version: 1,
            documents: self.documents.clone(),
            postings: self.postings.clone(),
        };
        let bytes = serde_json::to_vec(&segment)?;
        if bytes.len() as u64 > MAX_INDEX_BYTES {
            return Err(invalid("index exceeds size limit"));
        }
        let mut file = tempfile::NamedTempFile::new_in(
            path.parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )?;
        file.write_all(&bytes)?;
        file.as_file().sync_all()?;
        file.persist(path).map_err(|e| e.error)?;
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self> {
        let mut bytes = Vec::new();
        File::open(path)?
            .take(MAX_INDEX_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_INDEX_BYTES {
            return Err(invalid("index exceeds size limit"));
        }
        let segment: Segment = serde_json::from_slice(&bytes)?;
        if segment.version != 1 || segment.documents.len() > MAX_DOCUMENTS {
            return Err(invalid("unsupported or oversized index"));
        }
        let mut index = Self::default();
        for doc in segment.documents {
            if !index.insert(doc)? {
                return Err(invalid("duplicate or empty document in index"));
            }
        }
        // Reconstruct once at load to reject corrupt IDs, positions and frequencies.
        if index.postings != segment.postings {
            return Err(invalid("index postings do not match documents"));
        }
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn doc(url: &str, title: &str, text: &str) -> Document {
        Document {
            url: format!("https://example.org/{url}"),
            title: title.into(),
            text: text.into(),
        }
    }
    #[test]
    fn round_trip_ranking_and_deduplication() {
        let mut index = Index::default();
        assert!(
            index
                .insert(doc(
                    "a",
                    "Rust ownership",
                    "Ownership makes memory safe in Rust."
                ))
                .unwrap()
        );
        assert!(
            index
                .insert(doc("b", "Gardens", "Garden tools rust in the rain."))
                .unwrap()
        );
        assert!(
            !index
                .insert(doc("c", "Copy", "Ownership makes memory safe in Rust."))
                .unwrap()
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.json");
        index.save(&path).unwrap();
        let loaded = Index::load(&path).unwrap();
        assert_eq!(
            loaded.query("rust ownership", 10)[0].title,
            "Rust ownership"
        );
        assert!(loaded.query("unknown", 10).is_empty());
        let mut corrupt: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        corrupt["postings"]["rust"][0]["document"] = 999.into();
        std::fs::write(&path, serde_json::to_vec(&corrupt).unwrap()).unwrap();
        assert!(Index::load(&path).is_err());
    }
    #[test]
    fn merging_refreshes_pages_and_explains_results() {
        let mut index = Index::default();
        index.insert(doc("a", "Old title", "old body")).unwrap();
        let mut incoming = Index::default();
        incoming
            .insert(doc("a", "Solar power", "solar panels generate electricity"))
            .unwrap();
        incoming
            .insert(doc("b", "Wind", "wind turbines generate energy"))
            .unwrap();
        assert_eq!(index.merge(incoming).unwrap(), 1);
        assert!(index.query("old", 10).is_empty());
        let hits = index.query("solar electricity", 10);
        assert_eq!(hits[0].matched_terms, ["electricity", "solar"]);
        assert_eq!(hits[0].url, "https://example.org/a");
        assert!(hits[0].score > 0.0);
        assert!(hits[0].repetition_factor > 0.0 && hits[0].repetition_factor <= 1.0);
    }
    #[test]
    fn malformed_indexes_fail_without_panics() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.json");
        for input in [b"".as_slice(), b"{}", b"{\"version\":999}", b"[]", &[255]] {
            std::fs::write(&path, input).unwrap();
            assert!(Index::load(&path).is_err());
        }
    }
}
