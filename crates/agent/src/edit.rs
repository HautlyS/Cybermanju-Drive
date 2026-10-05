// Hash-anchored edits (hashline-inspired, BLAKE3-addressed).
//
// The model replaces one exact `old_block` with `new_block` instead of
// retyping lines, and may pin the file with `expected_hash` (BLAKE3 hex of
// the whole file before editing). Refusals carry machine prefixes:
// `not_found:` (block absent), `conflict:` (block ambiguous — resend a
// bigger block), `integrity:` (hash moved under you). Pure bytes in/out so
// every transport — and the unit tests — share one applier.

/// Apply one anchored replacement. Returns the new file bytes.
pub fn apply_edit(
    current: &str,
    old_block: &str,
    new_block: &str,
    expected_hash: Option<&str>,
) -> Result<String, String> {
    if old_block.is_empty() {
        return Err("invalid: old_block is empty".to_string());
    }
    if let Some(expected) = expected_hash {
        let actual = blake3_hex(current.as_bytes());
        if actual != expected {
            return Err(format!(
                "integrity: file changed since anchor (expected {expected}, got {actual}) — re-read and retry"
            ));
        }
    }
    let hits = current.matches(old_block).count();
    if hits == 0 {
        return Err("not_found: old_block does not occur in the file — re-read and retry".to_string());
    }
    if hits > 1 {
        return Err(format!(
            "conflict: old_block occurs {hits} times — resend a larger, unique block"
        ));
    }
    Ok(current.replacen(old_block, new_block, 1))
}

/// BLAKE3 hex of bytes (the anchor primitive).
pub fn blake3_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchored_edit_replaces_exactly_once() {
        let out = apply_edit("fn a() {}\nfn b() {}\n", "fn a() {}", "fn a() {\n  1\n}", None)
            .expect("edit");
        assert!(out.contains("fn a() {\n  1\n}"));
        assert!(out.contains("fn b() {}"));
    }

    #[test]
    fn missing_and_ambiguous_blocks_refuse_honestly() {
        assert!(apply_edit("aaa", "zzz", "b", None)
            .expect_err("missing")
            .starts_with("not_found:"));
        assert!(apply_edit("x\nx\n", "x", "y", None)
            .expect_err("ambiguous")
            .starts_with("conflict:"));
        assert!(apply_edit("", "", "y", None)
            .expect_err("empty")
            .starts_with("invalid:"));
    }

    #[test]
    fn stale_anchor_is_an_integrity_error() {
        let content = "hello";
        let wrong = "0".repeat(64);
        assert!(apply_edit(content, "hello", "bye", Some(&wrong))
            .expect_err("stale")
            .starts_with("integrity:"));
        let right = blake3_hex(content.as_bytes());
        assert_eq!(
            apply_edit(content, "hello", "bye", Some(&right)).expect("fresh"),
            "bye"
        );
    }
}
