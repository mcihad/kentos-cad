//! What each panel component does, in one table (plan/eklenti.md ADIM 22, "özellik/olay/eylem/erişilebilirlik
//! şeması"; docs/adr/8057 §71): the value a command gets of it in `ctx.params`, what starts the extension's code from
//! it, what the running code may change of it (`ctx.panel.set`, `rows`, `focus`), how the keyboard uses it and what
//! names it.
//!
//! One source: KentOS's panel session refuses what this table does not give, and a desktop test measures its keyboard
//! column on the panel KentOS draws (Tab reaches what holds a focusable widget; typing, what holds a text input).

use serde::Serialize;

/// What the running code changes of a component with `ctx.panel.set`; every component also takes `visible`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Prop {
    /// Its value, as the command gets it.
    Value,
    /// A choice's options.
    Options,
    /// A text's words.
    Text,
    /// A picture (base64 PNG).
    Image,
    /// Whether the user can use it.
    Enabled,
}

impl Prop {
    /// Its name in `ctx.panel.set`.
    pub fn name(self) -> &'static str {
        match self {
            Prop::Value => "value",
            Prop::Options => "options",
            Prop::Text => "text",
            Prop::Image => "image",
            Prop::Enabled => "enabled",
        }
    }
}

/// How the keyboard reaches and uses a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Keys {
    /// Tab reaches it, typing writes in it, Enter presses the panel's primary button (or applies a date that runs a
    /// command on change).
    Type,
    /// Tab reaches it; Space, Enter or F4 opens its list, ↑ and ↓ step its value.
    Choose,
    /// Tab reaches it; Space or Enter turns it on or off (a chip of a multiple choice each, a section's heading folds).
    Toggle,
    /// Tab reaches its buttons; Space or Enter presses the one focused.
    Press,
    /// Tab reaches its rows as one stop; ↑ and ↓ move its cursor, the page following; Enter or Space shows the cursor
    /// row's object.
    Rows,
    /// Only the mouse uses it: Tab passes it by.
    Pointer,
    /// Nothing to use: it is shown.
    Shown,
}

/// One component type's behaviour.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Behaviour {
    /// Its `type`.
    #[serde(rename = "type")]
    pub kind: &'static str,
    /// Its value in the command's `ctx.params`, by its id; none when it holds no value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<&'static str>,
    /// A value of it as JSON, as `ctx.panel.set(id, value=…)` gives it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<&'static str>,
    /// What starts the extension's code from it: `onChange` (the user changed it), `command` (pressed).
    pub events: &'static [&'static str],
    /// What `ctx.panel.set` changes of it besides `visible`.
    pub sets: &'static [Prop],
    /// Whether `ctx.panel.rows` fills it.
    pub rows: bool,
    /// Whether `ctx.panel.focus` puts the caret in it.
    pub focus: bool,
    /// How the keyboard uses it.
    pub keys: Keys,
    /// What names it when KentOS shows or speaks of it (an error, the halo's place): `label` or `text`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub named_by: Option<&'static str>,
    /// What KentOS does with it besides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<&'static str>,
}

const CHANGE: &[&str] = &["onChange"];
const NONE: &[&str] = &[];
const TYPED: &[Prop] = &[Prop::Value, Prop::Enabled];
const CHOICE: &[Prop] = &[Prop::Value, Prop::Options];
const VALUE: &[Prop] = &[Prop::Value];

/// A typed field's behaviour: its value, an example, its events.
const fn typed(
    kind: &'static str,
    value: &'static str,
    example: &'static str,
    events: &'static [&'static str],
) -> Behaviour {
    Behaviour {
        kind,
        value: Some(value),
        example: Some(example),
        events,
        sets: TYPED,
        rows: false,
        focus: true,
        keys: Keys::Type,
        named_by: Some("label"),
        note: None,
    }
}

/// A component holding a value, used by `keys`.
const fn holds(
    kind: &'static str,
    value: &'static str,
    example: &'static str,
    sets: &'static [Prop],
    keys: Keys,
) -> Behaviour {
    Behaviour {
        kind,
        value: Some(value),
        example: Some(example),
        events: CHANGE,
        sets,
        rows: false,
        focus: false,
        keys,
        named_by: Some("label"),
        note: None,
    }
}

/// A component showing rows the code gives (`ctx.panel.rows`): a row with a `uid` shows its object when clicked.
const fn rows(kind: &'static str) -> Behaviour {
    Behaviour {
        kind,
        value: None,
        example: None,
        events: NONE,
        sets: &[],
        rows: true,
        focus: false,
        keys: Keys::Pointer,
        named_by: Some("label"),
        note: Some("uid'i olan satıra tıklamak o nesneyi gösterir."),
    }
}

/// A component that is shown only, named by `named_by`.
const fn shown(kind: &'static str, named_by: Option<&'static str>) -> Behaviour {
    Behaviour {
        kind,
        value: None,
        example: None,
        events: NONE,
        sets: &[],
        rows: false,
        focus: false,
        keys: Keys::Shown,
        named_by,
        note: None,
    }
}

/// Every component type's behaviour, in Kapı 1's order (`component_types`).
pub const TABLE: [Behaviour; 28] = [
    Behaviour {
        sets: &[Prop::Text],
        ..shown("text", Some("text"))
    },
    Behaviour {
        keys: Keys::Toggle,
        note: Some(
            "Başlığına tıklamak sonraki bileşenleri katlar; katlıyken değerleri yine komuta gider.",
        ),
        ..shown("section", Some("label"))
    },
    shown("divider", None),
    typed("textField", "metin (boşsa \"\")", "\"Y-01\"", NONE),
    typed(
        "numberField",
        "sayı, alanın biriminde (unit); boşsa None",
        "12.5",
        NONE,
    ),
    typed(
        "dateField",
        "tarih \"YYYY-AA-GG\"; boşsa None",
        "\"2026-11-16\"",
        CHANGE,
    ),
    typed(
        "timeField",
        "saat \"SS:DD\"; boşsa None",
        "\"14:30\"",
        CHANGE,
    ),
    typed(
        "dateTimeField",
        "an \"YYYY-AA-GGTSS:DD\" (yerel); boşsa None",
        "\"2026-11-16T14:30\"",
        CHANGE,
    ),
    holds(
        "select",
        "seçilen seçeneğin değeri (seçilmemişse \"\")",
        "\"a\"",
        CHOICE,
        Keys::Choose,
    ),
    holds(
        "multiSelect",
        "seçilenlerin değerleri, seçeneklerin sırasıyla (liste)",
        "[\"a\"]",
        CHOICE,
        Keys::Toggle,
    ),
    holds(
        "layerPicker",
        "katmanın kimliği; seçilmemişse None",
        "\"bina\"",
        VALUE,
        Keys::Choose,
    ),
    holds(
        "fieldPicker",
        "bağlı katmanın bir alanının adı; seçilmemişse None",
        "\"Ad\"",
        VALUE,
        Keys::Choose,
    ),
    Behaviour {
        note: Some("Çizimde seçilenleri Seçimi al düğmesiyle alır."),
        ..holds(
            "featurePicker",
            "nesnenin kalıcı kimliği (multiple ise kimliklerin listesi); alınmamışsa None",
            "\"0192f5a1-25a0-7000-8000-000000000301\"",
            VALUE,
            Keys::Press,
        )
    },
    Behaviour {
        sets: TYPED,
        focus: true,
        ..holds(
            "filePicker",
            "dosyanın tam yolu; verilmemişse None",
            "\"/home/ayse/ruhsat.csv\"",
            VALUE,
            Keys::Type,
        )
    },
    holds(
        "crsPicker",
        "\"EPSG:<kod>\"; seçilmemişse None",
        "\"EPSG:5256\"",
        VALUE,
        Keys::Choose,
    ),
    Behaviour {
        sets: TYPED,
        focus: true,
        note: Some("Haritadan seç noktayı haritada aldırır, kenetlenir."),
        ..holds(
            "pointPicker",
            "[x, y], çizimin biriminde; boşsa None",
            "[500123.45, 4401234.5]",
            VALUE,
            Keys::Type,
        )
    },
    Behaviour {
        note: Some(
            "Haritada çiz şekli haritada çizdirir; panel gösterildikçe şekil haritada durur.",
        ),
        ..holds(
            "shapePicker",
            "GeoJSON geometri (LineString ya da Polygon); çizilmemişse None",
            "{\"type\": \"Polygon\", \"coordinates\": [[[0, 0], [10, 0], [10, 10], [0, 0]]]}",
            VALUE,
            Keys::Press,
        )
    },
    holds(
        "relationPicker",
        "üst katmandaki kaydın kalıcı kimliği; seçilmemişse None",
        "\"0192f5a1-25a0-7000-8000-000000000301\"",
        VALUE,
        Keys::Choose,
    ),
    holds("checkbox", "true ya da false", "true", VALUE, Keys::Toggle),
    Behaviour {
        events: &["command"],
        sets: &[Prop::Enabled],
        keys: Keys::Press,
        note: Some("Birincil düğmeye, metin, sayı, tarih ve saat alanında Enter da basar."),
        ..shown("button", Some("label"))
    },
    Behaviour {
        keys: Keys::Rows,
        ..rows("table")
    },
    rows("tree"),
    Behaviour {
        keys: Keys::Rows,
        ..rows("card")
    },
    rows("stats"),
    Behaviour {
        keys: Keys::Shown,
        note: Some("Satırları rows'tan ya da of ile başka bir bileşenden okur."),
        ..rows("chart")
    },
    Behaviour {
        sets: &[Prop::Image],
        ..shown("image", Some("label"))
    },
    Behaviour {
        keys: Keys::Rows,
        note: Some("Çizimden okunur, Python'suz; satıra tıklamak kaydı gösterir."),
        ..shown("relationList", Some("label"))
    },
    Behaviour {
        keys: Keys::Press,
        note: Some("Aç eki açar; Dosya ekle… ve Adres ekle… KentOS'un, katmanı kilitli değilse."),
        ..shown("attachments", Some("label"))
    },
];

/// The behaviour of component type `kind`; none for a type Kapı 1 does not know.
pub fn of(kind: &str) -> Option<&'static Behaviour> {
    TABLE.iter().find(|b| b.kind == kind)
}

/// The types `ctx.panel.set` changes `prop` of, in Kapı 1's order.
pub fn takers(prop: Prop) -> Vec<&'static str> {
    TABLE
        .iter()
        .filter(|b| b.sets.contains(&prop))
        .map(|b| b.kind)
        .collect()
}

impl Behaviour {
    /// Whether `ctx.panel.set` changes `prop` of it.
    pub fn takes(&self, prop: Prop) -> bool {
        self.sets.contains(&prop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table is Kapı 1's types, each once in its order; what a component starts is what its definition takes
    /// (`onChange` where the type has it, `command` on the button); an example is a value of the component.
    #[test]
    fn the_table_is_every_component_type_once() {
        let kinds: Vec<&str> = TABLE.iter().map(|b| b.kind).collect();
        assert_eq!(kinds, super::super::component_types().collect::<Vec<_>>());
        for b in &TABLE {
            assert_eq!(b.value.is_some(), b.example.is_some(), "{}", b.kind);
            assert_eq!(b.value.is_some(), b.takes(Prop::Value), "{}", b.kind);
            if let Some(example) = b.example {
                serde_json::from_str::<serde_json::Value>(example)
                    .unwrap_or_else(|e| panic!("{}: {e}", b.kind));
            }
            assert_eq!(
                b.focus,
                b.keys == Keys::Type,
                "the caret goes where typing does: {}",
                b.kind
            );
            assert!(!(b.rows && b.value.is_some()), "{}", b.kind);
        }
        assert_eq!(takers(Prop::Options), ["select", "multiSelect"]);
        assert_eq!(takers(Prop::Image), ["image"]);
        assert_eq!(takers(Prop::Text), ["text"]);
    }
}
