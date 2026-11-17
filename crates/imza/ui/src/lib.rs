//! KentOS İmza'nın imza penceresi (docs/adr/9002 §6): **ajan** (portalın tarayıcıdan geldiği yolda PIN'i alan ayrı
//! süreç) ve **masaüstü** (kartla süreç içinde konuşan KentOS) aynı pencereyi gösterir.
//!
//! * [`model`] — pencerenin gösterdiği: istek, belgeler, sertifika seçenekleri, aşama, sonuç (Iced bilmez),
//! * [`dialog`] — durum ve kararlar (Iced bilmez): sertifika seçimi, maskeli PIN, aşamanın ilerlemesi,
//! * `view` — çizim, yalnız KentOS UI bileşenleriyle; [`SigningDialog::view_window`] (kendi penceresi) ya da
//!   [`SigningDialog::view_modal`] (bir pencerenin üstünde),
//! * [`flow`] — canlı akışın Türkçe satırları, boyut ve özet yazımı,
//! * [`convert`] — alan modelinden görünüm modeline.
//!
//! Pencere kartla KONUŞMAZ: istediği işi ([`Action`]) ev sahibine verir. Ajan onu yerel servise, masaüstü süreç içi
//! motora götürür; ikisinin olayları aynı adlarla pencereye döner ([`SigningDialog::event`]).

pub mod convert;
pub mod dialog;
pub mod flow;
pub mod model;
mod view;

#[cfg(test)]
mod dialog_tests;

pub use dialog::{Action, Msg, PIN_MIN, SigningDialog};
pub use model::{CertificateOption, DocState, Document, Link, Notice, Outcome, Phase, Request};
pub use view::{PIN_FIELD, TITLE, WIDTH, focus_pin};
