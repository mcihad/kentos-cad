//! Eklenti ifade işlevleri (plan/eklenti.md ADIM 19; docs/adr/8057 §8): `ext.<eklenti>.<işlev>(…)`, a function an
//! installed extension gives. The compiler knows it by name from the schema (`Schema::externals`), without the
//! extension's code; the host answers it a batch of objects at a time (`ExternalCalls`), so that a run of 10 000
//! objects asks forty times, not 10 000. Without a host's answers (a preview, a host without extensions) it is empty.
//!
//!   ext.tr.ornek.mebis.mezar-etiketi(mezar_no, durum)
//!
//! The name is written whole: after `ext.` a dot and a hyphen belong to it (an extension's id and its function's local
//! id have them), anywhere else they stay operators.

/// A function the host offers: its whole name and one line for the builder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalDef {
    /// `ext.<extension>.<local>`.
    pub name: String,
    pub description: String,
}

/// An argument or an answer: what JSON carries between the engine and an extension's code.
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    Null,
    Num(f64),
    Text(String),
    Bool(bool),
}

/// The host's answers to a batch of calls.
pub trait ExternalCalls {
    /// `name`'s answers for each row of arguments, in their order; a missing answer is empty.
    fn call(&self, name: &str, rows: &[Vec<Arg>]) -> Vec<Arg>;
}

/// Whether a name is an extension's function's (`ext.` and more).
pub fn is_external(name: &str) -> bool {
    name.len() > 4 && name.starts_with("ext.")
}

impl Arg {
    /// A value the engine holds, as an argument.
    pub fn of(v: crate::scalar::V<'_>) -> Arg {
        use crate::scalar::V;
        match v {
            V::Null => Arg::Null,
            V::Num(x) => Arg::Num(x),
            V::Text(t) => Arg::Text(t.to_owned()),
            V::Bool(b) => Arg::Bool(b),
        }
    }

    /// An answer as the engine holds it.
    pub fn value(self) -> crate::Value<'static> {
        use crate::Value;
        match self {
            Arg::Null => Value::Null,
            Arg::Num(x) if x.is_finite() => Value::Num(x),
            Arg::Num(_) => Value::Null,
            Arg::Text(t) => Value::Text(std::borrow::Cow::Owned(t)),
            Arg::Bool(b) => Value::Bool(b),
        }
    }
}

/// A batch's calls of one function (`Ins::External`): each object's arguments read from the columns, the host asked
/// once for the batch, each answer written by the rules; an object one of whose arguments threw is thrown, without a
/// host every answer is empty.
pub(crate) fn column(
    name: &str,
    cols: &[crate::exec::Col<'_, '_>],
    host: Option<&dyn ExternalCalls>,
    out: &mut crate::exec::Out<'_, '_>,
) {
    use crate::exec::{NULL, THROWN};
    use crate::scalar::{R, V};
    let Some(host) = host else {
        out.k.fill(NULL);
        return;
    };
    let n = out.k.len();
    let mut rows: Vec<Vec<Arg>> = Vec::with_capacity(n);
    let mut asked: Vec<usize> = Vec::with_capacity(n);
    for (i, kind) in out.k.iter_mut().enumerate() {
        let row: Option<Vec<Arg>> = cols.iter().map(|c| c.view(i).map(Arg::of)).collect();
        match row {
            Some(row) => {
                rows.push(row);
                asked.push(i);
            }
            None => *kind = THROWN,
        }
    }
    let answers = if rows.is_empty() {
        Vec::new()
    } else {
        host.call(name, &rows)
    };
    let mut answers = answers.into_iter();
    for i in asked {
        let answer = answers.next().unwrap_or(Arg::Null);
        let mark = out.made.len();
        match answer {
            Arg::Null => out.put(i, R::V(V::Null), mark),
            Arg::Num(x) if x.is_finite() => out.put(i, R::V(V::Num(x)), mark),
            Arg::Num(_) => out.put(i, R::V(V::Null), mark),
            Arg::Bool(b) => out.put(i, R::V(V::Bool(b)), mark),
            Arg::Text(t) => out.put(i, R::V(V::Text(&t)), mark),
        }
    }
}

/// A source with the host's answers to the extensions' functions, as `world::WithWorld` adds a world.
pub struct WithExternals<'s, 'a> {
    pub inner: &'s dyn crate::exec::Source<'a>,
    pub externals: &'s dyn ExternalCalls,
}

impl<'a> crate::exec::Source<'a> for WithExternals<'_, 'a> {
    fn fill(&self, load: crate::program::Load, start: usize, slot: crate::exec::Slot<'_, 'a>) {
        self.inner.fill(load, start, slot);
    }

    fn world(&self) -> Option<&dyn crate::world::WorldCalls> {
        self.inner.world()
    }

    fn externals(&self) -> Option<&dyn ExternalCalls> {
        Some(self.externals)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::lexer::{Tok, tokenize};
    use crate::rows::{As, Layout, RowsInput, TEXT, Table, TableSource, column, copy};
    use crate::{Schema, compile_with};

    const NAME: &str = "ext.tr.ornek.imar-denetim.iki-kat";

    /// Doubles a number, marks a text; counts its batches.
    #[derive(Default)]
    struct Doubler {
        batches: RefCell<Vec<usize>>,
    }

    impl ExternalCalls for Doubler {
        fn call(&self, name: &str, rows: &[Vec<Arg>]) -> Vec<Arg> {
            assert_eq!(name, NAME);
            self.batches.borrow_mut().push(rows.len());
            rows.iter()
                .map(|r| match r.first() {
                    Some(Arg::Num(x)) => Arg::Num(x * 2.0),
                    Some(Arg::Text(t)) => Arg::Text(format!("{t}{t}")),
                    _ => Arg::Null,
                })
                .collect()
        }
    }

    fn schema() -> Schema {
        Schema {
            externals: vec![ExternalDef {
                name: NAME.to_owned(),
                description: "İki katı".to_owned(),
            }],
            ..Schema::default()
        }
    }

    #[test]
    fn an_extensions_name_is_read_whole_and_only_after_ext() {
        let toks = tokenize("ext.tr.ornek.imar-denetim.iki-kat(Ad) - a-b").unwrap();
        assert_eq!(toks[0].t, Tok::Word(NAME.to_owned()));
        // Elsewhere a hyphen is still a minus, and `ext` alone a field's name.
        assert!(toks.iter().any(|t| t.t == Tok::Op("-")));
        let toks = tokenize("ext - 1").unwrap();
        assert_eq!(toks[0].t, Tok::Word("ext".to_owned()));
    }

    #[test]
    fn a_function_the_host_does_not_offer_is_refused_where_it_is_called() {
        let Err(e) = compile_with("1 + ext.tr.ornek.yok.islev(2)", &schema()) else {
            panic!("refused");
        };
        assert!(
            e.message.contains("ext.tr.ornek.yok.islev()"),
            "{}",
            e.message
        );
        assert_eq!(e.at, 5);
        let Err(e) = compile_with(&format!("{NAME}(Ad)"), &Schema::default()) else {
            panic!("refused");
        };
        assert!(e.message.contains("kurulu ve açık bir eklentinin"));
        let e = compile_with(&format!("{NAME}(Ad) || '!'"), &schema()).unwrap();
        assert_eq!(e.externals(), [NAME.to_owned()]);
    }

    #[test]
    fn a_batch_asks_the_host_once_and_writes_its_answers() {
        let e = compile_with(&format!("{NAME}(Ad) || '!'"), &schema()).unwrap();
        // Three objects: their text field Ad ("ab", "c", none).
        let input = RowsInput {
            n: 3,
            texts: "abc",
            text_lens: &[2, 1, -1],
            numbers: &[],
            measures: &[],
            scale: f64::NAN,
        };
        let table = Table::new(copy(&input), Layout::of(&e)).unwrap();
        let rows = TableSource {
            table: &table,
            slots: None,
        };
        let host = Doubler::default();
        let source = WithExternals {
            inner: &rows,
            externals: &host,
        };
        let c = column(&e, &source, 3, As::Value);
        assert_eq!(c.kinds, [TEXT, TEXT, TEXT]);
        assert_eq!(c.texts, "abab!cc!!");
        assert_eq!(*host.batches.borrow(), [3], "one call for the batch");
        // Without the host's answers: empty.
        let c = column(&e, &rows, 3, As::Value);
        assert_eq!(c.texts, "!!!");
    }

    #[test]
    fn one_object_is_asked_as_a_row() {
        struct One<'h>(&'h Doubler);
        impl crate::Scope for One<'_> {
            fn field(&self, _: usize) -> Option<&str> {
                Some("x")
            }
            fn measured(&self) -> crate::Measured {
                crate::Measured::default()
            }
            fn vertices(&self) -> Option<f64> {
                None
            }
            fn kind(&self) -> &str {
                "point"
            }
            fn layer(&self) -> &str {
                "L"
            }
            fn label(&self) -> Option<&str> {
                None
            }
            fn index(&self) -> f64 {
                1.0
            }
            fn id(&self) -> f64 {
                1.0
            }
            fn scale(&self) -> Option<f64> {
                None
            }
            fn externals(&self) -> Option<&dyn ExternalCalls> {
                Some(self.0)
            }
        }
        let e = compile_with(&format!("{NAME}(21)"), &schema()).unwrap();
        let host = Doubler::default();
        assert_eq!(e.evaluate(&One(&host)), crate::Value::Num(42.0));
    }
}
