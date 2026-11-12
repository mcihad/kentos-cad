//! The network a project's addressed file is read through (docs/adr/0645 §5): a point cloud's, a raster's or a NetCDF
//! file's address by HTTP ranges, a Zarr store's objects by their addresses. The application layer stays without a
//! network: a host that reaches one (the desktop, the MCP server, the Python module) registers its reader once at
//! start; a host that registers none answers that an address is not read there.

use std::sync::OnceLock;

pub use kentos_services::remote::{RangeRead, Remote};

static REMOTE: OnceLock<Box<dyn Remote>> = OnceLock::new();

/// Registers the host's network; the first registration stays.
pub fn register(remote: Box<dyn Remote>) {
    let _ = REMOTE.set(remote);
}

/// The host's network, when it registered one.
pub fn remote() -> Option<&'static dyn Remote> {
    REMOTE.get().map(|b| b.as_ref())
}

/// Why an address is not read: this host reaches no network.
pub const NO_NETWORK: &str =
    "Bu ortam ağa çıkmıyor; adresteki dosya okunmaz. Dosyayı indirip bağlı dosya olarak ekleyin.";
