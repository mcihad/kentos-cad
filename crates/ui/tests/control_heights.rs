//! Her girdi aynı yükseklikte (docs/adr/0804): metin, açılır liste, sayı, tarih, saat, renk, arama, öneri, etiket,
//! parçalı seçim ve eylem düğmesi yan yana durunca hizası bozulmaz; yazı boyutu büyüyünce hepsi birlikte büyür. Izgara
//! hücresinin düzenleyicileri satırı tam doldurur. Uygulamalar `text_input`'u kendi iç boşluğuyla kurmaz.

#![cfg(feature = "snapshot")]

use std::path::{Path, PathBuf};

use iced::advanced::{layout, widget::Tree};
use iced::widget::button;
use iced::{Color, Element, Size};
use kentos_ui::label;
use kentos_ui::snapshot::Snapshot;
use kentos_ui::style;
use kentos_ui::theme::{metrics, typography};
use kentos_ui::widget::{
    Cell, ChipInput, Choice, ColorPicker, DatePicker, InputField, NumberInput, SearchBox,
    Segmented, Select, Suggest, TimePicker, bare_field, text_field,
};

fn height(snapshot: &Snapshot, mut element: Element<'_, ()>) -> f32 {
    let mut tree = Tree::new(&element);
    let limits = layout::Limits::new(Size::ZERO, Size::new(400.0, 400.0));
    element
        .as_widget_mut()
        .layout(&mut tree, snapshot.renderer(), &limits)
        .size()
        .height
}

fn select<'a>() -> Select<'a, ()> {
    Select::new([Choice::new("Bir"), Choice::new("İki")], Some(0), |_| ())
}

/// Every input, named.
fn inputs<'a>() -> Vec<(&'static str, Element<'a, ()>)> {
    vec![
        (
            "InputField",
            InputField::new("Ad", "değer").on_input(|_| ()).into(),
        ),
        (
            "InputField, birimli ve ikonlu",
            InputField::new("Ad", "2,5")
                .on_input(|_| ())
                .suffix("m")
                .icon(kentos_ui::icon::Icon::Search)
                .into(),
        ),
        ("text_field", text_field("Ad", "değer").on_input(|_| ()).into()),
        ("Select", select().into()),
        ("Select, önekli", select().prefix("Katman").into()),
        ("NumberInput", NumberInput::new(2.5, |_| ()).into()),
        (
            "NumberInput, etiketli",
            NumberInput::new(2.5, |_| ()).label("X").into(),
        ),
        ("DatePicker", DatePicker::date(None, |_| ()).into()),
        ("TimePicker", TimePicker::new(None, |_| ()).into()),
        ("ColorPicker", ColorPicker::new(Color::BLACK, |_| ()).into()),
        ("SearchBox", SearchBox::new("", "Ara", |_| ()).fill().into()),
        ("Suggest", Suggest::new("", "Ara").on_input(|_| ()).into()),
        ("ChipInput", ChipInput::new(&[], |_| ()).into()),
        (
            "Segmented",
            Segmented::new(["Bir", "İki"], "Bir", |_| ()).into(),
        ),
        (
            "Segmented, sıkışık",
            Segmented::new(["Bir", "İki"], "Bir", |_| ())
                .compact()
                .into(),
        ),
        (
            "eylem düğmesi",
            button(label::body("Kaydet"))
                .on_press(())
                .padding(metrics::action())
                .style(style::button::primary)
                .into(),
        ),
        (
            "ikincil düğme",
            button(label::body("Vazgeç"))
                .on_press(())
                .padding(metrics::action())
                .style(style::button::secondary)
                .into(),
        ),
    ]
}

/// Every editor of a grid's cell, named.
fn cells<'a>() -> Vec<(&'static str, Element<'a, ()>)> {
    vec![
        (
            "metin",
            Cell::new(InputField::new("", "a").on_input(|_| ()).cell()).into(),
        ),
        ("liste", Cell::new(select().cell()).into()),
        ("sayı", Cell::new(NumberInput::new(1.0, |_| ()).cell()).into()),
        (
            "tarih",
            Cell::new(
                DatePicker::date(None, |_| ())
                    .anchor(InputField::new("", "").on_input(|_| ()).cell())
                    .cell(),
            )
            .into(),
        ),
        (
            "parçalı",
            Cell::new(Segmented::new(["E", "H"], "E", |_| ()).cell()).into(),
        ),
        (
            "renk",
            Cell::new(ColorPicker::new(Color::BLACK, |_| ()).cell()).into(),
        ),
        ("düz", Cell::flat(label::body("a")).into()),
    ]
}

#[test]
fn every_input_has_the_one_height_at_every_size() {
    let snapshot = Snapshot::software(Size::new(600.0, 600.0)).expect("a renderer");
    for size in [11.0, 13.0, 15.0, 18.0] {
        typography::set(typography::Typography {
            size,
            ..typography::Typography::DEFAULT
        });
        let want = metrics::control();
        assert!(want >= 22.0, "{size}: {want}");
        let mut wrong = Vec::new();
        for (name, input) in inputs() {
            let got = height(&snapshot, input);
            if (got - want).abs() > 0.51 {
                wrong.push(format!("{name}: {got}"));
            }
        }
        assert!(wrong.is_empty(), "{size} px, {want} isteniyor: {wrong:?}");
        let row = metrics::row();
        assert_eq!(row, want, "a row is a control's height");
        let mut wrong = Vec::new();
        for (name, cell) in cells() {
            let got = height(&snapshot, cell);
            if got != row {
                wrong.push(format!("{name}: {got}"));
            }
        }
        assert!(wrong.is_empty(), "{size} px, hücre {row} isteniyor: {wrong:?}");
        assert!(
            (height(&snapshot, bare_field("", "").on_input(|_| ()).into()) - want).abs() <= 0.51
        );
    }
    typography::set(typography::Typography::DEFAULT);
}

fn sources(under: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(under) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, into);
        } else if path.extension().is_some_and(|e| e == "rs") {
            into.push(path);
        }
    }
}

/// `text_input(` as a call, on a line of code: not a path to the module, nor a method of an operation.
fn calls_it(line: &str) -> bool {
    let code = line.trim_start();
    if code.starts_with("//") {
        return false;
    }
    let Some(at) = code.find("text_input(") else {
        return false;
    };
    let before = code[..at].chars().next_back();
    !matches!(before, Some(c) if c.is_alphanumeric() || c == '_' || c == '.')
        && !code[..at].trim_end().ends_with("fn")
}

#[test]
fn the_apps_make_their_fields_with_the_kit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for dir in ["apps/desktop/src", "apps/launcher/src", "crates/sheet-ui/src"] {
        sources(&root.join(dir), &mut files);
    }
    assert!(files.len() > 100, "the sources were found");
    let mut raw = Vec::new();
    for file in files {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            if calls_it(line) {
                raw.push(format!("{}:{}", file.display(), i + 1));
            }
        }
    }
    assert!(
        raw.is_empty(),
        "iced'in text_input'u yerine kentos_ui::widget::text_field: {raw:?}"
    );
}
