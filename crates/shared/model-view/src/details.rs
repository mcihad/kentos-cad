//! What the properties panel shows of the chosen object (docs/adr/8061 §5 "Özellikler", docs/adr/8073 §2): groups of
//! named values, as the source writes them. An adapter fills them when the host asks (an IFC file's attributes,
//! property sets and quantity sets: kentos-ifc's `view` feature); the viewer only lays them out.

/// The chosen object's details: its groups in the order the source gives them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Details {
    pub groups: Vec<Group>,
}

/// A group (the object's own attributes, a property set, a quantity set): its title and its lines.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Group {
    pub title: String,
    pub lines: Vec<Line>,
}

/// A named value as the source writes it, its unit beside it, and whether it is a number (set in figures).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Line {
    pub name: String,
    pub value: String,
    pub unit: Option<String>,
    pub numeric: bool,
}
