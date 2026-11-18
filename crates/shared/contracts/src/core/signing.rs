//! E-signing a dossier's files (docs/adr/9002 the signing service, 9004 the portal's side): the server issues a signed
//! request naming the files and the digests of its own stored bytes; the signer's device (KentOS İmza on the computer,
//! or the desktop in its own process) signs with the card; the server checks what comes back against its own record
//! and keeps each verified signature on the file.
//!
//! The request's envelope and the result the client hands back are the signing service's wire types
//! (`kentos-imza-domain`); these are the portal's answers around them.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::DossierSide;

/// `GET /v1/signing/trust`: the key the signing service checks the server's requests with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SigningTrust {
    /// The server's Ed25519 public key: standard base64 of its 32 bytes.
    pub public_key: String,
    pub issuer: String,
    pub audience: String,
}

/// Where the signing happens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "UPPERCASE")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SigningClientType {
    /// A browser, through KentOS İmza on the computer.
    Web,
    /// The KentOS desktop, in its own process.
    Desktop,
    Mobile,
}

/// `POST /v1/dossiers/{id}/signing`: the files of the dossier to sign.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SigningRequestCreate {
    pub client_type: SigningClientType,
    /// 1–50 files of the dossier (`DossierFileView.id`).
    pub file_ids: Vec<String>,
}

/// A file as the request names it: the digest is of the bytes the server keeps.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SigningFile {
    pub file_id: String,
    pub name: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub size: u64,
    /// `SHA-384` (the measured cards are P-384) or `SHA-256`.
    pub digest_algorithm: String,
    /// Lowercase hex.
    pub digest: String,
}

/// The request issued: its envelope goes to the signing service as it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SigningRequestCreated {
    pub transaction_id: String,
    pub batch_id: String,
    /// The compact JWS (`EdDSA`, `KSI-SR`) the signing service checks against `GET /v1/signing/trust`.
    pub signed_request: String,
    /// RFC 3339, UTC: after this the service refuses it.
    pub expires_at: String,
    pub files: Vec<SigningFile>,
}

/// What came of a delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SigningOutcome {
    /// Every file signed and verified.
    Completed,
    /// Some were.
    Partially,
    Failed,
}

/// How far a signature's legal validity was checked: the key signed these bytes is proved; the certificate's chain to
/// the authority's roots and its revocation are not checked yet (`not_done`). More values will come.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SigningLegalCheck {
    NotDone,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SigningFileVerdict {
    pub file_id: String,
    pub verified: bool,
    /// Why not, in Turkish.
    pub error: Option<String>,
}

/// The signer as the server read the certificate (not as the client said).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SigningSigner {
    pub subject: String,
    pub issuer: String,
    pub serial_number: String,
    /// SHA-256 of the certificate, upper-case hex.
    pub fingerprint_sha256: String,
}

/// `POST /v1/signing/{transactionId}/result`: the server's check of what the client delivered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SigningVerdict {
    pub accepted: bool,
    /// Why it was refused, in Turkish.
    pub reason: Option<String>,
    pub status: SigningOutcome,
    pub signed: u32,
    pub required: u32,
    pub files: Vec<SigningFileVerdict>,
    pub signer: Option<SigningSigner>,
    pub legal_check: SigningLegalCheck,
}

/// `GET /v1/dossiers/{id}/signatures`: a verified signature kept on a file of the dossier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierSignatureView {
    pub id: String,
    pub file_id: String,
    pub file_name: String,
    /// The file's round (0: not sent yet).
    pub round: u32,
    /// The side of the person who signed.
    pub side: DossierSide,
    /// The KentOS account that delivered it.
    pub signer_name: String,
    pub subject: String,
    pub issuer: String,
    pub serial_number: String,
    pub fingerprint_sha256: String,
    pub key_algorithm: String,
    pub signature_algorithm: String,
    pub digest_algorithm: String,
    /// RFC 3339: when the card signed (the client's clock).
    pub signed_at: String,
    /// RFC 3339: when the server verified it.
    pub verified_at: String,
    /// Whether the file's stored bytes still have the digest that was signed.
    pub still_matches: bool,
    pub legal_check: SigningLegalCheck,
}
