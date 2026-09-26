//! Why a file is refused or a drawing cannot be written: a stable code (the
//! specification's table, docs/specs/kcad-v2.md §9; programs and fixtures
//! compare it) and a Turkish message that says the cause and the fix (CLAUDE.md §8).

use std::fmt;

/// The specification's error codes (§9), in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Code {
    Empty,
    NotKcad,
    DamagedSignature,
    Truncated,
    TrailingData,
    UnsupportedVersion,
    NewerVersion,
    BadHeader,
    UnknownEncoding,
    UnknownCodec,
    UnknownExtension,
    TooLarge,
    HashMismatch,
    Malformed,
    IndefiniteLength,
    NonShortest,
    NarrowFloat,
    NonFinite,
    SimpleValue,
    Tag,
    InvalidUtf8,
    NonTextKey,
    UnsortedKeys,
    DuplicateKey,
    TooDeep,
    TooLong,
    CborTruncated,
    CborTrailing,
    SchemaFormat,
    SchemaVersion,
    UnknownField,
    MissingField,
    WrongType,
    BadValue,
    UnknownKind,
    DuplicateUid,
    /// A writer's own check (§12): the bytes it made did not read back to the drawing.
    VerifyFailed,
}

impl Code {
    /// The code as the specification and the fixtures write it.
    pub fn as_str(self) -> &'static str {
        match self {
            Code::Empty => "empty",
            Code::NotKcad => "not_kcad",
            Code::DamagedSignature => "damaged_signature",
            Code::Truncated => "truncated",
            Code::TrailingData => "trailing_data",
            Code::UnsupportedVersion => "unsupported_version",
            Code::NewerVersion => "newer_version",
            Code::BadHeader => "bad_header",
            Code::UnknownEncoding => "unknown_encoding",
            Code::UnknownCodec => "unknown_codec",
            Code::UnknownExtension => "unknown_extension",
            Code::TooLarge => "too_large",
            Code::HashMismatch => "hash_mismatch",
            Code::Malformed => "malformed",
            Code::IndefiniteLength => "indefinite_length",
            Code::NonShortest => "non_shortest",
            Code::NarrowFloat => "narrow_float",
            Code::NonFinite => "non_finite",
            Code::SimpleValue => "simple_value",
            Code::Tag => "tag",
            Code::InvalidUtf8 => "invalid_utf8",
            Code::NonTextKey => "non_text_key",
            Code::UnsortedKeys => "unsorted_keys",
            Code::DuplicateKey => "duplicate_key",
            Code::TooDeep => "too_deep",
            Code::TooLong => "too_long",
            Code::CborTruncated => "cbor_truncated",
            Code::CborTrailing => "cbor_trailing",
            Code::SchemaFormat => "schema_format",
            Code::SchemaVersion => "schema_version",
            Code::UnknownField => "unknown_field",
            Code::MissingField => "missing_field",
            Code::WrongType => "wrong_type",
            Code::BadValue => "bad_value",
            Code::UnknownKind => "unknown_kind",
            Code::DuplicateUid => "duplicate_uid",
            Code::VerifyFailed => "verify_failed",
        }
    }
}

/// A file that cannot be read, or a drawing that cannot be written, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KcadError {
    pub code: Code,
    /// Turkish: what is wrong, where, and what to do.
    pub message: String,
}

impl KcadError {
    pub(crate) fn new(code: Code, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// A payload that breaks the CBOR profile or the schema: `what` happened at `path`, `at` bytes into the payload.
    pub(crate) fn payload(code: Code, path: &str, at: usize, what: &str) -> Self {
        let place = if path.is_empty() {
            format!("yükün {at}. baytı")
        } else {
            format!("{path}, yükün {at}. baytı")
        };
        Self::new(
            code,
            format!(
                "Dosya bozuk ya da KCAD 2 kurallarına uymuyor: {what} ({place}). Dosya bu kurallara uymayan bir programla yazılmış olabilir; özgün dosyayı ya da bir yedeği açın."
            ),
        )
    }

    /// A drawing that cannot be written as KCAD 2: `what` at `path`.
    pub(crate) fn unwritable(code: Code, path: &str, what: &str) -> Self {
        let place = if path.is_empty() {
            String::new()
        } else {
            format!("{path}: ")
        };
        Self::new(
            code,
            format!(
                "Çizim KCAD 2 olarak yazılamıyor: {place}{what}. Değeri düzeltip yeniden kaydedin."
            ),
        )
    }
}

impl fmt::Display for KcadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for KcadError {}
