// Permission evaluation: opencode/omp-compatible rulesets.
//
// Shape: `{"default": "ask", "bash": "allow", "edit": {"*": "deny", …}}`.
// Granular maps evaluate last-match-wins (catch-all first, specifics after);
// `*` spans any run, `?` exactly one char. Plan-kind sessions deny
// edit/write/bash unconditionally. `deny` always beats `auto_approve`.

use cybermanju_types::agent::{AgentKind, PermissionAction, PermissionRuleset};

/// Record an "allow always" decision: the tool becomes unconditionally
/// allowed. Explicit and reversible — the row shows exactly what "always"
/// meant, unlike an invisible always-list.
pub fn remember_allow(rules: &mut PermissionRuleset, tool: &str) {
    rules.rules.insert(
        tool.to_string(),
        cybermanju_types::agent::PermissionRule::Simple(PermissionAction::Allow),
    );
}

/// What the loop should do with a proposed tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    /// Pause the job and ask the user (carries a human summary).
    Ask {
        summary: String,
    },
    /// Refuse with an `unsupported:`-style machine prefix.
    Deny {
        reason: String,
    },
}

/// `*` = any run, `?` = exactly one char, everything else literal.
pub fn match_wildcard(pattern: &str, input: &str) -> bool {
    fn go(px: &[u8], ix: &[u8]) -> bool {
        if px.is_empty() {
            return ix.is_empty();
        }
        match px[0] {
            b'*' => {
                // Collapse runs, then try every split point.
                let mut p = 1;
                while p < px.len() && px[p] == b'*' {
                    p += 1;
                }
                (0..=ix.len()).any(|skip| go(&px[p..], &ix[skip..]))
            }
            b'?' => !ix.is_empty() && go(&px[1..], &ix[1..]),
            b => !ix.is_empty() && ix[0] == b && go(&px[1..], &ix[1..]),
        }
    }
    go(pattern.as_bytes(), input.as_bytes())
}

/// Glob-match `path` against `pattern`: `/`-separated segments where `*`
/// and `?` stay inside one segment and `**` crosses separators
/// (`src/**/*.rs`, `**/Cargo.toml`). Both sides are slash-normalized, so
/// Windows separators never break a match.
pub fn match_glob(pattern: &str, path: &str) -> bool {
    fn segments(text: &str) -> Vec<String> {
        text.replace('\\', "/")
            .split('/')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect()
    }
    fn go(px: &[&str], ix: &[&str]) -> bool {
        if px.is_empty() {
            return ix.is_empty();
        }
        if px[0] == "**" {
            // `**` eats zero or more whole segments (collapse runs first).
            let mut p = 1;
            while p < px.len() && px[p] == "**" {
                p += 1;
            }
            return (0..=ix.len()).any(|skip| go(&px[p..], &ix[skip..]));
        }
        if ix.is_empty() {
            return false;
        }
        match_wildcard(px[0], ix[0]) && go(&px[1..], &ix[1..])
    }
    let pattern = if pattern.trim().is_empty() {
        "**"
    } else {
        pattern
    };
    let (psegs, isegs) = (segments(pattern), segments(path));
    let psegs: Vec<&str> = psegs.iter().map(String::as_str).collect();
    let isegs: Vec<&str> = isegs.iter().map(String::as_str).collect();
    go(&psegs, &isegs)
}

/// A grep pattern: real regex when it compiles, literal substring when it
/// does not (an invalid regex must search literally, never fail the tool).
pub enum GrepPattern {
    Regex(regex::Regex),
    Literal(String),
}

impl GrepPattern {
    pub fn compile(pattern: &str) -> Self {
        match regex::Regex::new(pattern) {
            Ok(re) => GrepPattern::Regex(re),
            Err(_) => GrepPattern::Literal(pattern.to_string()),
        }
    }

    pub fn is_match(&self, line: &str) -> bool {
        match self {
            GrepPattern::Regex(re) => re.is_match(line),
            GrepPattern::Literal(lit) => line.contains(lit.as_str()),
        }
    }

    pub fn is_regex(&self) -> bool {
        matches!(self, GrepPattern::Regex(_))
    }
}
/// The match string a tool call is evaluated against: `name` plus its most
/// salient argument, mirroring how `grep *` needs `"grep *"` while
/// bare `"grep"` only matches the argless call.
pub fn match_input(tool: &str, input: &serde_json::Value) -> String {
    let arg = salient_arg(input);
    if arg.is_empty() {
        tool.to_string()
    } else {
        format!("{tool} {arg}")
    }
}

/// The salient argument alone (`git status --porcelain`, not
/// `bash git status --porcelain`): opencode-style `bash` patterns match the
/// parsed command, so rules are tried against all three shapes.
pub fn salient_arg(input: &serde_json::Value) -> &str {
    input
        .get("command")
        .or_else(|| input.get("pattern"))
        .or_else(|| input.get("path"))
        .or_else(|| input.get("glob"))
        .or_else(|| input.get("query"))
        .or_else(|| input.get("url"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
}

/// Decide a tool call under a ruleset.
pub fn decide(
    rules: &PermissionRuleset,
    kind: AgentKind,
    tool: &str,
    input: &serde_json::Value,
) -> PermissionDecision {
    // Plan persona: read-only, whatever the rules say.
    if kind == AgentKind::Plan && matches!(tool, "edit" | "write" | "bash") {
        return PermissionDecision::Deny {
            reason: format!("deny: plan agent may not run `{tool}`"),
        };
    }
    let action = match rules.rules.get(tool) {
        None => rules.default,
        Some(cybermanju_types::agent::PermissionRule::Simple(a)) => *a,
        Some(cybermanju_types::agent::PermissionRule::Granular(pairs)) => {
            let target = match_input(tool, input);
            let arg = salient_arg(input);
            let mut hit = None;
            for (pattern, action) in pairs {
                if match_wildcard(pattern, &target)
                    || match_wildcard(pattern, tool)
                    || (!arg.is_empty() && match_wildcard(pattern, arg))
                {
                    hit = Some(*action);
                }
            }
            hit.unwrap_or(rules.default)
        }
    };
    match action {
        PermissionAction::Allow => PermissionDecision::Allow,
        PermissionAction::Deny => PermissionDecision::Deny {
            reason: format!("deny: `{tool}` is denied by the permission ruleset"),
        },
        PermissionAction::Ask => PermissionDecision::Ask {
            summary: format!("Approve `{tool}`?"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cybermanju_types::agent::PermissionRule;
    use std::collections::BTreeMap;

    fn ruleset(pairs: Vec<(&str, PermissionRule)>) -> PermissionRuleset {
        PermissionRuleset {
            default: PermissionAction::Ask,
            rules: pairs
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect::<BTreeMap<_, _>>(),
        }
    }

    #[test]
    fn wildcard_matching_follows_shell_rules() {
        assert!(match_wildcard("*", "anything at all"));
        assert!(match_wildcard("git *", "git status"));
        assert!(!match_wildcard("git *", "git"));
        assert!(match_wildcard("rm ?", "rm x"));
        assert!(!match_wildcard("rm ?", "rm xy"));
        assert!(match_wildcard("*.env", ".env"));
        assert!(!match_wildcard("*.env", ".env.example"));
    }

    #[test]
    fn granular_rules_use_last_match_wins() {
        let rules = ruleset(vec![(
            "bash",
            PermissionRule::Granular(vec![
                ("*".into(), PermissionAction::Ask),
                ("git *".into(), PermissionAction::Allow),
                ("git push *".into(), PermissionAction::Deny),
            ]),
        )]);
        let run = |cmd: &str| {
            decide(
                &rules,
                AgentKind::Build,
                "bash",
                &serde_json::json!({ "command": cmd }),
            )
        };
        assert_eq!(run("git status"), PermissionDecision::Allow);
        assert!(matches!(run("rm -rf /"), PermissionDecision::Ask { .. }));
        assert!(matches!(
            run("git push origin main"),
            PermissionDecision::Deny { .. }
        ));
        // Bare-arg patterns match the command itself, opencode-style: the
        // tool prefix must not be required.
        assert!(match_wildcard("git *", "git status"));
        assert!(!match_wildcard("git *", "bash git status"));
        assert_eq!(
            salient_arg(&serde_json::json!({ "command": "git status" })),
            "git status"
        );
        assert_eq!(salient_arg(&serde_json::json!({})), "");
    }

    #[test]
    fn plan_persona_denies_mutation_whatever_the_rules() {
        let mut rules = PermissionRuleset::default();
        rules.rules.insert(
            "edit".into(),
            PermissionRule::Simple(PermissionAction::Allow),
        );
        let d = decide(
            &rules,
            AgentKind::Plan,
            "edit",
            &serde_json::json!({ "path": "a.rs" }),
        );
        assert!(matches!(d, PermissionDecision::Deny { .. }));
        // …but reads still follow the rules (default ask here).
        assert!(matches!(
            decide(&rules, AgentKind::Plan, "read", &serde_json::json!({})),
            PermissionDecision::Ask { .. }
        ));
    }
}

#[cfg(test)]
mod remember_tests {
    use super::*;

    #[test]
    fn remember_allow_makes_future_calls_pass() {
        let mut rules = PermissionRuleset::default();
        assert!(matches!(
            decide(&rules, AgentKind::Build, "bash", &serde_json::json!({})),
            PermissionDecision::Ask { .. }
        ));
        remember_allow(&mut rules, "bash");
        assert_eq!(
            decide(&rules, AgentKind::Build, "bash", &serde_json::json!({})),
            PermissionDecision::Allow
        );
    }
}

#[cfg(test)]
mod glob_tests {
    use super::*;

    #[test]
    fn glob_stars_segments_and_doublestars_cross() {
        assert!(match_glob("*.rs", "main.rs"));
        assert!(!match_glob("*.rs", "src/main.rs"));
        assert!(match_glob("src/*.rs", "src/main.rs"));
        assert!(match_glob("src/**/*.rs", "src/a/b/main.rs"));
        assert!(match_glob("src/**/*.rs", "src/main.rs"));
        assert!(match_glob("**/Cargo.toml", "crates/agent/Cargo.toml"));
        assert!(match_glob("**/Cargo.toml", "Cargo.toml"));
        assert!(!match_glob("**/Cargo.toml", "Cargo.lock"));
        assert!(match_glob("**", "anything/at/all.txt"));
        assert!(match_glob("", "anything.txt"));
        assert!(match_glob("src/**", "src/a/b/c"));
        assert!(!match_glob("src/**", "other/a"));
    }

    #[test]
    fn grep_patterns_prefer_regex_then_fall_back() {
        let re = GrepPattern::compile("fn\\s+\\w+");
        assert!(re.is_regex());
        assert!(re.is_match("fn alpha() {}"));
        assert!(!re.is_match("struct Beta;"));

        // Invalid regex is a literal, never an error.
        let lit = GrepPattern::compile("a(b");
        assert!(!lit.is_regex());
        assert!(lit.is_match("has a(b inside"));
        assert!(!lit.is_match("has ab inside"));
    }
}
