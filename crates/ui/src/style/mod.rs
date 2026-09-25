//! iced bileşenleri için stil fonksiyonları.
//!
//! Hepsi iced'in kendi imzalarını kullanır; kentos-ui bileşenlerinin
//! dışında da doğrudan verilebilir:
//!
//! ```ignore
//! button("Kaydet").style(kentos_ui::style::button::primary)
//! ```
//!
//! Renkler o anki temadan [`Tokens::of`](crate::theme::Tokens::of) ile
//! okunur.

pub mod button;
pub mod container;
pub mod field;
pub mod text;
