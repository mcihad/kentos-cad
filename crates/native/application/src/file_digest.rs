//! A bound file's SHA-256 known again without reading it whole (plan/gis.md CBS-42, docs/adr/0597): a bound
//! GeoPackage's revision is its file's SHA-256, asked by every read, write, area change, state and question; a ten
//! million row file (3.6 GB) took seconds each time. The digest is kept with the file's stamp (device, inode, size,
//! modification and change times) and given again while the stamp is the same.
//!
//! A write within the clock's tick after a read could leave the same stamp (git's “racy” entries): a digest is kept only
//! when the file did not change while it was read and its times were at least [`QUIET`] older than the read's start.
//! A file written moments ago is read whole every time until it has been quiet for that long.

use std::collections::HashMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

/// How long a file's times must be behind a read for its digest to be kept.
pub const QUIET: Duration = Duration::from_secs(2);

/// The files kept at most; the oldest kept goes first.
const MOST: usize = 256;

/// What says a file is the one read: unchanged while all of it is the same.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Stamp {
    device: u64,
    inode: u64,
    len: u64,
    modified: Option<SystemTime>,
    /// The inode's change time (seconds, nanoseconds); a program cannot set it back.
    changed: (i64, i64),
}

impl Stamp {
    fn of(path: &Path) -> std::io::Result<Self> {
        let m = std::fs::metadata(path)?;
        #[cfg(unix)]
        let (device, inode, changed) = {
            use std::os::unix::fs::MetadataExt;
            (m.dev(), m.ino(), (m.ctime(), m.ctime_nsec()))
        };
        #[cfg(not(unix))]
        let (device, inode, changed) = (0, 0, (0, 0));
        Ok(Stamp {
            device,
            inode,
            len: m.len(),
            modified: m.modified().ok(),
            changed,
        })
    }

    /// Whether the file's times are at least `quiet` before `start`.
    fn quiet_before(&self, start: SystemTime, quiet: Duration) -> bool {
        let Some(limit) = start.checked_sub(quiet) else {
            return false;
        };
        let changed = SystemTime::UNIX_EPOCH.checked_add(Duration::new(
            u64::try_from(self.changed.0).unwrap_or(0),
            u32::try_from(self.changed.1).unwrap_or(0),
        ));
        self.modified.is_some_and(|m| m <= limit)
            && (cfg!(not(unix)) || changed.is_some_and(|c| c <= limit))
    }
}

struct Known {
    stamp: Stamp,
    sha256: String,
    /// When it was kept: the oldest goes when the list is full.
    order: u64,
}

static KNOWN: Mutex<Option<HashMap<PathBuf, Known>>> = Mutex::new(None);
static NEXT: AtomicU64 = AtomicU64::new(0);

thread_local! {
    /// The files this thread read whole (a test's count).
    static READS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// The file's size and SHA-256 (64 hex digits): the one kept when its stamp is the same, else read a piece at a time.
pub fn sha256(path: &Path) -> std::io::Result<(u64, String)> {
    sha256_with(path, QUIET)
}

/// [`sha256`] with another quiet time (a test's).
pub(crate) fn sha256_with(path: &Path, quiet: Duration) -> std::io::Result<(u64, String)> {
    let key = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let before = Stamp::of(&key)?;
    if let Ok(known) = KNOWN.lock()
        && let Some(k) = known.as_ref().and_then(|m| m.get(&key))
        && k.stamp == before
    {
        return Ok((before.len, k.sha256.clone()));
    }
    let start = SystemTime::now();
    READS.with(|r| r.set(r.get() + 1));
    let (len, sha256) =
        kentos_commands::encoding::sha256_reader_hex(BufReader::new(std::fs::File::open(&key)?))?;
    let after = Stamp::of(&key)?;
    if after == before
        && after.len == len
        && after.quiet_before(start, quiet)
        && let Ok(mut known) = KNOWN.lock()
    {
        let map = known.get_or_insert_with(HashMap::new);
        if map.len() >= MOST
            && let Some(oldest) = map
                .iter()
                .min_by_key(|(_, k)| k.order)
                .map(|(p, _)| p.clone())
        {
            map.remove(&oldest);
        }
        map.insert(
            key,
            Known {
                stamp: after,
                sha256: sha256.clone(),
                order: NEXT.fetch_add(1, Ordering::Relaxed),
            },
        );
    }
    Ok((len, sha256))
}

/// How many files this thread read whole so far (a test's count).
#[cfg(test)]
fn reads() -> u64 {
    READS.with(std::cell::Cell::get)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, bytes: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kentos-digest-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");
        let p = dir.join("veri.gpkg");
        std::fs::write(&p, bytes).expect("written");
        p
    }

    fn sha(bytes: &[u8]) -> String {
        kentos_commands::encoding::sha256_hex(bytes)
    }

    #[test]
    fn a_quiet_file_is_read_once_and_a_changed_one_again() {
        let p = file("sessiz", b"birinci");
        let first = sha256_with(&p, Duration::ZERO).expect("read");
        assert_eq!(first, (7, sha(b"birinci")));
        let read = reads();
        assert_eq!(sha256_with(&p, Duration::ZERO).expect("kept"), first);
        assert_eq!(reads(), read, "read again");
        std::fs::write(&p, b"ikincisi uzun").expect("written");
        assert_eq!(
            sha256_with(&p, Duration::ZERO).expect("read").1,
            sha(b"ikincisi uzun")
        );
    }

    /// docs/adr/0636 §6: a part's band read again (a headless report asked twice, a book of pages) does not read the
    /// big file whole again: the binding's version check takes the kept digest.
    #[test]
    fn a_band_read_twice_reads_the_file_whole_once() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/geopackage/v1/komsu.gpkg");
        let p = file("bant", &std::fs::read(&src).expect("the fixture"));
        let setup = serde_json::json!({
            "format": "kentos.document", "version": 1, "name": "Bant",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2",
                         "angleUnit": "grad", "plotScale": 1000, "workspace": "gis"},
            "origin": {"x": 0.0, "y": 0.0},
            "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false,
                        "expanded": true, "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25},
                        "children": []}],
            "activeLayer": "cizim", "entities": [], "styles": {"items": [], "categories": []}
        });
        let mut doc = kentos_domain::Document::from_snapshot(
            kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup"),
        )
        .expect("setup");
        let linked = crate::registry::run(
            &mut crate::ExecutionContext::new(&mut doc),
            "cad.sources.edit",
            kentos_commands::CallOp::Execute,
            serde_json::json!({"edit": {"kind": "link", "path": p.display().to_string(), "table": "parsel",
                                        "extent": [486_990.0, 4_419_990.0, 487_009.0, 4_420_030.0]}}),
        )
        .expect("runs");
        assert_eq!(linked["status"], "completed", "{linked}");
        let link = doc.settings().sources[0].clone();
        // A file a drawing binds was written a while ago: past the quiet time (its change time cannot be set back).
        std::thread::sleep(QUIET + Duration::from_millis(100));
        let first = crate::sources::band_objects(&doc, &link).expect("a part");
        assert_eq!(first.1.as_ref().map(Vec::len), Ok(2), "B and C");
        let before = reads();
        for _ in 0..3 {
            let again = crate::sources::band_objects(&doc, &link).expect("a part");
            assert_eq!(again.1.as_ref().map(Vec::len), Ok(2));
        }
        assert_eq!(reads(), before, "the band read the file whole again");
    }

    #[test]
    fn a_file_written_moments_ago_is_not_kept() {
        let p = file("yeni", b"yeni yazildi");
        let before = reads();
        let a = sha256(&p).expect("read");
        let b = sha256(&p).expect("read again");
        assert_eq!(a, b);
        assert_eq!(reads(), before + 2, "a fresh file's digest was kept");
    }
}
