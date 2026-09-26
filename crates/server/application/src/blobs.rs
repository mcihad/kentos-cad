//! The object store of file projects (docs/adr/0031, ADR 0012: content in
//! an object store, catalog and permissions in PostgreSQL). This first store
//! is a folder of the server (`KENTOS_BLOB_DIR`); an S3-compatible store can
//! take its place behind the same calls later (TODOS.md SYNC-03).
//!
//! - An upload is written to `uploads/{tenant}/{project}/{upload}`, hashed
//!   while it arrives and cut off past its declared size. It is temporary:
//!   a file there older than two upload lifetimes belongs to no one and is
//!   removed whatever the database says.
//! - A committed revision is renamed to
//!   `revisions/{tenant}/{project}/{revision}-{sha256}` and never changes;
//!   the folder goes when the project is removed for good.
//! - A database project's checkpoint is written to
//!   `checkpoints/{tenant}/{project}/{checkpoint}-{sha256}` (docs/adr/0034)
//!   and never changes either.
//!
//! Keys are built here from ids and numbers only, so a key never leaves the
//! store's folder. Writes are flushed to disk before they are reported.

use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

/// The folder the store keeps its objects in.
#[derive(Clone, Debug)]
pub struct Blobs {
    root: PathBuf,
}

/// Why an upload's bytes were not kept.
#[derive(Debug)]
pub enum WriteError {
    /// More bytes came than the upload declared.
    TooLarge {
        declared: u64,
    },
    /// The request's body broke off (the client went away, a proxy cut it).
    Interrupted(String),
    Io(io::Error),
}

impl From<io::Error> for WriteError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl Blobs {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn upload_key(tenant: Uuid, project: Uuid, upload: Uuid) -> String {
        format!("uploads/{tenant}/{project}/{upload}")
    }

    pub fn revision_key(tenant: Uuid, project: Uuid, revision: i64, sha256: &str) -> String {
        format!("revisions/{tenant}/{project}/{revision:010}-{sha256}")
    }

    /// A database project's checkpoint (docs/adr/0034): its snapshot's object.
    pub fn checkpoint_key(tenant: Uuid, project: Uuid, checkpoint: Uuid, sha256: &str) -> String {
        format!("checkpoints/{tenant}/{project}/{checkpoint}-{sha256}")
    }

    fn path(&self, key: &str) -> io::Result<PathBuf> {
        // Keys are made above from ids, numbers and hex; anything else is a bug, never a path.
        let safe = !key.is_empty()
            && key.split('/').all(|part| {
                !part.is_empty()
                    && part != "."
                    && part != ".."
                    && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            });
        if !safe {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("geçersiz nesne anahtarı: {key}"),
            ));
        }
        Ok(self.root.join(key))
    }

    /// Starts writing an object; at most `declared` bytes are accepted.
    pub async fn create(&self, key: &str, declared: u64) -> io::Result<BlobWriter> {
        let path = self.path(key)?;
        if let Some(dir) = path.parent() {
            tokio::fs::create_dir_all(dir).await?;
        }
        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .await?;
        Ok(BlobWriter {
            file,
            path,
            hasher: Sha256::new(),
            written: 0,
            declared,
        })
    }

    /// Reads an object whole.
    pub async fn read(&self, key: &str) -> io::Result<Vec<u8>> {
        tokio::fs::read(self.path(key)?).await
    }

    /// Opens an object for streaming it out.
    pub async fn open(&self, key: &str) -> io::Result<tokio::fs::File> {
        tokio::fs::File::open(self.path(key)?).await
    }

    /// Gives an object its final key: a rename in the same folder tree, then
    /// the new folder flushed, so the name survives a crash. An object
    /// already at its new key (moved by a commit that did not finish; a
    /// revision's key is its content's hash) is promoted already; one at
    /// neither key is `NotFound`.
    pub async fn promote(&self, from: &str, to: &str) -> io::Result<()> {
        let (from, to) = (self.path(from)?, self.path(to)?);
        if let Some(dir) = to.parent() {
            tokio::fs::create_dir_all(dir).await?;
        }
        if let Err(e) = tokio::fs::rename(&from, &to).await
            && (e.kind() != io::ErrorKind::NotFound || !tokio::fs::try_exists(&to).await?)
        {
            return Err(e);
        }
        if let Some(dir) = to.parent() {
            let dir = dir.to_path_buf();
            tokio::task::spawn_blocking(move || std::fs::File::open(dir)?.sync_all())
                .await
                .map_err(io::Error::other)??;
        }
        Ok(())
    }

    /// Gives an object a second key (a copied project's revision): the same
    /// bytes under both, and neither ever changes. A hard link where the
    /// file system allows it, else a copy written beside and renamed into
    /// place; the new folder is flushed either way. An object already at
    /// `to` is kept: a revision's key carries its content's hash.
    pub async fn share(&self, from: &str, to: &str) -> io::Result<()> {
        let (from, to) = (self.path(from)?, self.path(to)?);
        let Some(dir) = to.parent().map(Path::to_path_buf) else {
            return Err(io::Error::other("nesne anahtarının klasörü yok"));
        };
        tokio::fs::create_dir_all(&dir).await?;
        match tokio::fs::hard_link(&from, &to).await {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(_) => {
                let part = to.with_extension("part");
                tokio::fs::copy(&from, &part).await?;
                tokio::fs::File::open(&part).await?.sync_all().await?;
                tokio::fs::rename(&part, &to).await?;
            }
        }
        tokio::task::spawn_blocking(move || std::fs::File::open(dir)?.sync_all())
            .await
            .map_err(io::Error::other)??;
        Ok(())
    }

    /// Removes an object; one that is not there is already removed.
    pub async fn remove(&self, key: &str) -> io::Result<()> {
        match tokio::fs::remove_file(self.path(key)?).await {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    /// Removes a project's revision objects numbered above `newest`, except
    /// `keep`: nothing committed them, a commit moved them and did not
    /// finish. The caller holds the project's lock, so no commit of it is
    /// under way. Returns how many went.
    pub async fn remove_revisions_after(
        &self,
        tenant: Uuid,
        project: Uuid,
        newest: i64,
        keep: &str,
    ) -> io::Result<usize> {
        let dir = self
            .root
            .join("revisions")
            .join(tenant.to_string())
            .join(project.to_string());
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
            entries => entries?,
        };
        let keep = self.path(keep)?;
        let mut removed = 0;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let number = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.split_once('-'))
                .and_then(|(n, _)| n.parse::<i64>().ok());
            if number.is_some_and(|n| n > newest) && path != keep {
                match tokio::fs::remove_file(&path).await {
                    Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                    _ => removed += 1,
                }
            }
        }
        Ok(removed)
    }

    /// Removes everything of a project removed for good.
    pub async fn remove_project(&self, tenant: Uuid, project: Uuid) -> io::Result<()> {
        for kind in ["uploads", "revisions", "checkpoints"] {
            let dir = self
                .root
                .join(kind)
                .join(tenant.to_string())
                .join(project.to_string());
            match tokio::fs::remove_dir_all(&dir).await {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                _ => {}
            }
        }
        Ok(())
    }

    /// The projects that have revisions or checkpoints in the store, as
    /// (tenant, project), whose folder has not changed for `settled`: a
    /// copy's folder is made just before its project's row is committed
    /// (docs/adr/0031), so a fresh folder may belong to a project that exists
    /// a moment later.
    pub async fn projects(&self, settled: Duration) -> io::Result<Vec<(Uuid, Uuid)>> {
        let mut found = Vec::new();
        let now = SystemTime::now();
        for kind in ["revisions", "checkpoints"] {
            let Ok(mut tenants) = tokio::fs::read_dir(self.root.join(kind)).await else {
                continue;
            };
            while let Some(t) = tenants.next_entry().await? {
                let Some(tenant) = uuid_name(&t.path()) else {
                    continue;
                };
                let mut projects = tokio::fs::read_dir(t.path()).await?;
                while let Some(p) = projects.next_entry().await? {
                    let still = age(&p.metadata().await?, now).is_some_and(|d| d >= settled);
                    if let (Some(project), true) = (uuid_name(&p.path()), still)
                        && !found.contains(&(tenant, project))
                    {
                        found.push((tenant, project));
                    }
                }
            }
        }
        Ok(found)
    }

    /// The checkpoint objects unchanged for `settled`, as (checkpoint id,
    /// key): a snapshot's object is written a moment before its row is
    /// committed, and one whose row never was belongs to nothing.
    pub async fn checkpoint_objects(&self, settled: Duration) -> io::Result<Vec<(Uuid, String)>> {
        let mut found = Vec::new();
        let now = SystemTime::now();
        let Ok(mut tenants) = tokio::fs::read_dir(self.root.join("checkpoints")).await else {
            return Ok(found);
        };
        while let Some(t) = tenants.next_entry().await? {
            let Some(tenant) = uuid_name(&t.path()) else {
                continue;
            };
            let mut projects = tokio::fs::read_dir(t.path()).await?;
            while let Some(p) = projects.next_entry().await? {
                let Some(project) = uuid_name(&p.path()) else {
                    continue;
                };
                let mut objects = tokio::fs::read_dir(p.path()).await?;
                while let Some(o) = objects.next_entry().await? {
                    let name = o.file_name().to_string_lossy().into_owned();
                    let id = name
                        .split_once('-')
                        .and_then(|_| name.get(..36))
                        .and_then(|id| Uuid::parse_str(id).ok());
                    let still = age(&o.metadata().await?, now).is_some_and(|d| d >= settled);
                    if let (Some(id), true) = (id, still) {
                        found.push((id, format!("checkpoints/{tenant}/{project}/{name}")));
                    }
                }
            }
        }
        Ok(found)
    }

    /// Removes upload files older than `age`: an upload lives
    /// `UPLOAD_LIFETIME_HOURS` (its row goes then), so a file much older than
    /// that is no one's. Returns how many went.
    pub async fn sweep_uploads(&self, age: Duration) -> io::Result<usize> {
        let root = self.root.join("uploads");
        tokio::task::spawn_blocking(move || sweep(&root, age))
            .await
            .map_err(io::Error::other)?
    }
}

/// How long ago something last changed (none when the clock or the file system cannot say).
fn age(meta: &std::fs::Metadata, now: SystemTime) -> Option<Duration> {
    meta.modified()
        .ok()
        .and_then(|m| now.duration_since(m).ok())
}

fn uuid_name(path: &Path) -> Option<Uuid> {
    path.file_name()?
        .to_str()
        .and_then(|n| Uuid::parse_str(n).ok())
}

fn sweep(dir: &Path, age: Duration) -> io::Result<usize> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(0);
    };
    let now = SystemTime::now();
    let mut removed = 0;
    for entry in entries {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_dir() {
            removed += sweep(&entry.path(), age)?;
            // An emptied folder goes too; one still in use stays (removing it fails).
            let _ = std::fs::remove_dir(entry.path());
        } else if meta
            .modified()
            .ok()
            .and_then(|m| now.duration_since(m).ok())
            .is_some_and(|d| d >= age)
        {
            std::fs::remove_file(entry.path())?;
            removed += 1;
        }
    }
    Ok(removed)
}

/// An object being written: hashed as it goes, cut off past its declared size.
pub struct BlobWriter {
    file: tokio::fs::File,
    path: PathBuf,
    hasher: Sha256,
    written: u64,
    declared: u64,
}

impl BlobWriter {
    pub async fn write(&mut self, chunk: &[u8]) -> Result<(), WriteError> {
        self.written += chunk.len() as u64;
        if self.written > self.declared {
            return Err(WriteError::TooLarge {
                declared: self.declared,
            });
        }
        self.hasher.update(chunk);
        self.file.write_all(chunk).await?;
        Ok(())
    }

    /// Flushes the bytes to disk; returns how many there were and their SHA-256 (hex).
    pub async fn finish(mut self) -> io::Result<(u64, String)> {
        self.file.flush().await?;
        self.file.sync_all().await?;
        let digest = self.hasher.finalize();
        let hex = digest.iter().map(|b| format!("{b:02x}")).collect();
        Ok((self.written, hex))
    }

    /// Where the object is, for removing a failed one.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (Blobs, PathBuf) {
        let dir = std::env::temp_dir().join(format!("kentos-blobs-{}", Uuid::now_v7()));
        (Blobs::new(&dir), dir)
    }

    #[tokio::test]
    async fn an_object_is_hashed_cut_off_promoted_and_removed() {
        let (blobs, dir) = store();
        let (t, p, u) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
        let key = Blobs::upload_key(t, p, u);
        let mut w = blobs.create(&key, 3).await.unwrap();
        w.write(b"abc").await.unwrap();
        let (size, sha) = w.finish().await.unwrap();
        assert_eq!(
            (size, sha.as_str()),
            (
                3,
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            )
        );
        let mut w = blobs.create(&key, 2).await.unwrap();
        assert!(matches!(
            w.write(b"abc").await,
            Err(WriteError::TooLarge { declared: 2 })
        ));
        let final_key = Blobs::revision_key(t, p, 1, &sha);
        let mut w = blobs.create(&key, 3).await.unwrap();
        w.write(b"abc").await.unwrap();
        w.finish().await.unwrap();
        blobs.promote(&key, &final_key).await.unwrap();
        assert_eq!(blobs.read(&final_key).await.unwrap(), b"abc");
        // Again (a commit that moved it and did not finish): already there. From neither key: missing.
        blobs.promote(&key, &final_key).await.unwrap();
        let nowhere = Blobs::revision_key(t, p, 2, &sha);
        assert_eq!(
            blobs.promote(&key, &nowhere).await.unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(blobs.projects(Duration::ZERO).await.unwrap(), vec![(t, p)]);
        // A folder that changed within the hour is left alone.
        assert!(
            blobs
                .projects(Duration::from_secs(3600))
                .await
                .unwrap()
                .is_empty()
        );
        // Objects above the newest committed revision go, but for the one kept.
        for (n, content) in [(2, "b"), (3, "c")] {
            let mut w = blobs
                .create(&Blobs::revision_key(t, p, n, content), 1)
                .await
                .unwrap();
            w.write(content.as_bytes()).await.unwrap();
            w.finish().await.unwrap();
        }
        let kept = Blobs::revision_key(t, p, 3, "c");
        assert_eq!(
            blobs.remove_revisions_after(t, p, 1, &kept).await.unwrap(),
            1
        );
        assert_eq!(blobs.read(&final_key).await.unwrap(), b"abc");
        assert_eq!(blobs.read(&kept).await.unwrap(), b"c");
        assert!(
            blobs
                .read(&Blobs::revision_key(t, p, 2, "b"))
                .await
                .is_err()
        );
        blobs.remove_project(t, p).await.unwrap();
        assert!(blobs.read(&final_key).await.is_err());
        blobs.remove(&final_key).await.unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn keys_never_leave_the_folder() {
        let (blobs, _) = store();
        for bad in ["../x", "uploads/../../etc", "", "a//b", "a/./b", "a b"] {
            assert!(blobs.read(bad).await.is_err(), "{bad}");
            assert!(blobs.create(bad, 1).await.is_err(), "{bad}");
        }
    }

    #[tokio::test]
    async fn old_upload_files_are_swept() {
        let (blobs, dir) = store();
        let key = Blobs::upload_key(Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
        let mut w = blobs.create(&key, 1).await.unwrap();
        w.write(b"x").await.unwrap();
        w.finish().await.unwrap();
        assert_eq!(
            blobs
                .sweep_uploads(Duration::from_secs(3600))
                .await
                .unwrap(),
            0
        );
        assert_eq!(blobs.sweep_uploads(Duration::ZERO).await.unwrap(), 1);
        assert!(blobs.read(&key).await.is_err());
        let _ = std::fs::remove_dir_all(dir);
    }
}
