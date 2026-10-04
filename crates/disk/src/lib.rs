//! Cybermanju Drive — `.cybermanju` disk & volume substrate (AGENT-6).
//!
//! The decentralized-OS storage object: a provider-backed virtual disk with an
//! adjustable, choosable capacity, a superblock that is sealed and
//! content-addressed, a block allocator with refcounts, and a volume manager
//! that merges N attached disks into one logical address space.
//!
//! AGENT-6: build the modules here — `superblock`, `disk`, `allocator`,
//! `volume`, `mount`. Declare each with `pub mod` as you create the file.
//! This crate deliberately does **not** depend on `cybermanju-web` or
//! `cybermanju-os`; the dependency arrow points *into* it.

/// On-disk container format version for `.cybermanju` superblocks.
pub const FORMAT_VERSION: u32 = 1;

/// Magic prefix of a `.cybermanju` superblock ("CYBMJU1").
pub const DISK_MAGIC: &[u8; 7] = b"CYBMJU1";
