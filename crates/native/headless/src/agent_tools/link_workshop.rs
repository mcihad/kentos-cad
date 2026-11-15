//! An agent's extension workshop on a desktop's link (docs/adr/8057 §30): the MCP server's agent, attached to a
//! desktop, opens its workshop package in the user's Gözden geçir ve kur window, loads the changes of what the user
//! installed from its folder, and reads what the user sees of it — the desktop's requests (`review_workshop`,
//! `reload_extension`, `preview_extension`), as Asistan does in the app. Kur and the capabilities stay the user's.
//! A host without them (a drawing of the server's own) says where the package is opened instead.

use std::path::Path;

use serde_json::{Value, json};

use super::Host;
use crate::HeadlessError;

/// A host's refusal of a workshop request: one without the desktop's requests says `instead`.
fn or_instead(e: HeadlessError, instead: String) -> HeadlessError {
    if e.code == "unknown_method" {
        return HeadlessError::new("not_offered", instead);
    }
    e
}

/// The workshop's `package` (made of `folder`) opened for the user's review on the desktop; nothing is installed.
pub fn link_review_workshop(
    host: &mut dyn Host,
    package: &Path,
    folder: &Path,
) -> Result<Value, HeadlessError> {
    host.request(
        "review_workshop",
        &json!({ "package": package.display().to_string(), "folder": folder.display().to_string() }),
    )
    .map_err(|e| {
        or_instead(
            e,
            format!(
                "Bu bağlantı paketi kuruluma açamaz: KentOS'ta Eklentiler › Paket yükle… ile {} dosyasını açın ya da \
                 ajanı masaüstüne bağlayın (desktop.attach).",
                package.display()
            ),
        )
    })
}

/// The changes of the extension `id` the user installed from `folder` loaded on the desktop, with the capabilities the
/// user granted.
pub fn link_reload(host: &mut dyn Host, id: &str, folder: &Path) -> Result<Value, HeadlessError> {
    host.request(
        "reload_extension",
        &json!({ "id": id, "folder": folder.display().to_string() }),
    )
    .map_err(|e| {
        or_instead(
            e,
            "Bu bağlantı eklentiyi canlı yükleyemez: ajanı KentOS masaüstüne bağlayın (desktop.attach) ya da \
             masaüstünün Asistan'ında çalışın."
                .to_owned(),
        )
    })
}

/// What the user sees of the extension `id` on the desktop: its tab, panels, tools and menus, in words.
pub fn link_preview(host: &mut dyn Host, id: &str) -> Result<Value, HeadlessError> {
    host.request("preview_extension", &json!({ "id": id }))
        .map_err(|e| {
            or_instead(
                e,
                "Bu bağlantı eklentinin görünüşünü veremez: ajanı KentOS masaüstüne bağlayın (desktop.attach) ya da \
                 masaüstünün Asistan'ında çalışın."
                    .to_owned(),
            )
        })
}
