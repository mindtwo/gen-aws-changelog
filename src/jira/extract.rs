use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::BTreeSet;

/// Matches JIRA-style keys (`PROJ-123`) case-insensitively, so a commit
/// message like `fix learn-9: typo` matches the ticket `LEARN-9`. The
/// project part must be at least 2 chars and start with a letter; the
/// numeric suffix is mandatory.
static GENERIC: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b([a-z][a-z0-9_]+)-(\d+)\b").expect("regex")
});

/// Returns the canonical ticket keys (uppercase, sorted, de-duplicated)
/// referenced in `message`. Used by the changelog renderer to attach
/// commits to JIRA tickets case-insensitively.
pub fn keys_in(message: &str) -> Vec<String> {
    let mut out = BTreeSet::new();
    for caps in GENERIC.captures_iter(message) {
        out.insert(format!("{}-{}", caps[1].to_ascii_uppercase(), &caps[2]));
    }
    out.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::keys_in;

    #[test]
    fn normalises_to_uppercase() {
        assert_eq!(keys_in("learn-9 and APP-1"), vec!["APP-1", "LEARN-9"]);
    }

    #[test]
    fn deduplicates_across_case() {
        assert_eq!(
            keys_in("learn-1 here, LEARN-1 again, Learn-1 once more"),
            vec!["LEARN-1"]
        );
    }

    #[test]
    fn ignores_single_letter_prefix() {
        assert!(keys_in("a-1 foo").is_empty());
    }

    #[test]
    fn matches_inside_subject_and_body() {
        let msg = "feat(api): learn-9 add foo\n\nRefs: APP-42";
        assert_eq!(keys_in(msg), vec!["APP-42", "LEARN-9"]);
    }
}
