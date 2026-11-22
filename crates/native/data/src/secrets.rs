//! Where a driver asks for a password (docs/adr/9100 §7): a source's address never holds one (a project file is
//! shared, a log is read); the host registers how it finds them (the device's saved connections, the session's
//! entered ones, the system's key ring) and a driver asks with the address's parts.

use std::sync::OnceLock;

/// A password finder.
pub trait Secrets: Send + Sync {
    /// The password for `user` on `host:port`'s `database`, none when not known.
    fn password(&self, host: &str, port: u16, database: &str, user: &str) -> Option<String>;
}

static SECRETS: OnceLock<Box<dyn Secrets>> = OnceLock::new();

/// Registers the host's password finder; the first registration stays.
pub fn set_secrets(secrets: Box<dyn Secrets>) {
    let _ = SECRETS.set(secrets);
}

/// The password the host knows for that login.
pub fn password(host: &str, port: u16, database: &str, user: &str) -> Option<String> {
    SECRETS
        .get()
        .and_then(|s| s.password(host, port, database, user))
}
