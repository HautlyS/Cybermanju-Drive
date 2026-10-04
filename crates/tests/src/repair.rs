// Cybermanju Drive — AGENT-7 durability & repair tests
//
// Pre-created and registered by the supervisor. Acceptance lives in
// `scripts/os-acceptance.sh` Tier 1; unit/contract cases belong here.

#[test]
fn default_parity_survives_one_provider_loss() {
    // The shipped default is `parity: 1` — a config written before RS landed
    // must still mean "tolerate one loss".
    assert_eq!(cybermanju_erasure::DEFAULT_PARITY, 1);
}
