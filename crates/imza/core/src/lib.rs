//! KentOS İmza çekirdeği: hata taksonomisi ve ortak tipler (docs/adr/9002).
//!
//! Bu crate hiçbir I/O yapmaz ve hiçbir üst katmana bağımlı değildir; öteki imza crate'lerinin hepsi buna bağlıdır.
//! Amacı tek: bir hatanın ne olduğunu, kullanıcıya nasıl anlatılacağını ve tekrar denenip denenmeyeceğini tek yerde
//! tanımlamak.

pub mod error;

pub use error::{ErrorCode, ImzaError, Retryable};
