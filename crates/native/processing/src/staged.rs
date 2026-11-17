//! A result's file written beside its place and put there when whole (docs/adr/0207 §7, docs/adr/8057 §120), the
//! headless server's and the desktop's alike. Its piece (`{name}.yaziliyor`) is not left behind: not by a run that
//! breaks or is stopped while writing (the sink dropped unfinished), not by a write that fails at its end, not by a file
//! that cannot be put in place (a folder of that name there; on Windows, a file another program holds open). Each host
//! wrote with its own copy, and both left the piece beside the result when the rename failed.

use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::files::{Sink, cog_file};

/// Where a file written at `path` is written first: beside it, `{name}.yaziliyor`.
pub fn piece_of(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map_or_else(|| "cikti".to_owned(), |n| n.to_string_lossy().into_owned());
    path.with_file_name(format!("{name}.yaziliyor"))
}

/// `staged` put at `path`; when it cannot be, `staged` is removed and why said.
pub fn put_in_place(staged: &Path, path: &Path) -> Result<(), String> {
    std::fs::rename(staged, path).map_err(|e| {
        let _ = std::fs::remove_file(staged);
        format!("“{}” yazılamadı: {e}.", path.display())
    })
}

/// A new file at `path` to write, its piece opened beside it.
pub fn create(path: &Path) -> Result<Staged, String> {
    let part = piece_of(path);
    let file = File::create(&part).map_err(|e| {
        format!(
            "“{}” yazılamadı: {e}. Klasörün yazılabilir olduğunu denetleyin.",
            path.display()
        )
    })?;
    Ok(Staged {
        path: path.to_path_buf(),
        part,
        file: Some(BufWriter::with_capacity(1 << 20, file)),
        placed: false,
    })
}

/// A file being written beside its place ([`create`]).
pub struct Staged {
    path: PathBuf,
    part: PathBuf,
    file: Option<BufWriter<File>>,
    /// Put in place: the piece is the file now.
    placed: bool,
}

impl Staged {
    fn fail(&self, e: std::io::Error) -> String {
        format!("“{}” yazılamadı: {e}.", self.path.display())
    }

    /// The piece written out and closed: Windows renames no open file.
    fn close(&mut self) -> Result<(), String> {
        let f = self.file.take().ok_or("Dosya kapalı.")?;
        let f = f.into_inner().map_err(|e| self.fail(e.into_error()))?;
        f.sync_all().map_err(|e| self.fail(e))
    }
}

impl Sink for Staged {
    fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        let path = self.path.display().to_string();
        let f = self.file.as_mut().ok_or("Dosya kapalı.")?;
        f.write_all(bytes)
            .map_err(|e| format!("“{path}” yazılamadı: {e}."))
    }

    fn patch(&mut self, at: u64, bytes: &[u8]) -> Result<(), String> {
        let path = self.path.display().to_string();
        let fail = |e: std::io::Error| format!("“{path}” yazılamadı: {e}.");
        let f = self.file.as_mut().ok_or("Dosya kapalı.")?;
        let end = f.stream_position().map_err(fail)?;
        f.seek(SeekFrom::Start(at)).map_err(fail)?;
        f.write_all(bytes).map_err(fail)?;
        f.seek(SeekFrom::Start(end)).map_err(fail)?;
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> Result<(), String> {
        self.close()?;
        put_in_place(&self.part, &self.path)?;
        self.placed = true;
        Ok(())
    }

    /// Laid out again beside its place ([`cog_file`]); the piece goes with the sink either way.
    fn finish_cog(mut self: Box<Self>) -> Result<(), String> {
        self.close()?;
        cog_file(&self.part, &self.path)
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        // Closed first: Windows removes no open file.
        drop(self.file.take());
        if !self.placed {
            let _ = std::fs::remove_file(&self.part);
        }
    }
}
