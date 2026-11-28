//! A value most objects leave at its default, held apart from them (plan/render.md RM-19, docs/adr/9236).
//!
//! An object's fields lie inside it: a field most objects never set still costs every object its whole size, and an
//! enum of objects is as large as its largest kind. A block reference's `ArchInfo` is 512 bytes and empty on nearly
//! every insert of a survey drawing. `Lean<T>` holds such a value behind a pointer, absent while it is the default:
//! an object without one carries eight bytes.
//!
//! It reads as the value itself: a field of it is read through it, and an absent value reads as the default. Writing
//! through it makes the value present. Its JSON, its TypeScript and its schema are the value's own; a default value
//! read from a file is held absent.

use std::fmt;
use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A value whose default every `Lean` of it shares while absent.
pub trait LeanDefault: Default + PartialEq + 'static {
    /// The default, made once.
    fn shared_default() -> &'static Self;
}

/// A value held behind a pointer, absent while it is the default (see the module).
#[derive(Clone, Default)]
pub struct Lean<T>(Option<Box<T>>);

impl<T: LeanDefault> Lean<T> {
    /// Whether the value is the default.
    pub fn is_default(&self) -> bool {
        self.0.as_deref().is_none_or(|v| *v == T::default())
    }

    /// The value itself.
    pub fn into_inner(self) -> T {
        self.0.map_or_else(T::default, |v| *v)
    }
}

impl<T: LeanDefault> From<T> for Lean<T> {
    fn from(value: T) -> Self {
        if value == T::default() {
            Self(None)
        } else {
            Self(Some(Box::new(value)))
        }
    }
}

impl<T: LeanDefault> Deref for Lean<T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self.0.as_deref() {
            Some(value) => value,
            None => T::shared_default(),
        }
    }
}

impl<T: LeanDefault> DerefMut for Lean<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0.get_or_insert_with(Default::default)
    }
}

impl<T: LeanDefault> PartialEq for Lean<T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T: LeanDefault + fmt::Debug> fmt::Debug for Lean<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: LeanDefault + Serialize> Serialize for Lean<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (**self).serialize(serializer)
    }
}

impl<'de, T: LeanDefault + Deserialize<'de>> Deserialize<'de> for Lean<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self::from)
    }
}

#[cfg(feature = "schema")]
impl<T: LeanDefault + schemars::JsonSchema> schemars::JsonSchema for Lean<T> {
    fn inline_schema() -> bool {
        T::inline_schema()
    }

    fn schema_name() -> std::borrow::Cow<'static, str> {
        T::schema_name()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        T::schema_id()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        T::json_schema(generator)
    }
}

// Its TypeScript is the value's own, as a `Box`'s is (ts-rs's wrappers): a field of it, flattened or not, writes what a
// field of the value wrote.
#[cfg(feature = "ts")]
impl<T: LeanDefault + ts_rs::TS> ts_rs::TS for Lean<T> {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name(cfg: &ts_rs::Config) -> String {
        T::name(cfg)
    }

    fn inline(cfg: &ts_rs::Config) -> String {
        T::inline(cfg)
    }

    fn inline_flattened(cfg: &ts_rs::Config) -> String {
        T::inline_flattened(cfg)
    }

    fn visit_dependencies(v: &mut impl ts_rs::TypeVisitor)
    where
        Self: 'static,
    {
        T::visit_dependencies(v);
    }

    fn visit_generics(v: &mut impl ts_rs::TypeVisitor)
    where
        Self: 'static,
    {
        T::visit_generics(v);
        v.visit::<T>();
    }

    fn decl(_: &ts_rs::Config) -> String {
        panic!("a wrapper is not declared")
    }

    fn decl_concrete(_: &ts_rs::Config) -> String {
        panic!("a wrapper is not declared")
    }
}

/// `LeanDefault` for a type: its default made once, at its first read.
#[macro_export]
macro_rules! lean_default {
    ($($t:ty),* $(,)?) => {
        $(
            impl $crate::lean::LeanDefault for $t {
                fn shared_default() -> &'static Self {
                    static DEFAULT: std::sync::LazyLock<$t> = std::sync::LazyLock::new(<$t>::default);
                    &DEFAULT
                }
            }
        )*
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
    struct Info {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        level: Option<String>,
    }

    lean_default!(Info);

    #[test]
    fn a_default_value_is_held_absent_and_reads_as_the_default() {
        let empty: Lean<Info> = Info::default().into();
        assert!(empty.0.is_none() && empty.is_default());
        assert_eq!(empty.level, None);
        assert_eq!(
            std::mem::size_of::<Lean<Info>>(),
            std::mem::size_of::<usize>()
        );
        let mut set = Lean::from(Info::default());
        set.level = Some("Z".into());
        assert!(set.0.is_some() && !set.is_default());
        assert_eq!(
            *set,
            Info {
                level: Some("Z".into())
            }
        );
        assert_ne!(set, empty);
    }

    #[test]
    fn its_json_is_the_value_s_own() {
        let set: Lean<Info> = Info {
            level: Some("Z".into()),
        }
        .into();
        assert_eq!(serde_json::to_string(&set).unwrap(), r#"{"level":"Z"}"#);
        assert_eq!(
            serde_json::to_string(&Lean::<Info>::default()).unwrap(),
            "{}"
        );
        let read: Lean<Info> = serde_json::from_str("{}").unwrap();
        assert!(read.0.is_none());
        let read: Lean<Info> = serde_json::from_str(r#"{"level":"Z"}"#).unwrap();
        assert_eq!(read, set);
    }
}
