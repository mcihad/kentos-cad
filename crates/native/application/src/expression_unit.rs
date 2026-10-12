//! The unit an expression's geometry values speak where they are readings
//! (docs/adr/0165 §2, the 12 Ekim addendum): a label's text and its class's
//! condition, the information card and the actions, a layer's filter,
//! Öznitelik tablosu's filter and the builder over them read lengths,
//! coordinates and areas in the project's drawing unit, as Öznitelikler
//! shows them. İşlemler, a network's costs and the symbols keep metres, as
//! their own numbers do. The web's is `readingUnit` (`model/projectSettings.ts`).

use kentos_contracts::ProjectSettings;
pub use kentos_expression::Unit;

/// The project's drawing unit, for an expression whose values are readings.
pub fn reading_unit(settings: &ProjectSettings) -> Unit {
    Unit::per_metre(settings.unit().per_metre())
}
