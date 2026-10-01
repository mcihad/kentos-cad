//! The ribbon's rules apart from its widgets (the web's
//! `ui/ribbon/ribbonPlan.ts`, docs/specs/ribbon.md): the quick access bar's
//! menu and how a command is added or taken off, what a right click offers
//! on each kind of item, a split button's list and its face. The kept bar
//! and split choices are layout_plan.rs's (`quick_access_of`,
//! `split_current`). Both platforms play fixtures/shell/v1/ribbon.json.

use crate::layout_plan::QUICK_ACCESS;

/// The ribbon's words (the web's `RIBBON_TEXTS`).
pub(crate) mod texts {
    pub(crate) const QUICK_ACCESS: &str = "Hızlı erişim";
    pub(crate) const CUSTOMIZE: &str = "Hızlı erişimi özelleştir";
    pub(crate) const CUSTOMIZE_TIP: &str =
        "Şeritteki bir düğmeye sağ tıklayarak da ekleyebilirsiniz.";
    pub(crate) const FIXED_HINT: &str = "sabit";
    pub(crate) const ADD: &str = "Hızlı erişime ekle";
    pub(crate) const REMOVE: &str = "Hızlı erişimden kaldır";
    pub(crate) const FIXED: &str = "Hızlı erişimde (sabit)";
    // The folded ribbon's words and the panel's ▾ are said by the key tips'
    // slice (TODOS UX-13); they are held to the web's now.
    #[cfg(test)]
    pub(crate) const FOLD: &str = "Şeridi daralt";
    #[cfg(test)]
    pub(crate) const PIN: &str = "Şeridi sabitle";
    #[cfg(test)]
    pub(crate) const FOLD_TIP: &str =
        "Daraltılmış şerit bir sekmeye tıklayınca çizimin üstünde açılır.";

    /// A split button's arrow.
    pub(crate) fn others(title: &str) -> String {
        format!("{title}: diğer seçenekler")
    }

    /// A tool's method (`Daire: 3 nokta`).
    pub(crate) fn method(title: &str, label: &str) -> String {
        format!("{title}: {label}")
    }

    /// A panel's ▾.
    pub(crate) fn panel_more(panel: &str) -> String {
        format!("{panel}: diğer araçlar")
    }

    /// A split entry whose tool did not take its method.
    pub(crate) fn cannot_start(title: &str, label: &str) -> String {
        format!("“{title}: {label}” şu an başlatılamadı.")
    }
}

/// Offered in the quick access bar's menu besides what the user added from
/// the ribbon, in this order.
pub(crate) const QUICK_ACCESS_OFFERS: [&str; 9] = [
    "file.new",
    "file.open",
    "file.saveAs",
    "edit.paste",
    "view.zoomExtents",
    "tool.zoomWindow",
    "tool.pan",
    "tools.options",
    "view.theme.toggle",
];

/// Folds the ribbon: the last row of its menus (a command with its own check).
pub(crate) const FOLD_COMMAND: &str = "view.ribbonCollapse";

/// A row of a ribbon menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Row {
    Header(&'static str),
    Separator,
    /// A command's own row, as the menus draw it (its title, shortcut and
    /// check; off while the command is).
    Command(&'static str),
    /// A quick access row about a command.
    Quick(Quick),
}

/// A quick access row: `label` its own words, else the command's title;
/// `set`, what choosing it makes of the command's place on the bar (true
/// adds, false takes off; none: nothing).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Quick {
    pub command: String,
    pub label: Option<&'static str>,
    pub icon: Option<&'static str>,
    pub checked: Option<bool>,
    pub disabled: bool,
    pub hint: Option<&'static str>,
    pub set: Option<bool>,
}

fn fixed(id: &str) -> bool {
    QUICK_ACCESS.contains(&id)
}

/// The kept list after a command is put on the bar (at its end) or taken off.
pub(crate) fn with_quick_access(kept: &[String], id: &str, on: bool) -> Vec<String> {
    let mut rest: Vec<String> = kept.iter().filter(|x| *x != id).cloned().collect();
    if on {
        rest.push(id.to_owned());
    }
    rest
}

/// The bar's ▾ menu: its title, then every command on the bar and every
/// offer (once each, the bar's first, then the offers' order; only commands
/// this app has), checked while on the bar; the fixed three are off and say
/// so. Then the ribbon's fold.
pub(crate) fn quick_access_menu(bar: &[String], exists: impl Fn(&str) -> bool) -> Vec<Row> {
    let mut offers: Vec<String> = Vec::new();
    for id in bar
        .iter()
        .map(String::as_str)
        .chain(QUICK_ACCESS_OFFERS.iter().copied())
    {
        if !offers.iter().any(|x| x == id) {
            offers.push(id.to_owned());
        }
    }
    let mut rows = vec![Row::Header(texts::QUICK_ACCESS)];
    for id in offers.into_iter().filter(|id| exists(id)) {
        let on = bar.contains(&id);
        let fixed = fixed(&id);
        rows.push(Row::Quick(Quick {
            checked: Some(on),
            disabled: fixed,
            hint: fixed.then_some(texts::FIXED_HINT),
            set: (!fixed).then_some(!on),
            command: id,
            ..Quick::default()
        }));
    }
    rows.push(Row::Separator);
    rows.push(Row::Command(FOLD_COMMAND));
    rows
}

/// A right click on a command's button (on a panel, on the bar, the top of
/// a split button or its arrow): the bar, then the fold.
pub(crate) fn command_menu(id: &str, bar: &[String]) -> Vec<Row> {
    let first = if fixed(id) {
        Quick {
            label: Some(texts::FIXED),
            icon: Some("pin"),
            disabled: true,
            ..Quick::default()
        }
    } else if bar.iter().any(|x| x == id) {
        Quick {
            label: Some(texts::REMOVE),
            icon: Some("close"),
            set: Some(false),
            ..Quick::default()
        }
    } else {
        Quick {
            label: Some(texts::ADD),
            icon: Some("pin"),
            set: Some(true),
            ..Quick::default()
        }
    };
    vec![
        Row::Quick(Quick {
            command: id.to_owned(),
            ..first
        }),
        Row::Separator,
        Row::Command(FOLD_COMMAND),
    ]
}

/// A right click anywhere else on the ribbon (a tab, a menu button, a
/// panel's title, the bar's ▾, empty space): the fold. A text field keeps
/// its own.
pub(crate) fn ribbon_menu() -> Vec<Row> {
    vec![Row::Command(FOLD_COMMAND)]
}

/// One choice of a split button: a tool, or a tool started with one of its
/// methods (the web's `SplitEntry`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SplitEntry<'a> {
    pub command: &'a str,
    /// Given to the tool right after it starts, as if typed.
    pub option: Option<&'a str>,
    /// The button's text: the tool's name (a method does not rename it).
    pub title: &'a str,
    /// The list's text: the method, or the tool's name.
    pub label: &'a str,
    pub description: Option<&'a str>,
    /// A method's own icon (Ölçülendirme's kinds, docs/adr/0147 §7); none: its command's.
    pub icon: Option<&'a str>,
}

/// A row of a split button's list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SplitRow<'a> {
    pub command: &'a str,
    pub option: Option<&'a str>,
    pub label: &'a str,
    pub hint: Option<&'a str>,
    /// The method's own icon; none: the command's.
    pub icon: Option<&'a str>,
}

/// A split button's list: a tool's methods under the tool's name, or a
/// family's tools without a title; each row the entry's words, its
/// description as the hint and a method's own icon.
pub(crate) fn split_menu<'a>(entries: &[SplitEntry<'a>]) -> (Option<&'a str>, Vec<SplitRow<'a>>) {
    let methods = entries
        .first()
        .is_some_and(|first| entries.iter().all(|e| e.command == first.command));
    let header = entries.first().filter(|_| methods).map(|e| e.title);
    let rows = entries
        .iter()
        .map(|e| SplitRow {
            command: e.command,
            option: e.option.filter(|o| !o.is_empty()),
            label: e.label,
            hint: e.description.filter(|d| !d.is_empty()),
            icon: e.icon.filter(|i| !i.is_empty()),
        })
        .collect();
    (header, rows)
}

/// The top of a split button: its name (a method does not rename it, a
/// trailing “…” goes), what it says it does (`Daire: 3 nokta`) and the
/// chosen method's own icon, when it has one (else its command's).
pub(crate) fn split_face<'a>(e: &SplitEntry<'a>) -> (String, String, Option<&'a str>) {
    let label = e.title.strip_suffix('…').unwrap_or(e.title).to_owned();
    let option = e.option.is_some_and(|o| !o.is_empty());
    let aria = if option || e.label != e.title {
        texts::method(e.title, e.label)
    } else {
        e.title.to_owned()
    };
    (label, aria, e.icon.filter(|i| !i.is_empty()))
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value, json};

    use super::*;

    const FIXTURE: &str = include_str!("../../../fixtures/shell/v1/ribbon.json");

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).expect("ribbon.json reads")
    }

    fn strings(v: &Value) -> Vec<String> {
        v.as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn opt<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
        v.get(key).and_then(Value::as_str)
    }

    /// A row as the web writes it: only the keys it has.
    fn row_json(row: &Row) -> Value {
        match row {
            Row::Header(label) => json!({ "kind": "header", "label": label }),
            Row::Separator => json!({ "kind": "separator" }),
            Row::Command(command) => json!({ "kind": "command", "command": command }),
            Row::Quick(q) => {
                let mut o = Map::new();
                o.insert("kind".into(), json!("quick"));
                o.insert("command".into(), json!(q.command));
                if let Some(label) = q.label {
                    o.insert("label".into(), json!(label));
                }
                if let Some(icon) = q.icon {
                    o.insert("icon".into(), json!(icon));
                }
                if let Some(checked) = q.checked {
                    o.insert("checked".into(), json!(checked));
                }
                if q.disabled {
                    o.insert("disabled".into(), json!(true));
                }
                if let Some(hint) = q.hint {
                    o.insert("hint".into(), json!(hint));
                }
                if let Some(set) = q.set {
                    o.insert("set".into(), json!(set));
                }
                Value::Object(o)
            }
        }
    }

    fn entry(v: &Value) -> SplitEntry<'_> {
        SplitEntry {
            command: opt(v, "command").unwrap_or_default(),
            option: opt(v, "option"),
            title: opt(v, "title").unwrap_or_default(),
            label: opt(v, "label").unwrap_or_default(),
            description: opt(v, "description"),
            icon: opt(v, "icon"),
        }
    }

    #[test]
    fn the_words_are_the_web_s() {
        let f = fixture();
        let t = &f["texts"];
        for (key, ours) in [
            ("quickAccess", texts::QUICK_ACCESS),
            ("customize", texts::CUSTOMIZE),
            ("customizeTip", texts::CUSTOMIZE_TIP),
            ("fixedHint", texts::FIXED_HINT),
            ("add", texts::ADD),
            ("remove", texts::REMOVE),
            ("fixed", texts::FIXED),
            ("fold", texts::FOLD),
            ("pin", texts::PIN),
            ("foldTip", texts::FOLD_TIP),
        ] {
            assert_eq!(t[key].as_str(), Some(ours), "{key}");
        }
        let sample = |key: &str| strings(&t[key]["sample"]);
        let said = |key: &str| t[key]["text"].as_str().unwrap_or_default().to_owned();
        assert_eq!(texts::others(&sample("others")[0]), said("others"));
        let m = sample("method");
        assert_eq!(texts::method(&m[0], &m[1]), said("method"));
        assert_eq!(
            texts::panel_more(&sample("panelMore")[0]),
            said("panelMore")
        );
        let c = sample("cannotStart");
        assert_eq!(texts::cannot_start(&c[0], &c[1]), said("cannotStart"));
        assert_eq!(strings(&f["quickAccessFixed"]), QUICK_ACCESS);
        assert_eq!(strings(&f["quickAccessOffers"]), QUICK_ACCESS_OFFERS);
    }

    #[test]
    fn the_bar_s_menu_lists_the_bar_then_the_offers() {
        let f = fixture();
        let cases = f["quickAccessMenus"].as_array().expect("cases");
        assert!(!cases.is_empty());
        for case in cases {
            let bar = strings(&case["bar"]);
            let exists = strings(&case["exists"]);
            let rows: Vec<Value> = quick_access_menu(&bar, |id| exists.iter().any(|x| x == id))
                .iter()
                .map(row_json)
                .collect();
            assert_eq!(Value::Array(rows), case["rows"], "{}", case["title"]);
        }
    }

    #[test]
    fn a_command_is_put_at_the_bar_s_end_or_taken_off() {
        let f = fixture();
        for case in f["toggles"].as_array().expect("cases") {
            let kept = strings(&case["kept"]);
            let id = case["command"].as_str().unwrap_or_default();
            let on = case["on"].as_bool().unwrap_or_default();
            assert_eq!(
                with_quick_access(&kept, id, on),
                strings(&case["result"]),
                "{case}"
            );
        }
    }

    #[test]
    fn a_right_click_offers_the_bar_then_the_fold() {
        let f = fixture();
        for case in f["commandMenus"].as_array().expect("cases") {
            let id = case["command"].as_str().unwrap_or_default();
            let bar = strings(&case["bar"]);
            let rows: Vec<Value> = command_menu(id, &bar).iter().map(row_json).collect();
            assert_eq!(Value::Array(rows), case["rows"], "{id}");
        }
        let rows: Vec<Value> = ribbon_menu().iter().map(row_json).collect();
        assert_eq!(Value::Array(rows), f["ribbonMenu"]);
    }

    #[test]
    fn a_split_button_lists_its_entries_and_shows_the_chosen_one() {
        let f = fixture();
        for case in f["splitMenus"].as_array().expect("cases") {
            let entries: Vec<SplitEntry> = case["entries"]
                .as_array()
                .into_iter()
                .flatten()
                .map(entry)
                .collect();
            let (header, rows) = split_menu(&entries);
            assert_eq!(header, case["menu"]["header"].as_str(), "{case}");
            let rows: Vec<Value> = rows
                .iter()
                .map(|r| {
                    let mut o = Map::new();
                    o.insert("command".into(), json!(r.command));
                    if let Some(option) = r.option {
                        o.insert("option".into(), json!(option));
                    }
                    o.insert("label".into(), json!(r.label));
                    if let Some(hint) = r.hint {
                        o.insert("hint".into(), json!(hint));
                    }
                    if let Some(icon) = r.icon {
                        o.insert("icon".into(), json!(icon));
                    }
                    Value::Object(o)
                })
                .collect();
            assert_eq!(Value::Array(rows), case["menu"]["rows"], "{case}");
        }
        for case in f["splitFaces"].as_array().expect("cases") {
            let (label, aria, icon) = split_face(&entry(&case["entry"]));
            assert_eq!(
                Some(label.as_str()),
                case["face"]["label"].as_str(),
                "{case}"
            );
            assert_eq!(Some(aria.as_str()), case["face"]["aria"].as_str(), "{case}");
            assert_eq!(icon, case["face"]["icon"].as_str(), "{case}");
        }
    }
}
