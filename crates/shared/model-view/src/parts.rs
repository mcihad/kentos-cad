//! The scene's parts and their colours (docs/adr/8067 §4, docs/adr/8073 §2): one a category, then what is chosen and
//! what a check found, a body's and a see-through volume's (a space, an opening). Linear RGB written to the screen as it is (the program draws with the
//! web's colours); alpha below 1 is see-through.

use crate::input::Category;

/// A part's index in [`PARTS`].
pub type Part = u32;

/// The parts: their names (the legend's words) and colours.
pub const PARTS: [(&str, [f32; 4]); 18] = [
    ("Duvar", [0.80, 0.78, 0.74, 1.0]),
    ("Döşeme", [0.60, 0.61, 0.64, 1.0]),
    ("Kolon", [0.52, 0.55, 0.60, 1.0]),
    ("Kiriş", [0.47, 0.50, 0.56, 1.0]),
    ("Çatı", [0.62, 0.32, 0.24, 1.0]),
    ("Merdiven", [0.72, 0.66, 0.55, 1.0]),
    ("Korkuluk", [0.33, 0.35, 0.38, 1.0]),
    ("Kapı", [0.52, 0.35, 0.20, 1.0]),
    ("Pencere", [0.55, 0.74, 0.88, 0.55]),
    ("Mahal", [0.30, 0.56, 0.86, 0.16]),
    ("Donatı", [0.27, 0.56, 0.52, 1.0]),
    ("Seçili", [1.0, 0.62, 0.05, 1.0]),
    ("Bulgu", [0.86, 0.10, 0.08, 1.0]),
    ("Seçili mahal", [1.0, 0.62, 0.05, 0.38]),
    ("Bulgulu mahal", [0.86, 0.10, 0.08, 0.42]),
    ("Diğer", [0.66, 0.66, 0.62, 1.0]),
    ("Açıklık", [0.62, 0.36, 0.78, 0.22]),
    // Another discipline's model beside the drawing's (docs/adr/8101, 8119): violet, a colour no class has, see-through
    // so the building's own bodies stay readable through it.
    ("Altlık model", [0.56, 0.42, 0.80, 0.55]),
];

pub const SPACE: Part = 9;
pub const SELECTED: Part = 11;
pub const FOUND: Part = 12;
pub const SELECTED_SPACE: Part = 13;
pub const FOUND_SPACE: Part = 14;
pub const OPENING: Part = 16;
pub const UNDERLAY: Part = 17;

/// A category's part.
pub fn of(category: Category) -> Part {
    match category {
        Category::Wall => 0,
        Category::Slab => 1,
        Category::Column => 2,
        Category::Beam => 3,
        Category::Roof => 4,
        Category::Stair => 5,
        Category::Railing => 6,
        Category::Door => 7,
        Category::Window => 8,
        Category::Space => SPACE,
        Category::Furnishing => 10,
        Category::Opening => OPENING,
        Category::Project | Category::Site | Category::Building | Category::Storey | Category::Other => 15,
    }
}
