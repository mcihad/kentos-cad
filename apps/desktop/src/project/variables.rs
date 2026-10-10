//! Proje ayarları › Değişkenler (docs/adr/0214 §2.3, §4): the project's own
//! `@` values, a row each (Ad, Etiket, Tür, Değer; up, down, delete;
//! Değişken ekle), typed as `kentos_project::variable_form` reads them,
//! what does not hold under its row (Kaydet waits); then the built-in values
//! with what they hold now. The web's is `variablesSection` in
//! `ui/settings/ProjectSettingsDialog.ts`.

use iced::widget::{Column, Id, button, column, container, row, text_input};
use iced::{Center, Element, Fill, Length};
use kentos_contracts::VariableKind;
use kentos_expression::{Value, Variable};
use kentos_project::variable_form::{self, FALSE, Row, TRUE};
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Switch};
use kentos_ui::{label, style};

use super::group;
use crate::app::Message;

/// A change to one row.
#[derive(Debug, Clone)]
pub enum Edit {
    Name(String),
    Label(String),
    Kind(VariableKind),
    Value(String),
    Up,
    Down,
    Delete,
}

/// The kinds as the row's list names them.
const KINDS: [(VariableKind, &str); 4] = [
    (VariableKind::Text, "Metin"),
    (VariableKind::Number, "Sayı"),
    (VariableKind::Bool, "Doğru/yanlış"),
    (VariableKind::Date, "Tarih"),
];

/// A row's name field, for the keyboard to go to a new one.
pub(super) fn name_id(i: usize) -> Id {
    Id::from(format!("project-variable-name-{i}"))
}

/// A change applied to the rows.
pub(super) fn edit(rows: &mut Vec<Row>, i: usize, e: Edit) {
    let n = rows.len();
    let Some(r) = rows.get_mut(i) else {
        return;
    };
    match e {
        Edit::Name(t) => r.name = t,
        Edit::Label(t) => r.label = t,
        Edit::Kind(k) => *r = variable_form::with_kind(r, k),
        Edit::Value(t) => r.value = t,
        Edit::Up if i > 0 => rows.swap(i, i - 1),
        Edit::Down if i + 1 < n => rows.swap(i, i + 1),
        Edit::Up | Edit::Down => {}
        Edit::Delete => {
            rows.remove(i);
        }
    }
}

/// The columns' widths: the name, the label and the value share the row,
/// the kind is as wide as its longest word.
fn width(j: usize) -> Length {
    match j {
        2 => Length::Fixed(typography::scaled(128.0)),
        3 => Length::FillPortion(6),
        _ => Length::FillPortion(5),
    }
}

fn cell<'a>(e: impl Into<Element<'a, Message>>, j: usize) -> Element<'a, Message> {
    container(e).width(width(j)).into()
}

/// The three buttons' room at the row's end (for the captions over the columns).
const BUTTONS: f32 = 3.0 * 26.0 + 2.0 * 6.0;

fn field<'a>(
    placeholder: &str,
    value: &'a str,
    mono: bool,
    on: impl Fn(String) -> Message + 'a,
) -> text_input::TextInput<'a, Message> {
    let input = text_input(placeholder, value)
        .on_input(on)
        .padding([4, 7])
        .size(typography::body())
        .style(style::field::input);
    if mono {
        input.font(typography::mono())
    } else {
        input
    }
}

fn small(glyph: &'static str, enabled: bool, on: Message) -> Element<'static, Message> {
    button(container(icon(crate::icons::from_web(Some(glyph))).size(14.0)).center_x(Length::Fill))
        .on_press_maybe(enabled.then_some(on))
        .padding([4, 0])
        .width(Length::Fixed(26.0))
        .style(style::button::ghost)
        .into()
}

/// A built-in value as its row shows it.
fn shown(v: &Value<'_>) -> String {
    match v {
        Value::Null => "yok".to_owned(),
        Value::Bool(true) => TRUE.to_owned(),
        Value::Bool(false) => FALSE.to_owned(),
        Value::Num(x) => variable_form::number_text(*x),
        Value::Text(t) => t.to_string(),
    }
}

/// The section: the rows, Değişken ekle, the built-in values (`builtins`,
/// the project's own left out of them).
pub(super) fn view<'a>(
    rows: &'a [Row],
    builtins: Vec<Variable>,
    on: impl Fn(usize, Edit) -> Message + Copy + 'a,
    on_add: Message,
) -> Element<'a, Message> {
    let read = variable_form::read(rows);
    let head = row![
        cell(
            row![iced::widget::space().width(14), label::caption("Ad")],
            0
        ),
        cell(label::caption("Etiket"), 1),
        cell(label::caption("Tür"), 2),
        cell(label::caption("Değer"), 3),
        iced::widget::space().width(Length::Fixed(BUTTONS)),
    ]
    .spacing(6);
    let mut table = Column::new().spacing(4).push(head);
    for (i, r) in rows.iter().enumerate() {
        let name = row![
            label::mono("@").width(Length::Fixed(10.0)),
            field("ad", &r.name, true, move |t| on(i, Edit::Name(t))).id(name_id(i)),
        ]
        .spacing(4)
        .align_y(Center);
        let kind = Select::new(
            KINDS.iter().map(|(_, words)| Choice::new(*words)),
            KINDS.iter().position(|(k, _)| *k == r.kind),
            move |j| on(i, Edit::Kind(KINDS[j.min(KINDS.len() - 1)].0)),
        )
        .searchable(false);
        let value: Element<'a, Message> = if r.kind == VariableKind::Bool {
            let on_now = r.value == TRUE;
            row![
                Switch::new(on_now, move |b| on(
                    i,
                    Edit::Value(if b { TRUE } else { FALSE }.to_owned())
                )),
                label::body(if r.value.is_empty() {
                    "değeri yok"
                } else {
                    r.value.as_str()
                }),
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        } else {
            let holder = match r.kind {
                VariableKind::Number => "ör. 1.5",
                VariableKind::Date => "YYYY-AA-GG",
                _ => "değeri yok",
            };
            field(holder, &r.value, true, move |t| on(i, Edit::Value(t))).into()
        };
        let line = row![
            cell(name, 0),
            cell(
                field("ne olduğu", &r.label, false, move |t| on(
                    i,
                    Edit::Label(t)
                )),
                1
            ),
            cell(kind, 2),
            cell(value, 3),
            small("chevronUp", i > 0, on(i, Edit::Up)),
            small("chevronDown", i + 1 < rows.len(), on(i, Edit::Down)),
            small("trash", true, on(i, Edit::Delete)),
        ]
        .spacing(6)
        .align_y(Center);
        table = table.push(line);
        if let Some(p) = read.problems.get(i) {
            let words: Vec<&str> = [p.name.as_deref(), p.value.as_deref()]
                .into_iter()
                .flatten()
                .collect();
            if !words.is_empty() {
                table = table.push(
                    label::caption(words.join(" "))
                        .style(style::text::danger)
                        .width(Fill),
                );
            }
        }
    }
    if let Some(list) = read.list {
        table = table.push(label::caption(list).style(style::text::danger));
    }
    let add = button(
        row![
            icon(crate::icons::from_web(Some("plus"))).size(13.0),
            label::body("Değişken ekle")
        ]
        .spacing(5)
        .align_y(Center),
    )
    .on_press(on_add)
    .padding([4, 10])
    .style(style::button::secondary);
    let own: Element<'a, Message> = if rows.is_empty() {
        Banner::info(
            "Projenin değişkeni yok. İş numarası, idare, katsayı gibi her ifadede aynı olan değerler için ekleyin.",
        )
        .into()
    } else {
        table.into()
    };
    let mut built = Column::new().spacing(8);
    for v in builtins.iter().filter(|v| {
        !rows
            .iter()
            .any(|r| r.name.trim().trim_start_matches('@') == v.name)
    }) {
        built = built.push(builtin_row(
            format!("@{}", v.name),
            v.description.clone(),
            shown(&v.value),
        ));
    }
    built = built.push(builtin_row(
        "@katman_adi, @katman".to_owned(),
        "Değerlendirilen nesnenin katmanının adı ($katman ile aynı); her yerde okunur.".to_owned(),
        "nesneye göre".to_owned(),
    ));
    column![
        group(
            "Projenin değişkenleri",
            column![
                label::caption(
                    "İfadelerde @ad olarak okunur: İşlemler'de, İfade oluşturucu'da, Öznitelik tablosunun süzgecinde ve paftada (paftanın kendi değişkenlerinden sonra). Stil, etiket, katman süzgeci ve ağ maliyetleri yalnız @katman_adi'yi okur."
                ),
                own,
                add,
            ]
            .spacing(10)
        ),
        group(
            "Hazır değişkenler",
            column![
                label::caption(
                    "Kendiliğinden dolar; tarih ve saat bu cihazın saatindendir, ifade boyunca aynıdır."
                ),
                built,
            ]
            .spacing(10)
        ),
    ]
    .spacing(16)
    .into()
}

/// A built-in value's row: its name and what it holds, its value on the right.
fn builtin_row<'a>(name: String, hint: String, value: String) -> Element<'a, Message> {
    row![
        container(
            Column::new()
                .spacing(2)
                .push(label::mono(name))
                .push(label::caption(hint))
        )
        .width(Fill),
        label::body(value),
    ]
    .spacing(16)
    .align_y(Center)
    .into()
}

#[cfg(test)]
mod tests {
    use kentos_contracts::{ProjectVariable, VariableKind, VariableValue};

    use super::Edit;
    use crate::app::{App, Message};
    use crate::project::settings::Section;
    use crate::project::{SettingsEvent, settings_message};

    fn send(app: &mut App, e: SettingsEvent) {
        let _ = app.update(settings_message(e));
    }

    fn row(app: &mut App, i: usize, e: Edit) {
        send(app, SettingsEvent::Variable(i, e));
    }

    fn variable(
        name: &str,
        label: &str,
        kind: VariableKind,
        value: VariableValue,
    ) -> ProjectVariable {
        ProjectVariable {
            name: name.into(),
            label: label.into(),
            kind,
            value,
        }
    }

    /// The four variables the pictures and the web's scenes show.
    fn sample() -> Vec<ProjectVariable> {
        vec![
            variable(
                "is_no",
                "İş numarası",
                VariableKind::Text,
                VariableValue::Text("2026/41".into()),
            ),
            variable(
                "idare",
                "İdare",
                VariableKind::Text,
                VariableValue::Text("Çankaya Belediyesi".into()),
            ),
            variable(
                "katsayi",
                "Emsal katsayısı",
                VariableKind::Number,
                VariableValue::Number(1.5),
            ),
            variable(
                "teslim",
                "Teslim tarihi",
                VariableKind::Date,
                VariableValue::Text("2026-11-30".into()),
            ),
            variable(
                "onayli",
                "Onaylı",
                VariableKind::Bool,
                VariableValue::Bool(true),
            ),
        ]
    }

    /// Değişkenler (docs/adr/0214 §4): a new row typed and saved goes into
    /// the project; a row that does not hold keeps Kaydet waiting; the rows
    /// move and go; the section's reset leaves none.
    #[test]
    fn the_variables_are_typed_checked_and_saved() {
        let mut app = crate::files_testing::app_with_drawing();
        let _ = app.update(Message::Run("file.settings"));
        send(&mut app, SettingsEvent::Section(Section::Variables));
        send(&mut app, SettingsEvent::AddVariable);
        row(&mut app, 0, Edit::Name("@İş_No".into()));
        row(&mut app, 0, Edit::Label(" İş numarası ".into()));
        row(&mut app, 0, Edit::Value("2026/41".into()));
        send(&mut app, SettingsEvent::AddVariable);
        row(&mut app, 1, Edit::Name("katsayi".into()));
        row(&mut app, 1, Edit::Kind(VariableKind::Number));
        row(&mut app, 1, Edit::Value("bir buçuk".into()));
        send(&mut app, SettingsEvent::Save);
        assert!(app.project.is_some(), "Kaydet waits for the number");
        row(&mut app, 1, Edit::Value("1,5".into()));
        row(&mut app, 1, Edit::Up);
        send(&mut app, SettingsEvent::Save);
        assert!(app.project.is_none());
        let settings = app.document.as_ref().expect("a drawing").settings().clone();
        assert_eq!(
            settings.variables,
            vec![
                variable(
                    "katsayi",
                    "",
                    VariableKind::Number,
                    VariableValue::Number(1.5)
                ),
                variable(
                    "İş_No",
                    "İş numarası",
                    VariableKind::Text,
                    VariableValue::Text("2026/41".into())
                ),
            ]
        );
        // The expressions read them, the built-in values after.
        let values = app.expression_variables();
        assert_eq!(values[0].name, "katsayi");
        assert!(values.iter().any(|v| v.name == "proje_adi"));
        // A built-in's name is refused; deleting and the reset leave none.
        let _ = app.update(Message::Run("file.settings"));
        send(&mut app, SettingsEvent::Section(Section::Variables));
        row(&mut app, 0, Edit::Name("proje_adi".into()));
        send(&mut app, SettingsEvent::Save);
        assert!(app.project.is_some(), "Kaydet waits for the name");
        row(&mut app, 0, Edit::Delete);
        send(&mut app, SettingsEvent::ResetSection);
        send(&mut app, SettingsEvent::Save);
        let settings = app.document.as_ref().expect("a drawing").settings().clone();
        assert!(settings.variables.is_empty());
    }

    /// Değişkenler's pictures, in the light theme at 1440×900 and the dark at
    /// 1100×650: none, five with the built-in values under them, and a row
    /// typed wrong (the web's are `shots.mjs variables`, degiskenler-*).
    #[test]
    #[ignore = "writes pictures: cargo test -p kentos-desktop project::variables::tests::screens -- --ignored --nocapture"]
    fn screens() {
        use iced::Size;
        use kentos_ui::snapshot::Snapshot;

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        for (theme, w, h) in [("light", 1440.0, 900.0), ("dark", 1100.0, 650.0)] {
            for name in ["bos", "dolu", "sorun"] {
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
                app.apply_settings();
                if name != "bos" {
                    let doc = app.document.as_mut().expect("a drawing");
                    let mut settings = doc.settings().clone();
                    settings.variables = sample();
                    if name == "sorun" {
                        settings.variables.truncate(3);
                    }
                    doc.model.set_settings(settings);
                }
                let _ = app.update(Message::Run("file.settings"));
                send(&mut app, SettingsEvent::Section(Section::Variables));
                if name == "sorun" {
                    send(&mut app, SettingsEvent::AddVariable);
                    row(&mut app, 3, Edit::Name("proje_adı".into()));
                    row(&mut app, 2, Edit::Value("bir buçuk".into()));
                }
                app.follow.flash = None;
                let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("degiskenler-{name}-{w}x{h}-{theme}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
