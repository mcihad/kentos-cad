//! The unit an expression's geometry values speak where they are readings
//! (docs/adr/0165 §2, the 12 and 13 Ekim addenda): a label's text and its
//! class's condition, the information card and the actions, a layer's filter,
//! Öznitelik tablosu's filter and the builder over them, and a layer's
//! renderer (a category's or a class's value, a rule's condition, a thematic
//! renderer's values: Katman stili's lists and counts, the drawing area, a
//! sheet's map and the PDF) read lengths, coordinates and areas in the
//! project's drawing unit, as Öznitelikler shows them. İşlemler, a network's
//! costs and a symbol's own expressions (its data-defined sizes, distances,
//! colours and texts, the geometry generator) keep metres: their numbers are
//! metres, or a portable symbol's own units. The web's is `readingUnit`
//! (`model/projectSettings.ts`).

use kentos_commands::ProjectSettings;
pub use kentos_expression::Unit;

/// The project's drawing unit, for an expression whose values are readings.
pub fn reading_unit(settings: &ProjectSettings) -> Unit {
    Unit::per_metre(settings.unit().per_metre())
}
