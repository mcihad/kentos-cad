//! The builder's tree and help (docs/adr/0100 §5): its groups in order
//! (Alanlar ve değerler, Değişkenler, Dönüşümler, Geometri, İşleçler,
//! Koşullar, Matematik, Metin), the entry each name is, its help
//! (signature, description, arguments, examples, other names), a field's
//! values as the builder lists and inserts them, and a value as the
//! preview shows it.

use super::complete::{by_label, signature};
use super::lex::{Lex, called, lex};
use super::{Item, Kind};
use crate::js::collate::compare_tr;
use crate::js::text::{fold_turkish, trim, utf16_len};
use crate::library::{
    FUNCTIONS, FuncDef, Group, OPERATORS, OpDef, VARIABLES, VarDef, find_function, find_variable,
};
use crate::parser::keyword_of;
use crate::value::number_text;
use crate::{FieldDef, FieldSource, FieldType, Schema, Value, read};

/// A type as the builder names it.
pub(crate) fn type_name(ty: FieldType) -> &'static str {
    match ty {
        FieldType::Text => "metin",
        FieldType::Number => "sayı",
        FieldType::Bool => "doğru/yanlış",
        FieldType::Date => "tarih",
    }
}

fn source_name(source: FieldSource) -> &'static str {
    match source {
        FieldSource::User => "kullanıcı alanı",
        FieldSource::Attribute => "öznitelik",
        FieldSource::Builtin => "yerleşik",
    }
}

/// The first clause of a description: what the list shows beside a name.
fn short(description: &str) -> &str {
    let end = [": ", "; "]
        .iter()
        .filter_map(|s| description.find(s))
        .min()
        .unwrap_or(description.len());
    &description[..end]
}

/// Whether a field's name can stand bare in an expression: a word of the
/// language that is no keyword (else it goes in brackets).
fn bare(name: &str) -> bool {
    let u: Vec<u16> = name.encode_utf16().collect();
    let pieces = lex(&u);
    matches!(pieces.as_slice(), [p] if matches!(&p.lex, Lex::Word { name: n } if n == name))
        && keyword_of(name).is_none()
}

/// How an expression names a field: bare, or in brackets.
fn reference(name: &str) -> String {
    if bare(name) {
        name.to_string()
    } else {
        format!("[{name}]")
    }
}

pub(crate) fn field_item(f: &FieldDef) -> Item {
    let insert = reference(&f.name);
    Item {
        kind: Kind::Field,
        label: f.name.clone(),
        detail: format!("{} · {}", type_name(f.ty), source_name(f.source)),
        caret: utf16_len(&insert),
        insert,
        key: format!("field:{}", f.name),
        alias: None,
    }
}

pub(crate) fn variable_item(v: &VarDef) -> Item {
    let insert = format!("${}", v.name);
    Item {
        kind: Kind::Variable,
        label: insert.clone(),
        detail: short(v.description).to_string(),
        caret: utf16_len(&insert),
        insert,
        key: format!("var:{}", v.name),
        alias: None,
    }
}

/// A function; `call` writes the parentheses too, with the cursor between them.
pub(crate) fn function_item(f: &FuncDef, call: bool) -> Item {
    let (insert, caret) = if call {
        (format!("{}()", f.name), utf16_len(f.name) + 1)
    } else {
        (f.name.to_string(), utf16_len(f.name))
    };
    Item {
        kind: Kind::Function,
        label: f.name.to_string(),
        detail: f.signature.to_string(),
        insert,
        caret,
        key: format!("func:{}", f.name),
        alias: None,
    }
}

/// An operator or a word of the language; `spaced` (the tree) puts a space
/// either side, as one writes them between values (`place` drops a space
/// already there); completion replaces the word being written with it bare.
pub(crate) fn operator_item(o: &OpDef, spaced: bool) -> Item {
    let word = o.symbol.chars().all(char::is_alphabetic);
    let insert = if spaced {
        format!(" {} ", o.symbol)
    } else {
        o.symbol.to_string()
    };
    Item {
        kind: if word { Kind::Keyword } else { Kind::Operator },
        label: o.symbol.to_string(),
        detail: short(o.description).to_string(),
        caret: utf16_len(&insert),
        insert,
        key: format!("op:{}", o.symbol),
        alias: None,
    }
}

/// A group of the builder's tree and its entries.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub group: Group,
    pub title: &'static str,
    pub items: Vec<Item>,
}

/// Whether an entry's names hold the folded query.
fn found(item: &Item, aliases: &[&str], query: &str) -> bool {
    query.is_empty()
        || fold_turkish(&item.label).contains(query)
        || aliases.iter().any(|a| fold_turkish(a).contains(query))
}

/// The builder's tree for objects with these fields: the groups in order,
/// each entry once, those whose name (or other name) holds `query` (case
/// and Turkish letters aside); a group with no entry is left out. Fields
/// keep the host's order, operators the language's; the rest are in
/// Turkish alphabetical order.
pub fn catalog(schema: &Schema, query: &str) -> Vec<Section> {
    let q = fold_turkish(query);
    Group::ALL
        .iter()
        .filter_map(|&group| {
            let mut items: Vec<Item> = Vec::new();
            match group {
                Group::Fields => {
                    items.extend(
                        schema
                            .fields
                            .iter()
                            .map(field_item)
                            .filter(|i| found(i, &[], &q)),
                    );
                }
                Group::Operators => {
                    items.extend(
                        OPERATORS
                            .iter()
                            .map(|o| (operator_item(o, true), o.aliases))
                            .filter(|(i, a)| found(i, a, &q))
                            .map(|(i, _)| i),
                    );
                }
                _ => {
                    items.extend(
                        VARIABLES
                            .iter()
                            .filter(|v| v.group == group)
                            .map(|v| (variable_item(v), v.aliases))
                            .chain(
                                FUNCTIONS
                                    .iter()
                                    .filter(|f| f.group == group)
                                    .map(|f| (function_item(f, true), f.aliases)),
                            )
                            .filter(|(i, a)| found(i, a, &q))
                            .map(|(i, _)| i),
                    );
                    items.sort_by(by_label);
                }
            }
            (!items.is_empty()).then(|| Section {
                group,
                title: group.title(),
                items,
            })
        })
        .collect()
}

/// An argument as the help lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct Arg {
    pub name: String,
    pub description: String,
    /// Whether the call may leave it out.
    pub optional: bool,
}

/// An entry's help: the builder's right pane.
#[derive(Clone, Debug, PartialEq)]
pub struct Help {
    pub key: String,
    pub kind: Kind,
    /// `yuvarla`, `$alan`, `Tapu alanı`, `ve`.
    pub title: String,
    /// The group's title.
    pub group: &'static str,
    /// How it is written: `yuvarla(sayı, basamak)`, `$alan`, `[Tapu alanı]`, `a ve b`.
    pub signature: String,
    pub description: String,
    pub args: Vec<Arg>,
    /// Expressions and what they give.
    pub examples: Vec<(String, String)>,
    /// Its other names (English first as the table lists them), without the
    /// ones that differ only in Turkish letters (names match without them).
    pub aliases: Vec<String>,
    /// A field's type and where its values come from.
    pub field: Option<(FieldType, FieldSource)>,
}

/// The other names worth showing: those that are not the name itself
/// written without Turkish letters.
fn other_names(name: &str, aliases: &[&str], prefix: &str) -> Vec<String> {
    let own = fold_turkish(name);
    aliases
        .iter()
        .filter(|a| fold_turkish(a) != own)
        .map(|a| format!("{prefix}{a}"))
        .collect()
}

/// A function's description without the example the menus show after it
/// (“…: yuvarla(12.345, 2) → 12.35”): the help lists its examples apart.
fn plain(f: &FuncDef) -> &'static str {
    let example = format!(": {}(", f.name);
    match f.description.find(&example) {
        Some(end) => &f.description[..end],
        None => f.description,
    }
}

fn function_help(f: &FuncDef) -> Help {
    Help {
        key: format!("func:{}", f.name),
        kind: Kind::Function,
        title: f.name.to_string(),
        group: f.group.title(),
        signature: f.signature.to_string(),
        description: plain(f).to_string(),
        args: f
            .args
            .iter()
            .enumerate()
            .map(|(i, (name, description))| Arg {
                name: (*name).to_string(),
                description: (*description).to_string(),
                optional: i >= f.arity.0,
            })
            .collect(),
        examples: pairs(f.examples),
        aliases: other_names(f.name, f.aliases, ""),
        field: None,
    }
}

fn variable_help(v: &VarDef) -> Help {
    Help {
        key: format!("var:{}", v.name),
        kind: Kind::Variable,
        title: format!("${}", v.name),
        group: v.group.title(),
        signature: format!("${}", v.name),
        description: v.description.to_string(),
        args: Vec::new(),
        examples: pairs(v.examples),
        aliases: other_names(v.name, v.aliases, "$"),
        field: None,
    }
}

fn operator_help(o: &OpDef) -> Help {
    let item = operator_item(o, true);
    Help {
        key: item.key,
        kind: item.kind,
        title: o.symbol.to_string(),
        group: Group::Operators.title(),
        signature: o.signature.to_string(),
        description: o.description.to_string(),
        args: Vec::new(),
        examples: pairs(o.examples),
        aliases: o.aliases.iter().map(|a| (*a).to_string()).collect(),
        field: None,
    }
}

/// A field's help; `known` is false for a name the objects do not have.
fn field_help(name: &str, def: Option<&FieldDef>, doubted: bool) -> Help {
    let r = reference(name);
    let (ty, source) = def.map_or((FieldType::Text, FieldSource::Attribute), |f| {
        (f.ty, f.source)
    });
    let description = match def {
        Some(f) if !f.description.is_empty() => f.description.clone(),
        Some(f) if f.source == FieldSource::User => format!(
            "Kullanıcının tanımladığı alan; değerleri {} olarak okunur.",
            type_name(f.ty)
        ),
        _ if doubted => format!("Bu nesnelerde “{name}” alanı yok; değeri boş okunur."),
        _ => "Nesnelerin özniteliği: değeri metindir, hesapta sayıya çevrilir; olmayan nesnede boştur.".to_string(),
    };
    let examples = match ty {
        FieldType::Number => vec![
            (format!("{r} > 100"), "100'den büyükse doğru".to_string()),
            (format!("yuvarla({r}, 2)"), "iki ondalıkla".to_string()),
        ],
        FieldType::Bool => vec![(
            format!("{r} = doğru"),
            "doğru olanlar için doğru".to_string(),
        )],
        FieldType::Date => vec![(
            format!("{r} >= '2026-01-01'"),
            "2026'dan bu yana olanlar için doğru".to_string(),
        )],
        FieldType::Text => vec![
            (
                format!("boş({r})"),
                "değeri olmayanlar için doğru".to_string(),
            ),
            (format!("büyük({r})"), "değeri büyük harflerle".to_string()),
        ],
    };
    Help {
        key: format!("field:{name}"),
        kind: Kind::Field,
        title: name.to_string(),
        group: Group::Fields.title(),
        signature: r,
        description,
        args: Vec::new(),
        examples,
        aliases: Vec::new(),
        field: Some((ty, source)),
    }
}

fn pairs(examples: &[(&str, &str)]) -> Vec<(String, String)> {
    examples
        .iter()
        .map(|(e, r)| ((*e).to_string(), (*r).to_string()))
        .collect()
}

/// The help of an entry by its key (`Item::key`): `func:yuvarla`,
/// `var:alan`, `op:ve`, `field:Tapu alanı`.
pub fn help(key: &str, schema: &Schema) -> Option<Help> {
    let (kind, name) = key.split_once(':')?;
    match kind {
        "func" => FUNCTIONS.iter().find(|f| f.name == name).map(function_help),
        "var" => VARIABLES.iter().find(|v| v.name == name).map(variable_help),
        "op" => OPERATORS
            .iter()
            .find(|o| o.symbol == name)
            .map(operator_help),
        "field" => {
            let def = schema.find(name);
            Some(field_help(
                name,
                def,
                def.is_none() && !schema.fields.is_empty(),
            ))
        }
        _ => None,
    }
}

/// The help of what is at the cursor: the name or operator it is in or
/// just after, else the call it is in.
pub fn help_at(src: &str, cursor: usize, schema: &Schema) -> Option<Help> {
    let u: Vec<u16> = src.encode_utf16().collect();
    let pieces = lex(&u);
    let at = pieces
        .iter()
        .position(|p| p.start < cursor && cursor <= p.end)
        .or_else(|| pieces.iter().position(|p| p.start == cursor));
    let own = at.and_then(|i| {
        let p = &pieces[i];
        match &p.lex {
            Lex::Word { name } if called(&pieces, i) => find_function(name).map(function_help),
            Lex::Word { name } => match keyword_of(name) {
                Some(_) => OPERATORS
                    .iter()
                    .find(|o| {
                        std::iter::once(o.symbol)
                            .chain(o.aliases.iter().copied())
                            .any(|n| fold_turkish(n) == fold_turkish(name))
                    })
                    .map(operator_help),
                None => help(&format!("field:{name}"), schema),
            },
            Lex::Field { name, .. } if !name.is_empty() => help(&format!("field:{name}"), schema),
            Lex::Var { name } => find_variable(name).map(variable_help),
            Lex::Op(op) if !matches!(*op, "(" | ")" | ",") => OPERATORS
                .iter()
                .find(|o| o.symbol == *op || o.aliases.contains(op))
                .map(operator_help),
            _ => None,
        }
    });
    own.or_else(|| {
        let s = signature(src, cursor)?;
        help(&s.key, schema)
    })
}

/// A value of a field as the builder lists it and inserts it.
#[derive(Clone, Debug, PartialEq)]
pub struct ValueItem {
    pub text: String,
    pub insert: String,
}

/// Text in quotes, a quote inside doubled.
fn quoted(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// A field's value as an expression writes it: a number field's numbers
/// bare, true/false as doğru/yanlış, anything else as quoted text.
pub fn literal(value: &str, ty: FieldType) -> String {
    match ty {
        FieldType::Number if read::number(trim(value)).is_some() => trim(value).to_string(),
        FieldType::Bool => match fold_turkish(value).as_str() {
            "DOGRU" | "TRUE" | "1" => "doğru".into(),
            "YANLIS" | "FALSE" | "0" => "yanlış".into(),
            _ => quoted(value),
        },
        _ => quoted(value),
    }
}

/// A field's distinct values in the order the builder lists them: numbers
/// by value when every value is one, else in Turkish alphabetical order;
/// empty values left out.
pub fn values<S: AsRef<str>>(raw: &[S], ty: FieldType) -> Vec<ValueItem> {
    let mut list: Vec<&str> = raw
        .iter()
        .map(AsRef::as_ref)
        .filter(|v| !trim(v).is_empty())
        .collect();
    list.sort_unstable();
    list.dedup();
    let numbers: Option<Vec<f64>> = list.iter().map(|v| read::number(trim(v))).collect();
    let mut order: Vec<usize> = (0..list.len()).collect();
    match numbers {
        Some(n) => order.sort_by(|&a, &b| {
            n[a].total_cmp(&n[b])
                .then_with(|| compare_tr(list[a], list[b]))
        }),
        None => order.sort_by(|&a, &b| compare_tr(list[a], list[b])),
    }
    order
        .into_iter()
        .map(|i| ValueItem {
            text: list[i].to_string(),
            insert: literal(list[i], ty),
        })
        .collect()
}

/// A value as the builder's preview shows it: text in quotes, numbers as
/// the language writes them, doğru/yanlış, and boş for no value.
pub fn preview(v: &Value) -> String {
    match v {
        Value::Null => "boş".into(),
        Value::Bool(b) => if *b { "doğru" } else { "yanlış" }.into(),
        Value::Num(x) => number_text(*x),
        Value::Text(t) => format!("'{t}'"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> Schema {
        Schema {
            fields: vec![
                FieldDef {
                    name: "Parsel".into(),
                    ty: FieldType::Text,
                    source: FieldSource::Attribute,
                    description: String::new(),
                },
                FieldDef {
                    name: "Tapu alanı".into(),
                    ty: FieldType::Number,
                    source: FieldSource::User,
                    description: String::new(),
                },
            ],
        }
    }

    #[test]
    fn the_tree_has_the_groups_in_order_and_every_entry_once() {
        let tree = catalog(&schema(), "");
        let titles: Vec<&str> = tree.iter().map(|s| s.title).collect();
        assert_eq!(
            titles,
            [
                "Alanlar ve değerler",
                "Değişkenler",
                "Dönüşümler",
                "Geometri",
                "İşleçler",
                "Koşullar",
                "Matematik",
                "Metin"
            ]
        );
        let count: usize = tree.iter().map(|s| s.items.len()).sum();
        assert_eq!(
            count,
            2 + VARIABLES.len() + FUNCTIONS.len() + OPERATORS.len()
        );
        assert_eq!(tree[0].items[1].insert, "[Tapu alanı]");
        // A search keeps the groups with a match; English names count.
        let found = catalog(&schema(), "round");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].items[0].label, "yuvarla");
        let found: Vec<String> = catalog(&schema(), "uzun")
            .iter()
            .flat_map(|s| s.items.iter().map(|i| i.label.clone()))
            .collect();
        assert_eq!(found, ["$uzunluk", "uzunluk"]);
        assert!(
            catalog(&Schema::default(), "")
                .iter()
                .all(|s| s.group != Group::Fields)
        );
    }

    #[test]
    fn help_covers_every_entry_and_what_is_under_the_cursor() {
        let s = schema();
        for section in catalog(&s, "") {
            for item in section.items {
                let h = help(&item.key, &s).expect("every entry has help");
                assert!(
                    !h.description.is_empty() && !h.examples.is_empty(),
                    "{}",
                    item.key
                );
            }
        }
        let h = help("func:parça", &s).expect("parça");
        assert_eq!(h.aliases, ["substr"]);
        assert_eq!(
            h.args.iter().map(|a| a.optional).collect::<Vec<_>>(),
            [false, false, true]
        );
        assert_eq!(
            help_at("yuvarla($alan, 2)", 3, &s).map(|h| h.title),
            Some("yuvarla".into())
        );
        assert_eq!(
            help_at("yuvarla($alan, 2)", 10, &s).map(|h| h.title),
            Some("$alan".into())
        );
        assert_eq!(
            help_at("yuvarla($alan, 2)", 16, &s).map(|h| h.title),
            Some("yuvarla".into())
        );
        assert_eq!(
            help_at("a AND b", 4, &s).map(|h| h.title),
            Some("ve".into())
        );
        assert_eq!(help_at("a <> b", 3, &s).map(|h| h.title), Some("!=".into()));
        let h = help_at("[Tapu alanı] > 1", 2, &s).expect("field");
        assert_eq!(
            (h.signature.as_str(), h.field),
            ("[Tapu alanı]", Some((FieldType::Number, FieldSource::User)))
        );
        let h = help_at("Pafta", 2, &s).expect("unknown field");
        assert_eq!(
            h.description,
            "Bu nesnelerde “Pafta” alanı yok; değeri boş okunur."
        );
    }

    /// The parcel the help's examples speak of: 20 × 30 m, Ada 1245, Parsel 12, an Arsa.
    struct Parcel<'e>(&'e [String]);

    impl crate::Scope for Parcel<'_> {
        fn field(&self, i: usize) -> Option<&str> {
            match self.0.get(i)?.as_str() {
                "Ada" => Some("1245"),
                "Parsel" => Some("12"),
                "Nitelik" => Some("Arsa"),
                "Kat" => Some("3"),
                _ => None,
            }
        }
        fn measured(&self) -> crate::Measured {
            crate::Measured {
                length: Some(100.0),
                area: Some(600.0),
                anchor: Some((500_010.0, 4_400_015.0)),
            }
        }
        fn vertices(&self) -> Option<f64> {
            Some(4.0)
        }
        fn kind(&self) -> &str {
            "Kapalı alan"
        }
        fn layer(&self) -> &str {
            "Parsel sınırı"
        }
        fn label(&self) -> Option<&str> {
            Some("12")
        }
        fn index(&self) -> f64 {
            1.0
        }
        fn id(&self) -> f64 {
            7.0
        }
        fn scale(&self) -> Option<f64> {
            None
        }
        fn geometry(&self, what: crate::Geometry) -> Option<f64> {
            match what {
                crate::Geometry::Width => Some(20.0),
                crate::Geometry::Height => Some(30.0),
                crate::Geometry::Length => Some(100.0),
                crate::Geometry::Area => Some(600.0),
                crate::Geometry::Vertices => Some(4.0),
                _ => None,
            }
        }
    }

    /// The value an example's result names (before a remark in parentheses),
    /// when it names one: a number, one quoted text, doğru, yanlış or boş.
    fn stated(result: &str) -> Option<&str> {
        let r = result.split(" (").next().unwrap_or(result);
        let text = r.len() >= 2 && r.starts_with('\'') && r.ends_with('\'') && !r.contains("', '");
        (text || read::number(r).is_some() || matches!(r, "doğru" | "yanlış" | "boş")).then_some(r)
    }

    #[test]
    fn the_examples_give_what_the_help_says() {
        let all = FUNCTIONS
            .iter()
            .flat_map(|f| f.examples.iter())
            .chain(VARIABLES.iter().flat_map(|v| v.examples.iter()))
            .chain(OPERATORS.iter().flat_map(|o| o.examples.iter()));
        let mut checked = 0;
        for (expression, result) in all {
            let Ok(e) = crate::compile(expression) else {
                panic!("the example {expression} does not compile");
            };
            let Some(want) = stated(result) else {
                continue;
            };
            let parcel = Parcel(&e.fields);
            let v = e.evaluate(&parcel);
            assert_eq!(preview(&v), want, "{expression}");
            checked += 1;
        }
        assert!(checked >= 35, "{checked}");
    }

    #[test]
    fn values_are_listed_in_order_and_inserted_as_the_language_writes_them() {
        let v = values(&["12", "2", "", "12", "100"], FieldType::Text);
        assert_eq!(
            v.iter()
                .map(|v| (v.text.as_str(), v.insert.as_str()))
                .collect::<Vec<_>>(),
            [("2", "'2'"), ("12", "'12'"), ("100", "'100'")]
        );
        let v = values(&["Çayır", "Arsa", "bağ", "O'Neil"], FieldType::Text);
        assert_eq!(
            v.iter().map(|v| v.insert.as_str()).collect::<Vec<_>>(),
            ["'Arsa'", "'bağ'", "'Çayır'", "'O''Neil'"]
        );
        assert_eq!(literal("452.5", FieldType::Number), "452.5");
        assert_eq!(literal("true", FieldType::Bool), "doğru");
        assert_eq!(preview(&Value::Num(0.1 + 0.2)), "0.3");
        assert_eq!(preview(&Value::text("Arsa".to_string())), "'Arsa'");
    }
}
