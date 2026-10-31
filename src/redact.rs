use std::sync::LazyLock;

use regex::Regex;

const REDACTED: &str = "[REDACTED]";

struct Rule {
    pattern: Regex,
    replacement: &'static str,
}

static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    let rules = [
        (
            r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----",
            REDACTED,
        ),
        (r"(?i)bearer\s+[A-Za-z0-9\-._~+/]+=*", "Bearer [REDACTED]"),
        (r"\bsk-[A-Za-z0-9]{16,}\b", REDACTED),
        (r"\bAKIA[0-9A-Z]{16}\b", REDACTED),
        (
            r"(?i)\b(api[_-]?key|secret|password|token)\b(\s*[:=]\s*)(\S+)",
            "$1$2[REDACTED]",
        ),
    ];
    rules
        .into_iter()
        .map(|(pattern, replacement)| Rule {
            pattern: Regex::new(pattern).expect("redaction pattern compiles"),
            replacement,
        })
        .collect()
});

/// Replace obvious secrets. The returned flag is true when the text changed.
pub fn redact(input: &str) -> (String, bool) {
    let mut output = input.to_string();
    for rule in RULES.iter() {
        if rule.pattern.is_match(&output) {
            output = rule
                .pattern
                .replace_all(&output, rule.replacement)
                .into_owned();
        }
    }
    let changed = output != input;
    (output, changed)
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn leaves_ordinary_text() {
        let (text, changed) = redact("use port 4080");
        assert_eq!(text, "use port 4080");
        assert!(!changed);
    }

    #[test]
    fn redacts_common_secret_shapes() {
        let (text, changed) = redact(
            "Authorization: Bearer abc.def-ghi token=supersecret sk-abcdefghijklmnopqrstuvwxyz AKIAIOSFODNN7EXAMPLE",
        );
        assert!(changed);
        assert!(!text.contains("abc.def"));
        assert!(!text.contains("supersecret"));
        assert!(!text.contains("sk-abc"));
        assert!(!text.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(text.contains("[REDACTED]"));
    }

    #[test]
    fn redacts_a_private_key_block() {
        let raw = "-----BEGIN PRIVATE KEY-----\nsecret-bytes\n-----END PRIVATE KEY-----";
        let (text, changed) = redact(raw);
        assert!(changed);
        assert_eq!(text, "[REDACTED]");
    }
}
