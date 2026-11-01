//! Subsequence fuzzy matching for the command palette and quick open.

/// Score `candidate` against `query`; `None` when the query is not a subsequence.
/// Higher is better: matches at the start, after separators and in a row score more.
pub fn score(query: &str, candidate: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let needle: Vec<char> = query.to_lowercase().chars().collect();
    let hay: Vec<char> = candidate.chars().collect();
    let lower: Vec<char> = candidate.to_lowercase().chars().collect();
    if lower.len() != hay.len() {
        // Case folding changed the length (rare); fall back to a plain containment test.
        return candidate
            .to_lowercase()
            .contains(&query.to_lowercase())
            .then_some(1);
    }
    let mut total = 0;
    let mut position = 0;
    let mut previous: Option<usize> = None;
    for wanted in needle {
        let found = (position..lower.len()).find(|&index| lower[index] == wanted)?;
        let mut gain = 1;
        if found == 0 {
            gain += 10;
        } else if matches!(hay[found - 1], '/' | '\\' | '.' | '_' | '-' | ' ') {
            gain += 8;
        } else if hay[found].is_uppercase() && hay[found - 1].is_lowercase() {
            gain += 6;
        }
        if previous.is_some_and(|last| found == last + 1) {
            gain += 5;
        }
        total += gain;
        previous = Some(found);
        position = found + 1;
    }
    // Prefer shorter candidates when everything else is equal.
    Some(total * 10 - (hay.len() as i32).min(200))
}

/// Indexes of `items` that match, best first.
pub fn rank<T: AsRef<str>>(query: &str, items: &[T], limit: usize) -> Vec<usize> {
    let mut scored: Vec<(i32, usize)> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| score(query, item.as_ref()).map(|value| (value, index)))
        .collect();
    scored.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(&right.1)));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, index)| index)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_a_subsequence() {
        assert!(score("sav", "Save File").is_some());
        assert!(score("sf", "Save File").is_some());
        assert!(score("xyz", "Save File").is_none());
        assert_eq!(score("", "anything"), Some(0));
    }

    #[test]
    fn prefers_prefixes_word_starts_and_runs() {
        let items = ["Close Tab", "Save As", "Toggle Sidebar", "Save File"];
        assert_eq!(rank("save", &items, 10)[0], 1, "shorter prefix match first");
        assert_eq!(rank("ts", &items, 10)[0], 2, "word-start initials");
        assert_eq!(rank("zzz", &items, 10).len(), 0);
    }

    #[test]
    fn works_on_paths() {
        let files = ["src/main.rs", "src/tabs.rs", "docs/maintenance.md"];
        assert_eq!(rank("main", &files, 10)[0], 0);
        assert_eq!(rank("tabs", &files, 10), vec![1]);
    }

    #[test]
    fn limit_is_respected() {
        let items: Vec<String> = (0..50).map(|n| format!("item {n}")).collect();
        assert_eq!(rank("item", &items, 8).len(), 8);
    }
}
