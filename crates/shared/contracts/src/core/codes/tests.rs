//! The codes' tests (moved out of codes.rs, which is over the size limit, CLAUDE.md §8).

use super::*;
use std::collections::HashSet;

#[test]
fn every_code_is_once_and_plain() {
    let mut seen = HashSet::new();
    for c in CODES {
        assert!(seen.insert(c.code), "{} twice", c.code);
        assert!(
            c.code.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
            "{}",
            c.code
        );
        assert!(!c.meaning.is_empty(), "{}", c.code);
        assert_eq!(
            c.retryable,
            c.kind == CodeKind::Conflict,
            "{}: only a conflict is retryable",
            c.code
        );
    }
}

#[test]
fn a_code_is_found_by_name() {
    assert_eq!(info(STALE_STATE).map(|c| c.kind), Some(CodeKind::Conflict));
    assert!(retryable(REVISION_CONFLICT));
    assert!(!retryable(LAYER_LOCKED));
    assert!(info("nope").is_none());
}
