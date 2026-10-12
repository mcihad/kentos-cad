//! Adres (docs/adr/0229 §11, §12): Adresleri eşle and Konumlara adres yaz,
//! over the project's address data or an online provider of the device's
//! (`crate::address`: the sources, what a found address is and writes).

pub mod geocode;
pub mod reverse;

use crate::types::{ParamDef, ParamKind};

/// The address source parameter.
pub fn source_param() -> ParamDef {
    ParamDef::new("via", "Adres kaynağı", ParamKind::AddressSource).describe(
        "Projenin adres verisi (Adres verisi penceresinde tanımlanır; veri cihazdan çıkmaz) ya da Adres sağlayıcıları'nda açılmış bir çevrimiçi sağlayıcı (adresler onun sunucusuna gider).",
    )
}
