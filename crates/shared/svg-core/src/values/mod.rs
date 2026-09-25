//! The values of an SVG file as the importer reads them
//! (`apps/web/src/style/svg/svgValues.ts`): colours (every CSS syntax and
//! keyword, with alpha), paints (the symbol's colours, references),
//! transforms, lengths with units, viewBox mapping with
//! preserveAspectRatio, and `<style>` sheets with the selectors icon files
//! use (class, id, tag, attribute, descendant and child). Text rules are
//! JavaScript's (`\s`, `trim`, `Number`, `parseFloat`), as the TypeScript's
//! regular expressions read them.
//!
//! One module per kind of value: `number` (JavaScript's reading of numbers
//! and lists), `color` (colours and paints), `units` (transforms, lengths,
//! viewBox), `xml` (the file's element list), `css` (style sheets).

mod color;
mod css;
mod number;
#[cfg(test)]
mod tests;
mod units;
mod xml;

pub use color::{
    Color, Flat, RawPaint, clamp01, hex2, is_near_black, raw_paint, read_color, read_paint,
    with_alpha,
};
pub use css::{Compound, Decl, PROPS, Rule, Selector, matches, parse_css, parse_decls};
pub use number::{js_number, nums, parse_float, parse_hex, split_on, split_ws, starts_with_digit};
pub use units::{
    Length, parse_length, px_per, read_transform, to_user, view_box_of, view_box_transform,
};
pub use xml::{XmlNode, XmlTree};
