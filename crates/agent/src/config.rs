// Permission evaluation: opencode/omp-compatible rulesets.
//
// Shape: `{"default": "ask", "bash": "allow", "edit": {"*": "deny", …}}`.
// Granular maps evaluate last-match-wins (catch-all first, specifics after);
// `*` spans any run, `?` exactly one char. Plan-kind sessions deny
// edit/write/bash unconditionally. `deny` always beats `auto_approve`.

use cybermanju_types::agent::{AgentKind, PermissionAction, PermissionRuleset};

/// What the loop should do with a proposed tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    /// Pause the job and ask the user (carries a human summary).
    Ask { summary: String },
    /// Refuse with an `unsupported:`-style machine prefix.
    Deny { reason: String },
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

/// The match string a tool call is evaluated against: `name` plus its most
/// salient argument, mirroring how `grep <pattern>` needs `"grep *"` while
/// bare `"grep"` only matches the argless call.
pub fn match_input(tool: &str, input: &serde_json::Value) -> String {
    let arg = input
        .get("command")
        .or_else(|| input.get("pattern"))
        .or_else(|| input.get("path"))
        .or_else(|| input.get("glob"))
        .or_else(|| input.get("query"))
        .or_else(|| input.get("url"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if arg.is_empty() {
        tool.to_string()
    } else {
        format!("{tool} {arg}")
    }
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
            let mut hit = None;
            for (pattern, action) in pairs {
                if match_wildcard(pattern, &target) || match_wildcard(pattern, tool) {
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
