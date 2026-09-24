//! Öznitelik veri modeli: türlü değerler, alanlar ve sorgular.
//!
//! - [`Value`]: tek bir öznitelik değeri (metin, sayı, evet/hayır, tarih,
//!   saat, nesne başvurusu ya da boş).
//! - [`Field`]: bir öznitelik sütunu; türü ([`FieldKind`]), birimi ve
//!   kısıtlarıyla. ArcGIS'teki alan türleri ve etki alanlarına (kodlu değer,
//!   aralık) karşılık gelir.
//! - [`Query`]: "öznitelikle seç" ve tablo filtrelerindeki koşullar.
//! - [`Date`], [`Time`], [`DateTime`]: harici bağımlılık olmadan takvim hesabı.
//!
//! Metinler Türkçe kurallarıyla karşılaştırılır ([`text`]): arama büyük/küçük
//! harf ve Türkçe harf duyarsızdır, sıralama Türk alfabesine göredir. Sayılar
//! Türkçe yazılır ([`number`]): 15.840.900 ve 1.234,5.
//!
//! Bu modül arayüzden bağımsızdır; sunucu tarafında da kullanılabilir.

pub mod field;
pub mod number;
pub mod query;
pub mod text;
pub mod time;
pub mod value;

pub use field::{Field, FieldKind};
pub use query::{Combinator, Condition, Operator, Query};
pub use time::{Date, DateTime, Time};
pub use value::{ObjectId, Value};
