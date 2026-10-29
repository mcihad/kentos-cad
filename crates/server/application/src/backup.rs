//! A backup the server can be restored from (docs/adr/0650): the database and the objects it refers to, taken at one
//! moment, with a record of both. The objects of the store never change once committed (docs/adr/0031), so the rows a
//! snapshot of the database sees name objects that are there; the backup reads the rows in a read-only snapshot,
//! hands the same snapshot to `pg_dump` and copies each object those rows name, checking its SHA-256 on the way.
//! A restored server is checked with [`check`]: every object its rows name is in its store with that size and
//! SHA-256. An object the store has and no row names is said, never removed.
//!
//! Backups taken again and again (docs/adr/0739 §4) go into folders of one root, named by when they began
//! ([`take_into`]): an object the newest complete backup there has with the same key, size and SHA-256 is linked
//! from it (objects never change; it is read and checked all the same), every other one copied; then the oldest
//! complete backups beyond `keep` and the failed attempts before the newest complete one are removed ([`rotation`]).
//! A backup that fails removes its own folder and nothing else.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::blobs::Blobs;

/// The record a backup writes beside its files.
pub const MANIFEST: &str = "yedek.json";
/// The database's dump (`pg_dump --format=custom`).
pub const DUMP: &str = "veritabani.dump";
/// The folder the objects are copied into, by their keys.
pub const OBJECTS: &str = "nesneler";
/// The record's format.
pub const FORMAT: &str = "kentos.backup/1";

/// An object a row of the database names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Named {
    pub key: String,
    pub size: i64,
    pub sha256: String,
    /// The table naming it: `project_file_revision`, `project_checkpoint`, `server_job`, `job_input`.
    pub table: String,
}

/// A backup's record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub format: String,
    /// When the snapshot was taken (the database's clock, RFC 3339).
    pub taken: String,
    /// The database's name and the last migration it has.
    pub database: String,
    pub migration: i64,
    /// The dump's size and SHA-256.
    pub dump_size: u64,
    pub dump_sha256: String,
    /// The objects the snapshot's rows name, each copied.
    pub objects: Vec<Named>,
}

/// What a check found.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Checked {
    /// Objects the rows name and the store has as named.
    pub sound: usize,
    /// Objects a row names that the store has not.
    pub missing: Vec<Named>,
    /// Objects the store has with another size or SHA-256 than its row says.
    pub wrong: Vec<Named>,
    /// Objects the store has that no row names (a purged project's, an upload's, a backup taken mid-purge's).
    pub unnamed: Vec<String>,
    /// For a backup's folder: whether its dump is the one its record names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dump: Option<bool>,
}

impl Checked {
    /// Whether every object named is there as named (and the dump, for a backup).
    pub fn sound(&self) -> bool {
        self.missing.is_empty() && self.wrong.is_empty() && self.dump != Some(false)
    }
}

/// The objects the database's rows name: committed file revisions, checkpoints' snapshots, succeeded jobs' artifacts
/// and received job inputs. An upload not committed is temporary and not named. Read as the owner (row-level
/// security shows the server role one project at a time).
pub async fn named(owner: &mut sqlx::PgConnection) -> Result<Vec<Named>, sqlx::Error> {
    let rows: Vec<(String, i64, String, String)> = sqlx::query_as(
        "select blob_key, size, sha256, 'project_file_revision' from kentos.project_file_revision
         union all
         select blob_key, size, sha256, 'project_checkpoint' from kentos.project_checkpoint where blob_key is not null
         union all
         select artifact_key, artifact_size, artifact_sha256, 'server_job' from kentos.server_job
          where artifact_key is not null
         union all
         select blob_key, size, sha256, 'job_input' from kentos.job_input where received_at is not null
         order by 1",
    )
    .fetch_all(owner)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(key, size, sha256, table)| Named {
            key,
            size,
            sha256,
            table,
        })
        .collect())
}

fn hex(d: impl AsRef<[u8]>) -> String {
    d.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// A file's size and SHA-256, read in parts; with `copy`, written there too (flushed to disk).
fn digest(from: &Path, copy: Option<&Path>) -> io::Result<(u64, String)> {
    let mut file = std::fs::File::open(from)?;
    let mut out = match copy {
        Some(to) => {
            if let Some(dir) = to.parent() {
                std::fs::create_dir_all(dir)?;
            }
            Some(std::fs::File::create(to)?)
        }
        None => None,
    };
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut size = 0u64;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        if let Some(o) = out.as_mut() {
            o.write_all(&buf[..n])?;
        }
        size += n as u64;
    }
    if let Some(o) = out {
        o.sync_all()?;
    }
    Ok((size, hex(hasher.finalize())))
}

/// The keys of the objects under a store's folder (`revisions/…`, `checkpoints/…`, `jobs/…`, `job-inputs/…`;
/// uploads are temporary and left out).
fn keys_under(root: &Path) -> io::Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    for top in ["revisions", "checkpoints", "jobs", "job-inputs"] {
        let mut stack = vec![root.join(top)];
        while let Some(dir) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e),
            };
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                if entry.file_type()?.is_dir() {
                    stack.push(path);
                } else if let Ok(rel) = path.strip_prefix(root) {
                    out.insert(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    }
    Ok(out)
}

/// Checks `named` against the objects under `root`: each there with its size and SHA-256 (read whole).
fn check_in(root: &Path, named: &[Named]) -> io::Result<Checked> {
    let blobs = Blobs::new(root);
    let mut out = Checked::default();
    let mut seen = BTreeSet::new();
    for n in named {
        seen.insert(n.key.clone());
        let path = blobs.local_path(&n.key)?;
        match digest(&path, None) {
            Ok((size, sha)) if size as i64 == n.size && sha == n.sha256 => out.sound += 1,
            Ok(_) => out.wrong.push(n.clone()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => out.missing.push(n.clone()),
            Err(e) => return Err(e),
        }
    }
    out.unnamed = keys_under(root)?
        .into_iter()
        .filter(|k| !seen.contains(k))
        .collect();
    Ok(out)
}

/// Checks a running (or restored) server: every object its rows name is in its store as named.
pub async fn check(owner: &PgPool, blobs: &Blobs) -> Result<Checked, String> {
    let mut conn = owner.acquire().await.map_err(|e| e.to_string())?;
    let named = named(&mut conn).await.map_err(|e| e.to_string())?;
    let root = blobs.root().to_path_buf();
    tokio::task::spawn_blocking(move || check_in(&root, &named))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("Nesne deposu okunamadı: {e}"))
}

/// Checks a backup's folder: its dump is the one its record names, each object it names is copied as named.
pub fn check_folder(dir: &Path) -> Result<Checked, String> {
    let manifest = read_manifest(dir)?;
    let (size, sha) =
        digest(&dir.join(DUMP), None).map_err(|e| format!("{DUMP} okunamadı: {e}"))?;
    let mut out = check_in(&dir.join(OBJECTS), &manifest.objects)
        .map_err(|e| format!("{OBJECTS} okunamadı: {e}"))?;
    out.dump = Some(size == manifest.dump_size && sha == manifest.dump_sha256);
    Ok(out)
}

/// A backup folder's record.
pub fn read_manifest(dir: &Path) -> Result<Manifest, String> {
    let text = std::fs::read_to_string(dir.join(MANIFEST))
        .map_err(|e| format!("{MANIFEST} okunamadı: {e}"))?;
    let m: Manifest =
        serde_json::from_str(&text).map_err(|e| format!("{MANIFEST} okunamadı: {e}"))?;
    if m.format != FORMAT {
        return Err(format!(
            "{MANIFEST} bu sürümün yedeği değil: biçimi “{}”, beklenen “{FORMAT}”.",
            m.format
        ));
    }
    Ok(m)
}

/// How the objects of a backup came into it: linked from an earlier backup or copied from the store.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Carried {
    pub linked: usize,
    pub copied: usize,
}

/// Takes a backup of the database at `owner_url` (the owner role's address) and of the objects in `blobs` into
/// `dir`, which must not exist or be empty. `pg_dump` is run with the snapshot the objects are read in.
pub async fn take(owner_url: &str, blobs: &Blobs, dir: &Path) -> Result<Manifest, String> {
    take_with(owner_url, blobs, dir, None).await.map(|(m, _)| m)
}

/// [`take`], an object an earlier backup's folder (`earlier`, with its record) has with the same key, size and SHA-256
/// linked from there instead of copied; each linked object is read and checked as a copied one is, and one that does
/// not read as its record says is copied from the store instead.
pub async fn take_with(
    owner_url: &str,
    blobs: &Blobs,
    dir: &Path,
    earlier: Option<(PathBuf, Manifest)>,
) -> Result<(Manifest, Carried), String> {
    if dir.exists()
        && std::fs::read_dir(dir)
            .map_err(|e| format!("{} okunamadı: {e}", dir.display()))?
            .next()
            .is_some()
    {
        return Err(format!(
            "{} boş değil: yedek boş ya da olmayan bir klasöre alınır.",
            dir.display()
        ));
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{} açılamadı: {e}", dir.display()))?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(owner_url)
        .await
        .map_err(|e| format!("Veritabanına bağlanılamadı: {e}"))?;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query("set transaction isolation level repeatable read, read only")
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    let (snapshot, taken, database, migration): (String, String, String, i64) = sqlx::query_as(
        "select pg_export_snapshot(),
                to_char(now() at time zone 'utc', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                current_database()::text,
                coalesce((select max(version) from _sqlx_migrations where success), 0)",
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let objects = named(&mut tx).await.map_err(|e| e.to_string())?;
    // The dump, with the same snapshot (held open by this transaction until it is done).
    let (bare, password) = kentos_postgres::env::split_password(owner_url);
    let dump = dir.join(DUMP);
    let status = {
        let dump = dump.clone();
        let snapshot = snapshot.clone();
        tokio::task::spawn_blocking(move || {
            let mut cmd = std::process::Command::new(
                std::env::var("KENTOS_PG_DUMP").unwrap_or_else(|_| "pg_dump".into()),
            );
            cmd.arg("--format=custom")
                .arg(format!("--snapshot={snapshot}"))
                .arg("--file")
                .arg(&dump)
                .arg("--dbname")
                .arg(&bare)
                .stdin(std::process::Stdio::null());
            if let Some(p) = &password {
                cmd.env("PGPASSWORD", p);
            }
            cmd.output()
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("pg_dump çalıştırılamadı: {e} (KENTOS_PG_DUMP ile yolunu verin)"))?
    };
    if !status.status.success() {
        return Err(format!(
            "pg_dump başarısız: {}",
            String::from_utf8_lossy(&status.stderr).trim()
        ));
    }
    // The objects, while the snapshot is held: linked or copied, and checked against their rows.
    let root = blobs.root().to_path_buf();
    let into = dir.join(OBJECTS);
    let copied = {
        let objects = objects.clone();
        tokio::task::spawn_blocking(move || -> Result<Carried, String> {
            let from = Blobs::new(&root);
            let to = Blobs::new(&into);
            let had: BTreeMap<&str, &Named> = earlier
                .as_ref()
                .map(|(_, m)| m.objects.iter().map(|n| (n.key.as_str(), n)).collect())
                .unwrap_or_default();
            let mut carried = Carried::default();
            for n in &objects {
                let dst = to.local_path(&n.key).map_err(|e| e.to_string())?;
                if let Some((folder, _)) = &earlier
                    && had
                        .get(n.key.as_str())
                        .is_some_and(|o| o.size == n.size && o.sha256 == n.sha256)
                    && linked(&folder.join(OBJECTS), &n.key, &dst, n)
                {
                    carried.linked += 1;
                    continue;
                }
                carried.copied += 1;
                let src = from.local_path(&n.key).map_err(|e| e.to_string())?;
                let (size, sha) = digest(&src, Some(&dst)).map_err(|e| {
                    if e.kind() == io::ErrorKind::NotFound {
                        format!(
                            "{} ({}) depoda yok: yedek alınırken silinmiş olabilir; yedeği yeniden alın, \
                             sürerse `kentosd backup check` ile deponun durumuna bakın.",
                            n.key, n.table
                        )
                    } else {
                        format!("{} kopyalanamadı: {e}", n.key)
                    }
                })?;
                if size as i64 != n.size || sha != n.sha256 {
                    return Err(format!(
                        "{} ({}) satırının söylediği değil: boyu {size}, SHA-256'sı {sha}.",
                        n.key, n.table
                    ));
                }
            }
            Ok(carried)
        })
        .await
        .map_err(|e| e.to_string())?
    };
    tx.rollback().await.map_err(|e| e.to_string())?;
    pool.close().await;
    let carried = copied?;
    let (dump_size, dump_sha256) =
        digest(&dump, None).map_err(|e| format!("{DUMP} okunamadı: {e}"))?;
    let manifest = Manifest {
        format: FORMAT.to_owned(),
        taken,
        database,
        migration,
        dump_size,
        dump_sha256,
        objects,
    };
    let text = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    write_synced(&dir.join(MANIFEST), text.as_bytes())
        .map_err(|e| format!("{MANIFEST} yazılamadı: {e}"))?;
    Ok((manifest, carried))
}

/// Links `key` from an earlier backup's objects (`folder`) to `dst` and checks it reads as `n` says; false (nothing
/// left at `dst`) when it is not there, cannot be linked (another file system) or does not read so.
fn linked(folder: &Path, key: &str, dst: &Path, n: &Named) -> bool {
    let Ok(src) = Blobs::new(folder).local_path(key) else {
        return false;
    };
    if let Some(dir) = dst.parent()
        && std::fs::create_dir_all(dir).is_err()
    {
        return false;
    }
    if std::fs::hard_link(&src, dst).is_err() {
        return false;
    }
    match digest(dst, None) {
        Ok((size, sha)) if size as i64 == n.size && sha == n.sha256 => true,
        _ => {
            let _ = std::fs::remove_file(dst);
            false
        }
    }
}

/// A backup folder's name under a root: when it began, UTC, in a form every file system takes
/// (`2026-10-29T12-04-05Z`), with `-2`, `-3` … after it for a second one in the same second.
pub fn folder_name(moment: time::OffsetDateTime, nth: u32) -> String {
    let m = moment.to_offset(time::UtcOffset::UTC);
    let base = format!(
        "{:04}-{:02}-{:02}T{:02}-{:02}-{:02}Z",
        m.year(),
        u8::from(m.month()),
        m.day(),
        m.hour(),
        m.minute(),
        m.second()
    );
    if nth <= 1 {
        base
    } else {
        format!("{base}-{nth}")
    }
}

/// A folder name's order (its moment, then its number); none for a name that is not a backup's.
pub fn folder_order(name: &str) -> Option<(String, u32)> {
    let base = name.get(..20)?;
    let shape = base.bytes().enumerate().all(|(i, b)| match i {
        4 | 7 | 13 | 16 => b == b'-',
        10 => b == b'T',
        19 => b == b'Z',
        _ => b.is_ascii_digit(),
    });
    if !shape {
        return None;
    }
    let nth = match &name[20..] {
        "" => 1,
        rest => {
            let digits = rest.strip_prefix('-')?;
            if digits.is_empty()
                || digits.starts_with('0')
                || !digits.bytes().all(|b| b.is_ascii_digit())
            {
                return None;
            }
            digits.parse::<u32>().ok().filter(|n| *n >= 2)?
        }
    };
    Some((base.to_owned(), nth))
}

/// A backup found under a root.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub name: String,
    pub path: PathBuf,
    /// Its record, when it reads: a complete backup. None: an attempt that failed or one under way.
    pub manifest: Option<Manifest>,
}

/// The backups under `root`, oldest first; folders not named as backups are not looked at.
pub fn found_in(root: &Path) -> Result<Vec<Found>, String> {
    let entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{} okunamadı: {e}", root.display())),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("{} okunamadı: {e}", root.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if folder_order(&name).is_none() || !entry.path().is_dir() {
            continue;
        }
        let path = entry.path();
        let manifest = read_manifest(&path).ok();
        found.push(Found {
            name,
            path,
            manifest,
        });
    }
    found.sort_by_key(|f| folder_order(&f.name));
    Ok(found)
}

/// The backups a rotation removes, of `found` (name and whether it is complete), when `keep` complete ones are kept:
/// the complete ones older than the newest `keep`, and the incomplete ones older than the newest complete one (an
/// attempt that failed; one newer than every complete backup may be under way and stays). Names that are not
/// backups' are never in it. `keep` is at least one.
pub fn rotation(found: &[(String, bool)], keep: usize) -> Vec<String> {
    let keep = keep.max(1);
    // Newest first: (order, name, complete).
    let mut named: Vec<_> = found
        .iter()
        .filter_map(|(name, complete)| folder_order(name).map(|order| (order, name, *complete)))
        .collect();
    named.sort_by(|a, b| b.0.cmp(&a.0));
    let newest_complete = named.iter().find(|n| n.2).map(|n| n.0.clone());
    let mut complete_seen = 0;
    let mut out = Vec::new();
    for (order, name, complete) in named {
        if complete {
            complete_seen += 1;
            if complete_seen > keep {
                out.push(name.clone());
            }
        } else if newest_complete.as_ref().is_some_and(|n| order < *n) {
            out.push(name.clone());
        }
    }
    out.sort_by_key(|n| folder_order(n));
    out
}

/// What [`take_into`] did.
#[derive(Clone, Debug, PartialEq)]
pub struct Rotated {
    pub name: String,
    pub manifest: Manifest,
    pub carried: Carried,
    /// The backups the rotation removed, oldest first.
    pub removed: Vec<String>,
}

/// Takes a backup into a new folder under `root` (named by now), its unchanged objects linked from the newest
/// complete backup there; then removes the backups the rotation says for `keep`. A backup that fails removes its own
/// folder and nothing else, and says why.
pub async fn take_into(
    owner_url: &str,
    blobs: &Blobs,
    root: &Path,
    keep: usize,
) -> Result<Rotated, String> {
    if keep == 0 {
        return Err("En az bir yedek saklanır: --keep 1 ya da daha büyük olmalı.".into());
    }
    std::fs::create_dir_all(root).map_err(|e| format!("{} açılamadı: {e}", root.display()))?;
    let before = found_in(root)?;
    let earlier = before
        .iter()
        .rev()
        .find_map(|f| f.manifest.clone().map(|m| (f.path.clone(), m)));
    let now = time::OffsetDateTime::now_utc();
    let mut nth = 1;
    let (name, dir) = loop {
        let name = folder_name(now, nth);
        let dir = root.join(&name);
        if !dir.exists() {
            break (name, dir);
        }
        nth += 1;
    };
    let (manifest, carried) = match take_with(owner_url, blobs, &dir, earlier).await {
        Ok(done) => done,
        Err(why) => {
            // Half a backup is none: its folder goes, so the disk it filled has room again.
            let _ = std::fs::remove_dir_all(&dir);
            return Err(why);
        }
    };
    let now_found: Vec<(String, bool)> = found_in(root)?
        .into_iter()
        .map(|f| (f.name, f.manifest.is_some()))
        .collect();
    let removed = rotation(&now_found, keep);
    for gone in &removed {
        std::fs::remove_dir_all(root.join(gone))
            .map_err(|e| format!("Eski yedek {gone} silinemedi: {e}"))?;
    }
    Ok(Rotated {
        name,
        manifest,
        carried,
        removed,
    })
}

fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}
