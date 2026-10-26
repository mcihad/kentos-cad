//! A backup the server can be restored from (docs/adr/0650): the database and the objects it refers to, taken at one
//! moment, with a record of both. The objects of the store never change once committed (docs/adr/0031), so the rows a
//! snapshot of the database sees name objects that are there; the backup reads the rows in a read-only snapshot,
//! hands the same snapshot to `pg_dump` and copies each object those rows name, checking its SHA-256 on the way.
//! A restored server is checked with [`check`]: every object its rows name is in its store with that size and
//! SHA-256. An object the store has and no row names is said, never removed.

use std::collections::BTreeSet;
use std::io::{self, Read, Write};
use std::path::Path;

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

/// Takes a backup of the database at `owner_url` (the owner role's address) and of the objects in `blobs` into
/// `dir`, which must not exist or be empty. `pg_dump` is run with the snapshot the objects are read in.
pub async fn take(owner_url: &str, blobs: &Blobs, dir: &Path) -> Result<Manifest, String> {
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
    // The objects, while the snapshot is held: copied and checked against their rows.
    let root = blobs.root().to_path_buf();
    let into = dir.join(OBJECTS);
    let copied = {
        let objects = objects.clone();
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let from = Blobs::new(&root);
            let to = Blobs::new(&into);
            for n in &objects {
                let src = from.local_path(&n.key).map_err(|e| e.to_string())?;
                let dst = to.local_path(&n.key).map_err(|e| e.to_string())?;
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
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    };
    tx.rollback().await.map_err(|e| e.to_string())?;
    pool.close().await;
    copied?;
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
    Ok(manifest)
}

fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}
