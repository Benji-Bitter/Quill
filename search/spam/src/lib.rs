use std::collections::HashMap;
/// A bounded heuristic, not a judgment about truth or authorship.
pub fn repetition_penalty(tokens: &[String]) -> f64 {
    if tokens.len() < 30 {
        return 1.0;
    }
    let mut counts = HashMap::new();
    for token in tokens {
        *counts.entry(token).or_insert(0usize) += 1;
    }
    let share = counts.values().copied().max().unwrap_or(0) as f64 / tokens.len() as f64;
    if share > 0.25 { 0.5 } else { 1.0 }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repetitive_text_is_downweighted() {
        assert_eq!(repetition_penalty(&vec!["buy".into(); 100]), 0.5);
        assert_eq!(repetition_penalty(&["buy".into()]), 1.0);
    }
}
