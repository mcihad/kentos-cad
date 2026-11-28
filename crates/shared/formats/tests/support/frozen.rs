//! A written file held to its fingerprint, and the way to write a new one (docs/adr/9403 §6).
//!
//! A fingerprint says that a file changed, not how. Before a new one is written into a test, the file is read
//! against the one the last fingerprint was taken of: `KENTOS_WRITE_FROZEN=<folder>` makes the test put its file
//! there, once from the code before the change and once after, and the two are compared line by line.

/// A file's length and FNV-1a (64 bits) over its bytes.
pub fn print(bytes: &[u8]) -> (usize, u64) {
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    (bytes.len(), hash)
}

/// Whether the file was put aside instead of being checked: under `KENTOS_WRITE_FROZEN` it is written to that folder
/// as `name` and its fingerprint is said.
pub fn kept(name: &str, bytes: &[u8]) -> bool {
    let Some(dir) = std::env::var_os("KENTOS_WRITE_FROZEN") else {
        return false;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).expect("the folder");
    std::fs::write(dir.join(name), bytes).expect("the file");
    let (length, hash) = print(bytes);
    eprintln!("{name}: {length} bytes, fingerprint {hash:#018x}");
    true
}
