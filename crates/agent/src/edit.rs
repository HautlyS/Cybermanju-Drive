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
        // Second chance: whitespace-normalized match (tabs vs spaces, trailing
        // whitespace). Still requires exactly one candidate, and the
        // replacement keeps the model's own whitespace — only *locating* is
        // fuzzy, never the written bytes.
        match fuzzy_locate(current, old_block) {
            Ok(Some(unique)) => {
                let mut out = current.to_string();
                out.replace_range(unique, new_block);
                return Ok(out);
            }
            Ok(None) => {
                return Err(
                    "not_found: old_block does not occur in the file — re-read and retry"
                        .to_string(),
                )
            }
            Err(count) => {
                return Err(format!(
                    "conflict: old_block occurs {count} times (whitespace-normalized) — resend a larger, unique block"
                ));
            }
        }
    }
    if hits > 1 {
        return Err(format!(
            "conflict: old_block occurs {hits} times — resend a larger, unique block"
        ));
    }
    Ok(current.replacen(old_block, new_block, 1))
}

/// Locate `needle` in `haystack` ignoring runs of whitespace (and
/// indentation width). `Ok(Some(range))` on a unique match, `Ok(None)` when
/// absent, `Err(count)` when it matches more than once (ambiguity must be
/// reported, never silently resolved).
fn fuzzy_locate(haystack: &str, needle: &str) -> Result<Option<std::ops::Range<usize>>, usize> {
    /// Collapse whitespace runs to one space; `map[i]` is the original byte
    /// offset where normalized byte `i` came from (so every mapped span lands
    /// on char boundaries by construction).
    fn squash(text: &str) -> (String, Vec<usize>) {
        let mut out = String::new();
        let mut map = Vec::new();
        let mut in_gap = false;
        for (byte, ch) in text.char_indices() {
            if ch.is_whitespace() {
                if !in_gap {
                    out.push(' ');
                    map.push(byte);
                    in_gap = true;
                }
                continue;
            }
            in_gap = false;
            let before = out.len();
            out.push(ch);
            map.extend(std::iter::repeat_n(byte, out.len() - before));
        }
        (out, map)
    }
    let (hay, map) = squash(haystack);
    let (ndl, _) = squash(needle);
    if ndl.trim().is_empty() {
        return Ok(None);
    }
    let mut occurrences: Vec<usize> = Vec::new();
    let mut from = 0;
    while let Some(at) = hay[from..].find(&ndl) {
        occurrences.push(from + at);
        from += at + ndl.len();
        if from >= hay.len() {
            break;
        }
    }
    if occurrences.len() > 1 {
        return Err(occurrences.len());
    }
    let first = match occurrences.first() {
        Some(first) => *first,
        None => return Ok(None),
    };
    let start = map[first];
    let end = map
        .get(first + ndl.len())
        .copied()
        .unwrap_or(haystack.len());
    if start >= end {
        return Ok(None);
    }
    Ok(Some(start..end))
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
        let out = apply_edit(
            "fn a() {}\nfn b() {}\n",
            "fn a() {}",
            "fn a() {\n  1\n}",
            None,
        )
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
    fn whitespace_differences_fall_back_to_fuzzy_match() {
        // Model retyped with spaces what the file holds as a tab.
        let out = apply_edit(
            "fn a() {\n\treturn 1;\n}\n",
            "fn a() {\n  return 1;\n}",
            "fn a() {\n  return 2;\n}",
            None,
        )
        .expect("fuzzy");
        assert!(out.contains("return 2;"), "{out}");
        assert!(!out.contains("\treturn"), "{out}");

        // Trailing whitespace is ignored while locating.
        let out = apply_edit("x = 1;  \n", "x = 1;", "x = 2;", None).expect("trailing");
        assert!(out.contains("x = 2;"), "{out}");

        // …but ambiguity is still refused, fuzzy or not.
        assert!(apply_edit("a  b\na\tb\n", "a b", "c", None)
            .expect_err("ambiguous")
            .starts_with("conflict:"));
    }

    #[test]
    fn fuzzy_match_is_byte_exact_on_unicode() {
        let content = "héllo wörld\n";
        let out = apply_edit(content, "héllo   wörld", "héllo Rust", None).expect("unicode");
        assert_eq!(out, "héllo Rust\n");
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
