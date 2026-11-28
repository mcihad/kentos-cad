//! E-signing a dossier's files (docs/adr/9004; plan BUL-21). The server is the one that knows what is signed: it reads
//! the files' bytes from its own store, writes their digests into a request it signs with its Ed25519 key (the signing
//! service trusts only that key, docs/adr/9002), and keeps the request. The signer's device signs those digests with the
//! card and hands the result back; the server checks it against its own request and against the bytes it keeps now
//! (`kentos_imza_signature::delivery`), and keeps each verified signature on its file. Nothing the client says about
//! the files, the digests or the signer is taken as it is.
//!
//! What is proved is that the certificate's key signed these bytes. The certificate's chain to the authority's roots
//! and its revocation are not checked yet: every answer says so (`legal_check: not_done`).

use std::collections::{BTreeMap, BTreeSet};

use kentos_contracts::{
    DossierSide, DossierSignatureView, SigningClientType, SigningFile, SigningFileVerdict,
    SigningLegalCheck, SigningOutcome, SigningRequestCreate, SigningRequestCreated, SigningSigner,
    SigningTrust, SigningVerdict, TenantRole,
};
use kentos_imza_domain::{BatchResult, PayloadFile, PayloadPackage, SignedRequestPayload};

/// What the signing service hands back after signing (its `BatchResult`), delivered to the server as it is.
pub use kentos_imza_domain::BatchResult as SigningDelivery;
use kentos_imza_security::issue::{ServerKey, issue};
use kentos_imza_security::{DEFAULT_AUDIENCE, DEFAULT_ISSUER};
use kentos_imza_signature::delivery::verify_delivery;
use kentos_postgres::Db;
use sha2::{Digest, Sha384};
use sqlx::types::Json;
use sqlx::{Postgres, Transaction};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use super::accounts::user_scope;
use super::dossiers::{Held, hold, record};
use super::flow::Party;
use super::rows::side_code;
use super::{id_at, rfc3339};
use crate::blobs::Blobs;
use crate::error::{AppError, AppResult};
use crate::identity::Actor;

/// How long a request is good for: the person has this long to open it and enter the PIN.
pub const REQUEST_LIFETIME: Duration = Duration::minutes(15);
/// At most this many files in one request.
pub const MAX_FILES: usize = 50;
const DIGEST_ALGORITHM: &str = "SHA-384";

/// The server's signing key and the names the signing service checks (`KENTOS_IMZA_SIGNING_KEY`,
/// `KENTOS_IMZA_ISSUER`, `KENTOS_IMZA_AUDIENCE`).
pub struct Signer {
    key: ServerKey,
    issuer: String,
    audience: String,
}

impl std::fmt::Debug for Signer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signer")
            .field("issuer", &self.issuer)
            .field("audience", &self.audience)
            .field("public_key", &self.key.public_base64())
            .finish_non_exhaustive()
    }
}

impl Signer {
    /// From the key as configured (PEM, base64 PKCS#8 DER or a base64 32-byte seed); the default issuer
    /// `kentos-server` and audience `kentos-imza` when not given.
    pub fn from_config(
        key: &str,
        issuer: Option<&str>,
        audience: Option<&str>,
    ) -> Result<Self, String> {
        let key = ServerKey::from_config(key.trim())
            .map_err(|e| format!("KENTOS_IMZA_SIGNING_KEY okunamadı: {e}"))?;
        let name = |v: Option<&str>, default: &str| {
            v.map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(default)
                .to_owned()
        };
        Ok(Self {
            key,
            issuer: name(issuer, DEFAULT_ISSUER),
            audience: name(audience, DEFAULT_AUDIENCE),
        })
    }

    /// A fresh key, for tests.
    pub fn generated() -> Result<Self, String> {
        let key = ServerKey::generate().map_err(|e| e.to_string())?;
        Ok(Self {
            key,
            issuer: DEFAULT_ISSUER.into(),
            audience: DEFAULT_AUDIENCE.into(),
        })
    }

    /// `GET /v1/signing/trust`.
    pub fn trust(&self) -> SigningTrust {
        SigningTrust {
            public_key: self.key.public_base64(),
            issuer: self.issuer.clone(),
            audience: self.audience.clone(),
        }
    }
}

/// E-signing is not set up on this server.
pub fn off() -> AppError {
    AppError::Conflict {
        message: "E-imza bu sunucuda yapılandırılmamış (KENTOS_IMZA_SIGNING_KEY); sistem yöneticisine başvurun.".into(),
        conflicts: Vec::new(),
        revision: None,
    }
}

fn conflict(message: impl Into<String>) -> AppError {
    AppError::Conflict {
        message: message.into(),
        conflicts: Vec::new(),
        revision: None,
    }
}

/// The SHA-384 of a stored file, lowercase hex, read in pieces.
async fn digest_of(blobs: &Blobs, key: &str) -> AppResult<String> {
    let mut file = blobs.open(key).await.map_err(AppError::Storage)?;
    let mut hasher = Sha384::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = file.read(&mut buf).await.map_err(AppError::Storage)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

fn utc(t: OffsetDateTime) -> String {
    t.to_offset(time::UtcOffset::UTC)
        .format(&Rfc3339)
        .unwrap_or_default()
}

/// Whether `held`'s caller signs its files: the professional's side who prepares it, the municipality's editors and
/// above; a viewer reads.
fn may_sign(held: &Held) -> AppResult<()> {
    if !held.may_act() {
        return Err(AppError::forbidden(
            "Meslek mensubu kaydınız doğrulanmadığı için bu dosyada belge imzalayamazsınız.",
        ));
    }
    let ok = match held.party {
        Party::Member(r) => r >= TenantRole::Editor,
        p => p.works_for_professional(),
    };
    if ok {
        Ok(())
    } else {
        Err(AppError::forbidden(
            "Belgeleri e-imzalamak düzenleyici ve üstünün işidir.",
        ))
    }
}

type FileRow = (Uuid, String, String, i64, String, String);

/// `POST /v1/dossiers/{id}/signing`: a request for the files' signatures, signed by the server.
pub async fn request(
    db: &Db,
    blobs: &Blobs,
    signer: Option<&Signer>,
    actor: &Actor,
    dossier: Uuid,
    input: &SigningRequestCreate,
) -> AppResult<SigningRequestCreated> {
    let signer = signer.ok_or_else(off)?;
    let mut ids = Vec::new();
    let mut seen = BTreeSet::new();
    for (i, text) in input.file_ids.iter().enumerate() {
        let id = id_at(&format!("fileIds[{i}]"), text)?;
        if seen.insert(id) {
            ids.push(id);
        }
    }
    if ids.is_empty() || ids.len() > MAX_FILES {
        return Err(AppError::invalid_at(
            "fileIds",
            format!("İmzalanacak 1–{MAX_FILES} belge seçin."),
        ));
    }
    let mut tx = db.scoped(user_scope(actor)).await?;
    let held = hold(&mut tx, actor, dossier, false).await?;
    may_sign(&held)?;
    let rows: Vec<FileRow> = sqlx::query_as(
        "select id, name, media_type, size, blob_key, sha256 from kentos.dossier_file
          where dossier_id = $1 and id = any($2)",
    )
    .bind(dossier)
    .bind(&ids)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    if rows.len() != ids.len() {
        return Err(AppError::not_found(
            "Seçilen belgelerden biri bu dosyada yok ya da göremiyorsunuz.",
        ));
    }
    let by_id: BTreeMap<Uuid, &FileRow> = rows.iter().map(|r| (r.0, r)).collect();
    let mut files = Vec::new();
    for id in &ids {
        let (fid, name, media_type, size, key, _) = by_id[id];
        let digest = digest_of(blobs, key).await?;
        files.push(PayloadFile {
            id: fid.to_string(),
            name: name.clone(),
            role: "MAIN_DOCUMENT".into(),
            mime_type: media_type.clone(),
            size: u64::try_from(*size).unwrap_or(0),
            digest_algorithm: DIGEST_ALGORITHM.into(),
            digest,
            signing_required: true,
        });
    }
    let now = OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .unwrap_or_else(|_| OffsetDateTime::now_utc());
    let expires = now + REQUEST_LIFETIME;
    let transaction = Uuid::now_v7();
    let batch = Uuid::new_v4();
    let side = held.party.side();
    let payload = SignedRequestPayload {
        transaction_id: transaction.to_string(),
        batch_id: batch.to_string(),
        user_id: actor.user_id.to_string(),
        // KentOS does not bind signing to a session; the transaction names it.
        session_id: transaction.to_string(),
        signature_method: "ELECTRONIC_SIGNATURE".into(),
        client_type: match input.client_type {
            SigningClientType::Web => "WEB",
            SigningClientType::Desktop => "DESKTOP",
            SigningClientType::Mobile => "MOBILE",
        }
        .into(),
        packages: vec![PayloadPackage {
            id: dossier.to_string(),
            title: held.row.title.clone(),
            reference_number: held.row.number(),
            document_type: Some(held.row.kind.clone()),
            metadata: serde_json::json!({ "dossierId": dossier, "side": side_code(side) }),
            files: files.clone(),
        }],
        created_at: utc(now),
        expires_at: utc(expires),
        nonce: format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple()),
        issuer: signer.issuer.clone(),
        audience: signer.audience.clone(),
    };
    let signed_request = issue(&payload, &signer.key, None).map_err(|e| {
        AppError::Storage(std::io::Error::other(format!(
            "imza isteği imzalanamadı: {e}"
        )))
    })?;
    let mut tx = db.scoped(user_scope(actor)).await?;
    sqlx::query(
        "insert into kentos.signing_request (id, batch_id, dossier_id, requested_by, requested_by_name, side,
                                             client_type, payload, expires_at)
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(transaction)
    .bind(batch)
    .bind(dossier)
    .bind(actor.user_id)
    .bind(&actor.display_name)
    .bind(side_code(side))
    .bind(&payload.client_type)
    .bind(Json(&payload))
    .bind(expires)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(SigningRequestCreated {
        transaction_id: payload.transaction_id,
        batch_id: payload.batch_id,
        signed_request,
        expires_at: payload.expires_at,
        files: files
            .into_iter()
            .map(|f| SigningFile {
                file_id: f.id,
                name: f.name,
                size: f.size,
                digest_algorithm: f.digest_algorithm,
                digest: f.digest,
            })
            .collect(),
    })
}

type RequestRow = (
    Uuid,
    String,
    Json<SignedRequestPayload>,
    OffsetDateTime,
    String,
);

async fn held_request(tx: &mut Transaction<'static, Postgres>, id: Uuid) -> AppResult<RequestRow> {
    sqlx::query_as(
        "select dossier_id, state, payload, expires_at, side from kentos.signing_request where id = $1 for update",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| AppError::not_found("Böyle bir imza isteği yok ya da sizin değil."))
}

fn settled(state: &str) -> AppError {
    conflict(match state {
        "cancelled" => "Bu imza isteği iptal edildi; yeniden imzalamak için yeni bir istek açın.",
        "expired" => "Bu imza isteğinin süresi doldu; yeniden imzalamak için yeni bir istek açın.",
        _ => "Bu imza isteği sonuçlandı; ilk teslim geçerli.",
    })
}

fn verdict_of(v: &kentos_imza_signature::delivery::DeliveryVerdict) -> SigningVerdict {
    SigningVerdict {
        accepted: v.accepted,
        reason: v.reason.clone(),
        status: match v.status() {
            "completed" => SigningOutcome::Completed,
            "partially" => SigningOutcome::Partially,
            _ => SigningOutcome::Failed,
        },
        signed: u32::try_from(v.signed).unwrap_or(u32::MAX),
        required: u32::try_from(v.required).unwrap_or(u32::MAX),
        files: v
            .files
            .iter()
            .map(|f| SigningFileVerdict {
                file_id: f.file_id.clone(),
                verified: f.verified,
                error: f.error.clone(),
            })
            .collect(),
        signer: v.signer.as_ref().map(|s| SigningSigner {
            subject: s.subject.clone(),
            issuer: s.issuer.clone(),
            serial_number: s.serial_number.clone(),
            fingerprint_sha256: s.fingerprint_sha256.clone(),
        }),
        legal_check: SigningLegalCheck::NotDone,
    }
}

/// The certificate subject's common name, for a line of the timeline (`CN=Ayşe Yılmaz, …` → `Ayşe Yılmaz`).
fn common_name(subject: &str) -> String {
    subject
        .split(',')
        .map(str::trim)
        .find_map(|part| part.strip_prefix("CN=").map(str::to_owned))
        .unwrap_or_else(|| subject.to_owned())
}

/// `POST /v1/signing/{transactionId}/result`: what the signer's device delivered, checked against the server's own
/// request and the bytes it keeps now; each verified signature is kept on its file.
pub async fn deliver(
    db: &Db,
    blobs: &Blobs,
    actor: &Actor,
    transaction: Uuid,
    result: &BatchResult,
) -> AppResult<SigningVerdict> {
    if result.transaction_id != transaction.to_string() {
        return Err(AppError::invalid(
            "Teslim edilen sonuç bu imza isteğine ait değil.",
        ));
    }
    let mut tx = db.scoped(user_scope(actor)).await?;
    let (dossier, state, Json(issued), expires, side) = held_request(&mut tx, transaction).await?;
    if state != "issued" {
        return Err(settled(&state));
    }
    if OffsetDateTime::now_utc() > expires {
        sqlx::query("update kentos.signing_request set state = 'expired' where id = $1")
            .bind(transaction)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Err(settled("expired"));
    }
    let held = hold(&mut tx, actor, dossier, true).await?;
    let file_ids: Vec<Uuid> = issued
        .packages
        .iter()
        .flat_map(|p| p.files.iter())
        .filter_map(|f| Uuid::parse_str(&f.id).ok())
        .collect();
    let stored: Vec<(Uuid, String, String)> =
        sqlx::query_as("select id, blob_key, sha256 from kentos.dossier_file where dossier_id = $1 and id = any($2)")
            .bind(dossier)
            .bind(&file_ids)
            .fetch_all(&mut *tx)
            .await?;
    let mut now_digest = BTreeMap::new();
    for (id, key, _) in &stored {
        now_digest.insert(id.to_string(), digest_of(blobs, key).await?);
    }
    let file_sha: BTreeMap<String, String> = stored
        .iter()
        .map(|(id, _, sha)| (id.to_string(), sha.clone()))
        .collect();
    let verdict = verify_delivery(&issued, result, |id| now_digest.get(id).cloned());
    let answer = verdict_of(&verdict);
    if let (Some(signer), Some(certificate)) = (&verdict.signer, &result.signer) {
        for f in verdict.files.iter().filter(|f| f.verified) {
            let Some(sig) = result.results.iter().find(|r| r.file_id == f.file_id) else {
                continue;
            };
            let Some(issued_file) = issued
                .packages
                .iter()
                .flat_map(|p| p.files.iter())
                .find(|x| x.id == f.file_id)
            else {
                continue;
            };
            let signed_at = OffsetDateTime::parse(&sig.signature.signed_at, &Rfc3339)
                .unwrap_or_else(|_| OffsetDateTime::now_utc());
            sqlx::query(
                "insert into kentos.dossier_signature (id, dossier_id, file_id, request_id, side, signer_user_id,
                        signer_name, subject, issuer, serial_number, fingerprint_sha256, key_algorithm,
                        signature_algorithm, digest_algorithm, digest, file_sha256, signature_value, certificate_der,
                        signed_at)
                 values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)",
            )
            .bind(Uuid::now_v7())
            .bind(dossier)
            .bind(Uuid::parse_str(&f.file_id).map_err(|_| AppError::invalid("Belge kimliği geçersiz."))?)
            .bind(transaction)
            .bind(&side)
            .bind(actor.user_id)
            .bind(&actor.display_name)
            .bind(&signer.subject)
            .bind(&signer.issuer)
            .bind(&signer.serial_number)
            .bind(&signer.fingerprint_sha256)
            .bind(&signer.key_algorithm)
            .bind(&signer.signature_algorithm)
            .bind(&issued_file.digest_algorithm)
            .bind(&issued_file.digest)
            .bind(file_sha.get(&f.file_id).cloned().unwrap_or_default())
            .bind(&sig.signature.signature_value)
            .bind(&certificate.certificate_der)
            .bind(signed_at)
            .execute(&mut *tx)
            .await?;
        }
    }
    sqlx::query(
        "update kentos.signing_request set state = $2, delivered_at = now(), verdict = $3 where id = $1",
    )
    .bind(transaction)
    .bind(verdict.status())
    .bind(Json(&answer))
    .execute(&mut *tx)
    .await?;
    if verdict.signed > 0 {
        let subject = verdict
            .signer
            .as_ref()
            .map(|s| common_name(&s.subject))
            .unwrap_or_default();
        record(
            &mut tx,
            dossier,
            actor,
            held.party.side(),
            "signed",
            held.row.round(),
            serde_json::json!({ "files": verdict.signed, "subject": subject }),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(answer)
}

/// `POST /v1/signing/{transactionId}/cancel`: its requester gives it up while it is open.
pub async fn cancel(db: &Db, actor: &Actor, transaction: Uuid) -> AppResult<()> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let (_, state, ..) = held_request(&mut tx, transaction).await?;
    if state != "issued" {
        return Err(settled(&state));
    }
    sqlx::query("update kentos.signing_request set state = 'cancelled' where id = $1")
        .bind(transaction)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

type SignatureRow = (
    Uuid,
    Uuid,
    String,
    Option<i32>,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    OffsetDateTime,
    OffsetDateTime,
    bool,
);

/// `GET /v1/dossiers/{id}/signatures`: the verified signatures on the dossier's files the caller sees.
pub async fn signatures(
    db: &Db,
    actor: &Actor,
    dossier: Uuid,
) -> AppResult<Vec<DossierSignatureView>> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    hold(&mut tx, actor, dossier, false).await?;
    let rows: Vec<SignatureRow> = sqlx::query_as(
        "select s.id, s.file_id, f.name, f.round, s.side, s.signer_name, s.subject, s.issuer, s.serial_number,
                s.fingerprint_sha256, s.key_algorithm, s.signature_algorithm, s.digest_algorithm, s.signed_at,
                s.verified_at, f.sha256 = s.file_sha256
           from kentos.dossier_signature s join kentos.dossier_file f on f.id = s.file_id
          where s.dossier_id = $1 order by s.verified_at, s.id",
    )
    .bind(dossier)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows
        .into_iter()
        .map(|r| DossierSignatureView {
            id: r.0.to_string(),
            file_id: r.1.to_string(),
            file_name: r.2,
            round: r.3.and_then(|n| u32::try_from(n).ok()).unwrap_or(0),
            side: if r.4 == "municipality" {
                DossierSide::Municipality
            } else {
                DossierSide::Professional
            },
            signer_name: r.5,
            subject: r.6,
            issuer: r.7,
            serial_number: r.8,
            fingerprint_sha256: r.9,
            key_algorithm: r.10,
            signature_algorithm: r.11,
            digest_algorithm: r.12,
            signed_at: rfc3339(r.13),
            verified_at: rfc3339(r.14),
            still_matches: r.15,
            legal_check: SigningLegalCheck::NotDone,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signer_names_its_key_and_hides_it() {
        let s = Signer::generated().unwrap();
        let t = s.trust();
        assert_eq!(
            (t.issuer.as_str(), t.audience.as_str()),
            ("kentos-server", "kentos-imza")
        );
        assert_eq!(t.public_key.len(), 44, "standard base64 of 32 bytes");
        let shown = format!("{s:?}");
        assert!(shown.contains(&t.public_key));
        // The same key read back from its PKCS#8 form names the same public key.
        let again =
            Signer::from_config(&s.key.private_pkcs8_base64(), Some("  "), Some("başka")).unwrap();
        assert_eq!(again.trust().public_key, t.public_key);
        assert_eq!(again.trust().audience, "başka");
        assert!(Signer::from_config("anahtar değil", None, None).is_err());
    }

    #[test]
    fn the_timeline_names_the_certificate_s_holder() {
        assert_eq!(
            common_name("CN=Ayşe Yılmaz, SERIALNUMBER=1, C=TR"),
            "Ayşe Yılmaz"
        );
        assert_eq!(common_name("O=Kurum"), "O=Kurum");
    }
}
