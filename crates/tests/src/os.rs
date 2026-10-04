// Cybermanju Drive — AGENT-8 OS layer tests (cybsh, tasks, compute)
//
// Pre-created and registered by the supervisor. Acceptance lives in
// `scripts/os-acceptance.sh` Tier 2; unit/contract cases belong here.

#[test]
fn shell_prompt_is_stable() {
    assert_eq!(cybermanju_os::PROMPT, "cybsh> ");
}
