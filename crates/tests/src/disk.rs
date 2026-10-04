// Cybermanju Drive — AGENT-6 disk & volume tests
//
// Pre-created and registered by the supervisor so `cargo test -p cybermanju-tests`
// picks the module up as soon as AGENT-6 adds cases. Acceptance lives in
// `scripts/os-acceptance.sh` Tier 0; unit/contract cases belong here.

use cybermanju_disk::{DISK_MAGIC, FORMAT_VERSION};

#[test]
fn superblock_magic_and_format_version_are_stable() {
    // Pins the on-disk container: changing either is a breaking format change
    // and must bump MANIFEST/superblock compatibility handling.
    assert_eq!(DISK_MAGIC, b"CYBMJU1");
    assert_eq!(FORMAT_VERSION, 1);
}
