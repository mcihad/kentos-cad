//! The extension catalog as the desktop sends to it (docs/adr/8059 §10; docs/adr/8057 §107): a package uploaded to a
//! catalog as a draft and sent to its review, as a program may (the decisions are a person's, in the web's window).
//!
//! - `POST /v1/extension-catalogs/{katalog}/releases`, the `.kek`'s bytes: the draft's record (201).
//! - `POST …/releases/{eklenti}/{sürüm}/moves`, `{"action": "submit", "note": …}`: sent to review; the record again.
//!
//! A catalog is `personal:<hesap>`, `tenant:<kurumun kısa adı>` or `public`. The answers are the server's views, read
//! as JSON: the desktop shows their state and gates, it decides nothing on them.

use std::future::Future;

use reqwest::Method;
use reqwest::header;
use serde_json::{Value, json};

use super::{Cloud, FILE_TIMEOUT, Inner, TIMEOUT};
use crate::failure::ApiFailure;

impl Inner {
    /// `/v1/extension-catalogs/{catalog}/releases` and the segments after it, each written as a path segment.
    fn releases_url(&self, catalog: &str, rest: &[&str]) -> Result<reqwest::Url, ApiFailure> {
        let mut url = self.url("/v1/extension-catalogs")?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|()| ApiFailure::local("Sunucu adresi bir yol taşıyamıyor.".to_owned()))?;
            segments.push(catalog).push("releases");
            for segment in rest {
                segments.push(segment);
            }
        }
        Ok(url)
    }
}

impl Cloud {
    /// `package` (a `.kek`'s bytes) uploaded to `catalog` as a draft: the server's record of it.
    pub fn upload_release(
        &self,
        catalog: &str,
        package: Vec<u8>,
    ) -> impl Future<Output = Result<Value, ApiFailure>> + Send + 'static {
        let catalog = catalog.to_owned();
        self.call(move |inner| async move {
            let url = inner.releases_url(&catalog, &[])?;
            let b = inner
                .request(Method::POST, url, FILE_TIMEOUT)
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .body(package);
            inner.json(b, FILE_TIMEOUT).await
        })
    }

    /// The release `extension` `version` of `catalog` sent to its review, with the publisher's `note`: the record.
    pub fn submit_release(
        &self,
        catalog: &str,
        extension: &str,
        version: &str,
        note: Option<String>,
    ) -> impl Future<Output = Result<Value, ApiFailure>> + Send + 'static {
        let (catalog, extension, version) =
            (catalog.to_owned(), extension.to_owned(), version.to_owned());
        self.call(move |inner| async move {
            let url = inner.releases_url(&catalog, &[&extension, &version, "moves"])?;
            let b = inner.request(Method::POST, url, TIMEOUT);
            let body = json!({ "action": "submit", "note": note });
            inner.json(Inner::json_body(b, &body)?, TIMEOUT).await
        })
    }
}
