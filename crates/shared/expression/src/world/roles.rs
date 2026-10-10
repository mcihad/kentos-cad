//! What each argument of a function that looks at other objects is
//! (docs/adr/0214 §3): a layer's name or a constant text, an expression
//! computed on the other objects (`ifade`, `grup`, `koşul`), or a value of
//! the object being evaluated (`uzaklık`, `değer`).

use crate::library::Func;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The layer's name: a constant text.
    Layer,
    /// A constant text: katman_toplamı's operation, katmandan's field,
    /// değerleri_birleştir's separator.
    Text,
    /// The value computed on each of the other objects.
    Value,
    /// The group: the other objects whose group equals the object's.
    Group,
    /// The condition the other objects must meet.
    Condition,
    /// A value of the object being evaluated.
    Current,
}

/// The roles of a function's arguments in order; None for a function that
/// looks at no other object.
pub fn roles(f: Func) -> Option<&'static [Role]> {
    use Role::*;
    Some(match f {
        Func::Sum
        | Func::Mean
        | Func::Count
        | Func::CountDistinct
        | Func::Minimum
        | Func::Maximum
        | Func::Median
        | Func::StdDev
        | Func::ArrayAgg => &[Value, Group, Condition],
        Func::ConcatValues => &[Value, Text, Group, Condition],
        Func::Aggregate => &[Layer, Text, Value, Condition],
        Func::FromLayer => &[Layer, Value, Text, Current],
        Func::Intersects | Func::IntersectCount => &[Layer, Current, Condition],
        Func::Encloses
        | Func::Inside
        | Func::CenterIn
        | Func::Distance
        | Func::OverlapArea
        | Func::OverlapLength => &[Layer, Condition],
        Func::Intersecting => &[Layer, Value, Current, Condition],
        Func::Nearest => &[Layer, Value, Condition],
        _ => return None,
    })
}

/// Whether an argument is an expression over the other objects.
pub fn is_inner(r: Role) -> bool {
    matches!(r, Role::Value | Role::Group | Role::Condition)
}
