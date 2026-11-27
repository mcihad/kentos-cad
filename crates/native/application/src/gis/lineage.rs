//! What a layer's derivation (docs/adr/0633 §2) and a run's record name besides the layers they read: the engine and the
//! time, one rule for İşlemler's tools and the network analyses (docs/adr/9334). The layers' digests are
//! [`crate::sources::layer_digest`].

/// The engine a layer's derivation names (`KentOS <version>`, docs/adr/0633 §2): İşlemler's tools and the network
/// analyses (docs/adr/9334) alike.
pub fn engine() -> String {
    format!("KentOS {}", env!("CARGO_PKG_VERSION"))
}

/// A time as a derivation and a run's record write it, `YYYY-MM-DDTHH:MM:SS.SSSZ`, from milliseconds since the Unix
/// epoch.
pub fn utc_at(ms: u64) -> String {
    let text = kentos_geometry_core::time::write(ms as f64, false);
    // A whole second is written without its milliseconds, and midnight as the day alone.
    let text = if text.len() == 10 {
        format!("{text}T00:00:00")
    } else {
        text
    };
    let text = if text.len() == 19 {
        format!("{text}.000")
    } else {
        text
    };
    format!("{text}Z")
}
