/// BM25 with fixed, inspectable constants. There is no source reputation table.
pub fn bm25(
    tf: u32,
    document_frequency: usize,
    documents: usize,
    length: usize,
    average_length: f64,
) -> f64 {
    if tf == 0 || documents == 0 {
        return 0.0;
    }
    let idf = (1.0
        + (documents as f64 - document_frequency as f64 + 0.5) / (document_frequency as f64 + 0.5))
        .ln();
    let normalization = 1.2 * (0.25 + 0.75 * length as f64 / average_length.max(1.0));
    idf * (tf as f64 * 2.2) / (tf as f64 + normalization)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frequency_saturates_and_rare_terms_matter() {
        assert!(bm25(1, 1, 10, 100, 100.0) > bm25(1, 8, 10, 100, 100.0));
        assert!(bm25(100, 1, 10, 100, 100.0) < 3.0 * bm25(1, 1, 10, 100, 100.0));
        assert_eq!(bm25(0, 0, 0, 0, 0.0), 0.0);
    }
}
