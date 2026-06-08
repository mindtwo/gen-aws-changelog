use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::BTreeSet;

static GENERIC: Lazy<Regex> = Lazy::new(|| {
    // Match standard JIRA keys: uppercase project key followed by `-NNN`.
    Regex::new(r"\b([A-Z][A-Z0-9_]+)-(\d+)\b").expect("regex")
});

/// Extract ticket keys (e.g. `LEARN-123`) from the given messages.
/// If `prefixes` is non-empty, only keys whose project key is in the list
/// are returned. Output is sorted and de-duplicated.
pub fn extract_keys<'a, I: IntoIterator<Item = &'a str>>(
    messages: I,
    prefixes: &[String],
) -> Vec<String> {
    let allow: Option<BTreeSet<&str>> = if prefixes.is_empty() {
        None
    } else {
        Some(prefixes.iter().map(|s| s.as_str()).collect())
    };
    let mut out = BTreeSet::new();
    for msg in messages {
        for caps in GENERIC.captures_iter(msg) {
            let prefix = &caps[1];
            if let Some(allow) = &allow {
                if !allow.contains(prefix) {
                    continue;
                }
            }
            out.insert(format!("{}-{}", prefix, &caps[2]));
        }
    }
    out.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::extract_keys;

    #[test]
    fn extracts_basic() {
        let msgs = ["LEARN-123 fix bug", "Refactor LEARN-9 and APP-42"];
        assert_eq!(
            extract_keys(msgs.iter().copied(), &[]),
            vec!["APP-42", "LEARN-123", "LEARN-9"]
        );
    }

    #[test]
    fn filters_by_prefix() {
        let msgs = ["LEARN-123 foo", "APP-1 bar"];
        let out = extract_keys(msgs.iter().copied(), &["LEARN".to_string()]);
        assert_eq!(out, vec!["LEARN-123"]);
    }

    #[test]
    fn deduplicates() {
        let msgs = ["LEARN-1 here", "and LEARN-1 again"];
        assert_eq!(extract_keys(msgs.iter().copied(), &[]), vec!["LEARN-1"]);
    }

    #[test]
    fn ignores_lowercase_words() {
        let msgs = ["nothing-here", "abc-1 should not match"];
        assert!(extract_keys(msgs.iter().copied(), &[]).is_empty());
    }
}
