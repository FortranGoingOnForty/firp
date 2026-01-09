//! Suggestion generation for "did you mean?" diagnostics
//!
//! This module provides:
//! - Levenshtein distance calculation for typo detection
//! - Suggestion generation from symbol tables

/// Calculate the Levenshtein (edit) distance between two strings.
/// This measures the minimum number of single-character edits (insertions,
/// deletions, or substitutions) required to change one string into another.
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    // Early returns for edge cases
    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    // Use only two rows (current and previous) for space efficiency
    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row: Vec<usize> = vec![0; b_len + 1];

    for (i, a_char) in a_chars.iter().enumerate() {
        curr_row[0] = i + 1;

        for (j, b_char) in b_chars.iter().enumerate() {
            let cost = if a_char.eq_ignore_ascii_case(b_char) { 0 } else { 1 };
            curr_row[j + 1] = (prev_row[j + 1] + 1)  // deletion
                .min(curr_row[j] + 1)                // insertion
                .min(prev_row[j] + cost);            // substitution
        }

        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
}

/// Find suggestions from a list of candidates for a given name.
/// Returns suggestions sorted by similarity (best match first).
///
/// # Arguments
/// * `name` - The name to find suggestions for
/// * `candidates` - A list of valid names to suggest from
/// * `max_distance` - Maximum edit distance to consider (default: 3)
/// * `max_suggestions` - Maximum number of suggestions to return (default: 3)
pub fn find_suggestions(
    name: &str,
    candidates: &[String],
    max_distance: Option<usize>,
    max_suggestions: Option<usize>,
) -> Vec<String> {
    let max_dist = max_distance.unwrap_or(3);
    let max_sugg = max_suggestions.unwrap_or(3);

    // Don't suggest for very short names
    if name.len() <= 1 {
        return Vec::new();
    }

    let mut scored: Vec<(usize, &String)> = candidates
        .iter()
        .filter_map(|candidate| {
            // Skip if name lengths are too different
            let len_diff = (name.len() as isize - candidate.len() as isize).unsigned_abs();
            if len_diff > max_dist {
                return None;
            }

            let distance = levenshtein_distance(name, candidate);
            if distance <= max_dist && distance > 0 {
                Some((distance, candidate))
            } else {
                None
            }
        })
        .collect();

    // Sort by distance, then alphabetically
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));

    scored
        .into_iter()
        .take(max_sugg)
        .map(|(_, s)| s.clone())
        .collect()
}

/// Format suggestions as a help message
pub fn format_suggestions(suggestions: &[String]) -> Option<String> {
    match suggestions.len() {
        0 => None,
        1 => Some(format!("did you mean '{}'?", suggestions[0])),
        2 => Some(format!("did you mean '{}' or '{}'?", suggestions[0], suggestions[1])),
        _ => {
            let last = &suggestions[suggestions.len() - 1];
            let rest: Vec<_> = suggestions[..suggestions.len() - 1]
                .iter()
                .map(|s| format!("'{}'", s))
                .collect();
            Some(format!("did you mean {}, or '{}'?", rest.join(", "), last))
        }
    }
}

/// Suggest similar identifiers for an undeclared variable
pub fn suggest_similar_identifiers(name: &str, scope_names: &[String]) -> Vec<String> {
    find_suggestions(name, scope_names, Some(2), Some(3))
}

/// Check if a name might be a typo of a keyword
pub fn suggest_keyword_typo(name: &str) -> Option<&'static str> {
    let keywords = [
        "PROGRAM", "MODULE", "SUBROUTINE", "FUNCTION", "END",
        "IF", "THEN", "ELSE", "ELSEIF", "ENDIF",
        "DO", "WHILE", "ENDDO", "EXIT", "CYCLE",
        "SELECT", "CASE", "DEFAULT",
        "INTEGER", "REAL", "DOUBLE", "COMPLEX", "CHARACTER", "LOGICAL",
        "DIMENSION", "ALLOCATABLE", "POINTER", "TARGET", "OPTIONAL",
        "INTENT", "PARAMETER", "SAVE", "IMPLICIT", "NONE",
        "PRINT", "WRITE", "READ", "OPEN", "CLOSE",
        "CALL", "RETURN", "STOP", "CONTAINS", "USE",
        "ALLOCATE", "DEALLOCATE", "NULLIFY",
        "PUBLIC", "PRIVATE", "INTERFACE", "ABSTRACT", "EXTENDS",
        "CLASS", "TYPE", "ASSOCIATE", "BLOCK", "PURE", "ELEMENTAL",
        "RECURSIVE", "PRESENT", "RESULT",
    ];

    let name_upper = name.to_uppercase();
    for keyword in &keywords {
        let distance = levenshtein_distance(&name_upper, keyword);
        if distance > 0 && distance <= 2 {
            return Some(keyword);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_identical() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
        assert_eq!(levenshtein_distance("HELLO", "hello"), 0); // case insensitive
    }

    #[test]
    fn test_levenshtein_empty() {
        assert_eq!(levenshtein_distance("", "hello"), 5);
        assert_eq!(levenshtein_distance("hello", ""), 5);
        assert_eq!(levenshtein_distance("", ""), 0);
    }

    #[test]
    fn test_levenshtein_one_char_diff() {
        assert_eq!(levenshtein_distance("hello", "hallo"), 1); // substitution
        assert_eq!(levenshtein_distance("hello", "hell"), 1);  // deletion
        assert_eq!(levenshtein_distance("hell", "hello"), 1);  // insertion
    }

    #[test]
    fn test_levenshtein_multiple_diffs() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("saturday", "sunday"), 3);
    }

    #[test]
    fn test_find_suggestions() {
        let candidates = vec![
            "counter".to_string(),
            "count".to_string(),
            "index".to_string(),
            "total".to_string(),
        ];

        let suggestions = find_suggestions("counte", &candidates, None, None);
        assert!(suggestions.contains(&"counter".to_string()));
        assert!(suggestions.contains(&"count".to_string()));

        let suggestions = find_suggestions("cont", &candidates, None, None);
        assert!(suggestions.contains(&"count".to_string()));
    }

    #[test]
    fn test_find_suggestions_no_match() {
        let candidates = vec!["apple".to_string(), "banana".to_string()];
        let suggestions = find_suggestions("xyz", &candidates, None, None);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_format_suggestions_single() {
        let suggestions = vec!["counter".to_string()];
        assert_eq!(format_suggestions(&suggestions), Some("did you mean 'counter'?".to_string()));
    }

    #[test]
    fn test_format_suggestions_double() {
        let suggestions = vec!["counter".to_string(), "count".to_string()];
        assert_eq!(format_suggestions(&suggestions), Some("did you mean 'counter' or 'count'?".to_string()));
    }

    #[test]
    fn test_format_suggestions_triple() {
        let suggestions = vec!["counter".to_string(), "count".to_string(), "counting".to_string()];
        assert_eq!(format_suggestions(&suggestions), Some("did you mean 'counter', 'count', or 'counting'?".to_string()));
    }

    #[test]
    fn test_format_suggestions_empty() {
        let suggestions: Vec<String> = vec![];
        assert_eq!(format_suggestions(&suggestions), None);
    }

    #[test]
    fn test_suggest_keyword_typo() {
        assert_eq!(suggest_keyword_typo("INTEGR"), Some("INTEGER"));
        assert_eq!(suggest_keyword_typo("PRINTT"), Some("PRINT"));
        assert_eq!(suggest_keyword_typo("FUNCTOIN"), Some("FUNCTION"));
        assert_eq!(suggest_keyword_typo("xyz"), None);
    }

    #[test]
    fn test_suggest_similar_identifiers() {
        let names = vec![
            "counter".to_string(),
            "result".to_string(),
            "total".to_string(),
        ];

        let suggestions = suggest_similar_identifiers("countr", &names);
        assert!(suggestions.contains(&"counter".to_string()));
    }
}
