//! Files in the cloud for every client (docs/adr/9000 §13-§15): an upload in parts that a dossier or a project takes,
//! the scan of what arrives, and a project's attachments.
//!
//! The flow, the same for the desktop, a phone and an agent:
//! 1. `POST /v1/uploads` with the file's name, size and SHA-256 opens an upload ([`UploadBegin`] → [`UploadView`]).
//! 2. `PUT /v1/uploads/{id}?offset=N` sends the bytes from `N` on, at most `partMax` a request; a request cut short
//!    costs only itself, and `GET /v1/uploads/{id}` says how far it came (`receivedBytes`).
//! 3. The last part checks the size and the hash and scans the file; the upload is then `received`, with its scan.
//! 4. A destination takes it by its id: `POST /v1/dossiers/{id}/files/from-upload` or
//!    `POST /v1/tenants/{tenant}/projects/{project}/attachments`. An upload nobody takes goes after a day.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// How far a kept file was checked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ScanState {
    /// An antivirus daemon scanned it and found nothing; the type rules passed it too.
    Clean,
    /// Only the type rules passed it: the server has no antivirus daemon, or the daemon could not scan it then.
    #[default]
    Unscanned,
}

impl ScanState {
    pub fn code(self) -> &'static str {
        match self {
            ScanState::Clean => "clean",
            ScanState::Unscanned => "unscanned",
        }
    }

    pub fn of(code: &str) -> Self {
        if code == "clean" {
            ScanState::Clean
        } else {
            ScanState::Unscanned
        }
    }
}

/// `POST /v1/uploads`: what is about to be sent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UploadBegin {
    /// The file's name with its extension (`pafta-3.pdf`): its kind is read from it and checked against its bytes.
    pub name: String,
    /// Bytes, at least 1.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub size: u64,
    /// The whole file's SHA-256, 64 lower-case hexadecimal digits.
    pub sha256: String,
}

/// An upload and how far it came.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UploadView {
    pub id: String,
    pub name: String,
    /// The media type of the name's kind; after the scan, the kind its bytes were found to be.
    pub media_type: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub size: u64,
    pub sha256: String,
    /// The bytes that arrived: the next part starts here.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub received_bytes: u64,
    /// Every byte arrived, matched the hash and passed the scan: a destination may take it.
    pub received: bool,
    pub scan: Option<ScanState>,
    pub scan_note: Option<String>,
    /// The largest part one request may carry, bytes.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub part_max: u64,
    pub created_at: String,
    /// When it is removed if nothing takes it.
    pub expires_at: String,
}

/// `POST /v1/dossiers/{id}/files/from-upload`: a received upload becomes a dossier's file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierFileFromUpload {
    pub upload_id: String,
    /// `package` (the application's or the delivery's documents) or `attachment` (with the next message).
    pub purpose: String,
    /// A document type of the portal's catalogue (`GET /v1/portal/catalog`); absent: found from the name.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub doc_type: Option<String>,
}

/// `POST /v1/tenants/{tenant}/projects/{project}/attachments`: a received upload becomes the project's attachment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AttachmentCreate {
    pub upload_id: String,
    /// A word on what it is (“Zemin etüdü, 2. sondaj”), at most 2000 characters.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub note: Option<String>,
    /// The drawing's revision it goes with (a file project's), if any.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub revision: Option<i64>,
}

/// A project's attachment: a report, a sheet, a table, a photo or a source file kept beside the drawing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AttachmentView {
    pub id: String,
    pub name: String,
    pub media_type: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub size: u64,
    pub sha256: String,
    pub scan: ScanState,
    pub scan_note: Option<String>,
    pub note: Option<String>,
    pub revision: Option<i64>,
    pub created_by_name: String,
    pub created_at: String,
    /// Shown in the browser or the phone as it is (a PDF or a picture), not only saved.
    pub viewable: bool,
}
