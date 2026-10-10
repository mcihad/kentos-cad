//! What the flow's values are, as hints (docs/adr/0101): the language
//! converts between numbers, text and conditions, so a port of another type
//! still takes a value; the port says how it will be read.

use super::tree::T;
use crate::library::{Family, Func, Var};
use crate::parser::BinOp;
use crate::world::roles::{Role, roles};
use crate::{FieldType, Schema};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    /// Anything: a condition's branches, `=`'s sides.
    Any,
    Number,
    Text,
    /// doğru or yanlış.
    Bool,
}

impl Type {
    pub fn id(self) -> &'static str {
        match self {
            Type::Any => "any",
            Type::Number => "number",
            Type::Text => "text",
            Type::Bool => "bool",
        }
    }

    /// As a port's hint reads.
    pub fn name(self) -> &'static str {
        match self {
            Type::Any => "değer",
            Type::Number => "sayı",
            Type::Text => "metin",
            Type::Bool => "koşul",
        }
    }
}

/// What function `f`'s argument `i` expects.
pub(crate) fn takes(f: Func, i: usize) -> Type {
    use Type::{Any, Bool, Number as N, Text};
    match f {
        Func::Round
        | Func::Int
        | Func::Abs
        | Func::Min
        | Func::Max
        | Func::Sqrt
        | Func::Ceil
        | Func::Floor
        | Func::Ln
        | Func::Log10
        | Func::Log
        | Func::Exp
        | Func::Sin
        | Func::Cos
        | Func::Tan
        | Func::Asin
        | Func::Acos
        | Func::Atan
        | Func::Atan2
        | Func::Degrees
        | Func::Radians => N,
        Func::Text => [Any, N][i.min(1)],
        Func::Number => Any,
        Func::Upper | Func::Lower | Func::Trim | Func::Length => Text,
        Func::Substr => [Text, N, N][i.min(2)],
        Func::Pad | Func::PadEnd => [Any, N, Text][i.min(2)],
        Func::Replace | Func::Contains | Func::Starts | Func::Ends | Func::Find => Text,
        Func::If => [Bool, Any, Any][i.min(2)],
        Func::Empty | Func::Coalesce | Func::Concat => Any,
        Func::Left | Func::Right => [Text, N][i.min(1)],
        Func::Pi => Any,
        // docs/adr/0214.
        Func::Date | Func::DateTime => Any,
        Func::Year
        | Func::Month
        | Func::Day
        | Func::Hour
        | Func::Minute
        | Func::Second
        | Func::Week
        | Func::Weekday
        | Func::DayOfYear
        | Func::Matches
        | Func::RegexFind
        | Func::RegexPart
        | Func::RegexGroups
        | Func::RegexReplace
        | Func::TextToArray
        | Func::FromJson => Text,
        Func::DateAdd | Func::DateDiff => {
            [Text, if f == Func::DateAdd { N } else { Text }, Text][i.min(2)]
        }
        Func::DateFormat => Text,
        Func::ArrayGet => [Any, N][i.min(1)],
        Func::ArraySlice => [Any, N, N][i.min(2)],
        Func::ArraySort => [Any, Bool][i.min(1)],
        f if f.family() == Family::World => match roles(f).and_then(|r| r.get(i)) {
            Some(Role::Layer | Role::Text) => Text,
            Some(Role::Condition) => Bool,
            Some(Role::Current) if f != Func::FromLayer => N,
            _ => Any,
        },
        _ => Any,
    }
}

/// What function `f` gives.
pub(crate) fn gives(f: Func) -> Type {
    match f {
        Func::Round
        | Func::Number
        | Func::Int
        | Func::Abs
        | Func::Min
        | Func::Max
        | Func::Length
        | Func::Sqrt
        | Func::Ceil
        | Func::Floor
        | Func::Pi
        | Func::Find
        | Func::Ln
        | Func::Log10
        | Func::Log
        | Func::Exp
        | Func::Sin
        | Func::Cos
        | Func::Tan
        | Func::Asin
        | Func::Acos
        | Func::Atan
        | Func::Atan2
        | Func::Degrees
        | Func::Radians => Type::Number,
        Func::Text
        | Func::Upper
        | Func::Lower
        | Func::Trim
        | Func::Substr
        | Func::Pad
        | Func::Replace
        | Func::Left
        | Func::Right
        | Func::Concat
        | Func::PadEnd => Type::Text,
        Func::Contains | Func::Starts | Func::Ends | Func::Empty => Type::Bool,
        Func::If | Func::Coalesce => Type::Any,
        // docs/adr/0214.
        Func::Year
        | Func::Month
        | Func::Day
        | Func::Hour
        | Func::Minute
        | Func::Second
        | Func::Week
        | Func::Weekday
        | Func::DayOfYear
        | Func::DateDiff
        | Func::RegexFind
        | Func::ArrayLength
        | Func::ArrayFind
        | Func::ArraySum
        | Func::ArrayMean
        | Func::Sum
        | Func::Mean
        | Func::Count
        | Func::CountDistinct
        | Func::Median
        | Func::StdDev
        | Func::IntersectCount
        | Func::Distance
        | Func::OverlapArea
        | Func::OverlapLength => Type::Number,
        Func::Date
        | Func::DateTime
        | Func::Now
        | Func::Today
        | Func::DateAdd
        | Func::DateFormat
        | Func::RegexPart
        | Func::RegexReplace
        | Func::ArrayToText
        | Func::ToJson
        | Func::ConcatValues => Type::Text,
        Func::Matches
        | Func::ArrayContains
        | Func::MapHas
        | Func::Intersects
        | Func::Encloses
        | Func::Inside
        | Func::CenterIn => Type::Bool,
        _ => Type::Any,
    }
}

/// What a `$` value gives.
fn variable(v: Var) -> Type {
    match v {
        Var::Kind | Var::Layer | Var::Label => Type::Text,
        _ => Type::Number,
    }
}

/// What a node gives.
pub(crate) fn of(t: &T, schema: &Schema) -> Type {
    match t {
        T::Hole | T::Null => Type::Any,
        T::Num(_) | T::Neg(_) => Type::Number,
        T::Text(_) => Type::Text,
        T::Bool(_) | T::Not(_) | T::In(..) | T::Between(..) | T::Like(..) | T::IsNull(..) => {
            Type::Bool
        }
        T::Field(name) => match schema.find(name).map(|f| f.ty) {
            Some(FieldType::Number) => Type::Number,
            Some(FieldType::Bool) => Type::Bool,
            // Attributes are text; dates are ISO text.
            _ => Type::Text,
        },
        T::Var(v) => variable(*v),
        T::At(_) => Type::Any,
        T::Call(f, _) => gives(*f),
        T::Bin(op, ..) => match op {
            BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem | BinOp::Pow => Type::Number,
            BinOp::Join => Type::Text,
            // + adds numbers and joins anything else.
            BinOp::Add => Type::Any,
            _ => Type::Bool,
        },
        T::Case(whens, otherwise) => {
            let mut values = whens
                .iter()
                .map(|(_, v)| v)
                .chain(otherwise.as_deref())
                .filter(|v| !matches!(v, T::Hole | T::Null))
                .map(|v| of(v, schema));
            match values.next() {
                Some(first) if values.all(|t| t == first) => first,
                _ => Type::Any,
            }
        }
    }
}

/// What an operator's sides expect.
pub(crate) fn operand(op: BinOp) -> Type {
    match op {
        BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem | BinOp::Pow => Type::Number,
        BinOp::And | BinOp::Or => Type::Bool,
        _ => Type::Any,
    }
}

/// How a value of type `given` is read by a port expecting `wanted`, when
/// that is worth saying.
pub(crate) fn note(given: Type, wanted: Type) -> Option<String> {
    Some(
        match (given, wanted) {
            (Type::Text, Type::Number) => "Metin sayıya çevrilir; sayı değilse boş olur.",
            (Type::Bool, Type::Number) => "doğru 1, yanlış 0 sayılır.",
            (Type::Number, Type::Text) => "Sayı metin olarak yazılır.",
            (Type::Bool, Type::Text) => "“doğru” ya da “yanlış” olarak yazılır.",
            (Type::Number, Type::Bool) => "Koşul olarak 0 yanlış, öbür sayılar doğru sayılır.",
            (Type::Text, Type::Bool) => "Koşul olarak boş metin yanlış, öbürü doğru sayılır.",
            _ => return None,
        }
        .to_string(),
    )
}
