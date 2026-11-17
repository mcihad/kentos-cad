//! The persistent store's packages (docs/adr/8059 §13): a package's bytes in the object store under its content
//! digest, written once under a lock on the digest (to a key of their own, flushed, renamed into place, then the row),
//! read whole and checked against the row's size.

use axum::body::Bytes;
use kentos_application::blobs::Blobs;
use uuid::Uuid;

use super::{PgStore, failed};
use crate::extension_catalog::store::StoreError;

/// The hex of a content digest (`sha256:` and 64 lowercase hex digits).
fn digest_hex(digest: &str) -> Option<&str> {
    digest
        .strip_prefix("sha256:")
        .filter(|h| h.len() == 64 && h.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
}

impl PgStore {
    /// Keeps `bytes` under `digest` unless a package is kept there already.
    pub(super) async fn keep_package(&self, digest: &str, bytes: Bytes) -> Result<(), StoreError> {
        let hex = digest_hex(digest)
            .ok_or_else(|| StoreError::Failed(format!("“{digest}” bir içerik özeti değil")))?;
        let mut tx = self.db.pool.begin().await.map_err(failed)?;
        // One writer per digest: the second waits, then finds the row.
        sqlx::query("select pg_advisory_xact_lock(hashtextextended($1, 8059))")
            .bind(digest)
            .execute(&mut *tx)
            .await
            .map_err(failed)?;
        let kept: bool = sqlx::query_scalar(
            "select exists (select 1 from kentos.extension_package where digest = $1)",
        )
        .bind(digest)
        .fetch_one(&mut *tx)
        .await
        .map_err(failed)?;
        if kept {
            return Ok(());
        }
        let key = Blobs::extension_package_key(hex);
        let staging = Blobs::extension_package_staging_key(Uuid::now_v7());
        let written = async {
            let mut writer = self.blobs.create(&staging, bytes.len() as u64).await?;
            writer
                .write(&bytes)
                .await
                .map_err(|e| std::io::Error::other(format!("{e:?}")))?;
            let done = writer.finish().await?;
            self.blobs.promote(&staging, &key).await?;
            Ok::<_, std::io::Error>(done)
        }
        .await;
        let (size, sha256) = match written {
            Ok(done) => done,
            Err(e) => {
                let _ = self.blobs.remove(&staging).await;
                return Err(StoreError::Failed(format!(
                    "{digest} paketi nesne deposuna yazılamadı: {e}"
                )));
            }
        };
        sqlx::query(
            "insert into kentos.extension_package (digest, size, sha256, object_key) values ($1, $2, $3, $4)",
        )
        .bind(digest)
        .bind(i64::try_from(size).map_err(failed)?)
        .bind(&sha256)
        .bind(&key)
        .execute(&mut *tx)
        .await
        .map_err(failed)?;
        tx.commit().await.map_err(failed)
    }

    /// The bytes kept under `digest`; none when there is no such package, a failure when its object is not whole.
    pub(super) async fn kept_package(&self, digest: &str) -> Result<Option<Bytes>, StoreError> {
        let row: Option<(String, i64)> = sqlx::query_as(
            "select object_key, size from kentos.extension_package where digest = $1",
        )
        .bind(digest)
        .fetch_optional(&self.db.pool)
        .await
        .map_err(failed)?;
        let Some((key, size)) = row else {
            return Ok(None);
        };
        let bytes = self.blobs.read(&key).await.map_err(|e| {
            StoreError::Failed(format!("{digest} paketinin nesnesi okunamadı: {e}"))
        })?;
        if i64::try_from(bytes.len()).ok() != Some(size) {
            return Err(StoreError::Failed(format!(
                "{digest} paketinin nesnesi {} bayt, kaydı {size} bayt diyor",
                bytes.len()
            )));
        }
        Ok(Some(Bytes::from(bytes)))
    }
}
