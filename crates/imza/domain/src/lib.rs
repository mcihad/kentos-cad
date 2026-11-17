//! KentOS İmza alan modeli (docs/adr/9002 §3).
//!
//! Bu crate saf veridir: I/O yok, ağ yok, kart yok.
//!
//! * [`model`] — `SigningBatch` / `SigningPackage` / `SigningFile`,
//! * [`state`] — üç durum makinesi ve **geçersiz geçişi REDDEDEN** tek kapı,
//! * [`signature`] — imza sonucu, sertifika ve cihaz bilgisi,
//! * [`wire`] — sunucunun imzaladığı isteğin gövdesi ve istemcinin geri verdiği sonuç (tel sözleşmesi),
//! * [`record`] — bir batch'in bellekteki kaydı; servis ve masaüstü aynı geçiş kurallarını bununla uygular.
//!
//! Alan modelinin I/O'dan ayrı durması bir üslup tercihi değil: durum makinesi kuralları ancak saf oldukları için
//! kartsız, ağsız ve tam olarak test edilebiliyor. KentOS sunucusu da tel tiplerini buradan alır.

pub mod model;
pub mod record;
pub mod signature;
pub mod state;
pub mod wire;

pub use model::{
    DigestAlgorithm, SignatureMethod, SigningBatch, SigningClientType, SigningFile,
    SigningFileRole, SigningPackage,
};
pub use record::BatchRecord;
pub use signature::{
    CertificateInfo, DeviceInfo, QualifiedStatus, RevocationStatus, SignatureFormat,
    SignatureResult, VerificationResult, VerificationStatus,
};
pub use state::{BatchStatus, FileStatus, InvalidTransition, PackageStatus, StateMachine};
pub use wire::{
    BatchResult, FileSignature, PayloadFile, PayloadPackage, SignedRequestPayload,
    SignerCertificate,
};
