//! Fare ve klavye kısayolları (the web's `openShortcutsDialog`,
//! ui/dialogs.ts; docs/inventory/parity-audit.md H1, H2): what each mouse
//! action does, then every command with a key by the web's categories, with
//! its icon and command line names; a search over all of it. The keys are
//! the catalog's, as the web binds them.

use iced::widget::{Column, Row, column, container, row, text_input};
use iced::{Center, Element, Fill, Task, Theme};
use kentos_ui::icon::icon;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::catalog::catalog;
use crate::exchange::apply::fold_turkish;

/// The search field, focused when the window opens.
pub const SEARCH: &str = "shortcuts-search";

/// What each mouse action does (the web's `MOUSE_ACTIONS`): its meaning, then the action.
pub const MOUSE_ACTIONS: [(&str, &str); 17] = [
    ("Seç, nokta koy, adımı onayla", "Sol tık"),
    ("Komutu onayla ya da bitir (Enter gibi)", "Sağ tık"),
    (
        "Komut menüsü: seçenekler, tek seferlik kenet, orto",
        "Sağ tuşu basılı tut",
    ),
    (
        "Tek seferlik kenet (uç, orta, kesişim, dik…)",
        "Shift + sağ tık",
    ),
    ("Boştayken bağlam menüsü", "Sağ tık"),
    (
        "Köşeyi sil, kenara köşe ekle, yaya dönüştür",
        "Tutamaca sağ tık",
    ),
    ("Nesnenin türü, katmanı, alanı", "Üzerinde bekle"),
    (
        "İzleme noktası al ya da bırak (nesne izleme)",
        "Kenet noktasında bekle",
    ),
    (
        "Mesafe ya da koordinat (imleç yanında açılır)",
        "Komut sırasında sayı yaz",
    ),
    (
        "Pencere seçimi: tamamen içeride kalanlar",
        "Soldan sağa sürükle",
    ),
    ("Kesişim seçimi: dokunan her şey", "Sağdan sola sürükle"),
    ("Seçime ekle ya da çıkar", "Shift + tık"),
    ("Görünümü kaydır", "Orta tuşla sürükle"),
    ("Yakınlaştır, uzaklaştır", "Tekerlek"),
    ("Tümünü göster", "Orta tuşa çift tık"),
    ("Yazıyı ya da ölçüyü yerinde düzenle", "Çift tık"),
    (
        "Köşeyi taşı; kenar ortasındaki baklava köşe ekler",
        "Tutamacı sürükle",
    ),
];

/// A chord as menus and tooltips write it (the web's `formatChord`).
pub fn chord(text: &str) -> String {
    text.split('+')
        .map(|part| match part {
            "Delete" => "Del",
            "Space" => "Boşluk",
            other => other,
        })
        .collect::<Vec<_>>()
        .join("+")
}

/// One command's row: its icon, title, first two command line names and keys.
struct Line {
    icon: kentos_ui::icon::Icon,
    title: &'static str,
    aliases: Vec<&'static str>,
    chords: Vec<String>,
}

/// The commands with keys, by category in the catalog's order, those the
/// search finds (its title, keys or names).
fn groups(query: &str) -> Vec<(&'static str, Vec<Line>)> {
    let mut out: Vec<(&'static str, Vec<Line>)> = Vec::new();
    for c in catalog().commands() {
        if c.shortcuts.is_empty() {
            continue;
        }
        let line = Line {
            icon: c.icon,
            title: c.title,
            aliases: c.aliases.iter().take(2).copied().collect(),
            chords: c.shortcuts.iter().map(|s| chord(s)).collect(),
        };
        let found = query.is_empty()
            || fold_turkish(line.title).contains(query)
            || line.chords.iter().any(|k| fold_turkish(k).contains(query))
            || line.aliases.iter().any(|a| fold_turkish(a).contains(query));
        if !found {
            continue;
        }
        let category = if c.category.is_empty() {
            "Diğer"
        } else {
            c.category
        };
        match out.iter_mut().find(|(name, _)| *name == category) {
            Some((_, lines)) => lines.push(line),
            None => out.push((category, vec![line])),
        }
    }
    out
}

/// A key as a cap.
fn kbd<'a>(key: String) -> Element<'a, Message> {
    container(label::mono_caption(key))
        .padding([1, 6])
        .style(|theme: &Theme| {
            let t = Tokens::of(theme);
            container::Style {
                background: Some(t.surface_alt.into()),
                border: iced::border::rounded(kentos_ui::theme::shape::radius(3.0))
                    .width(1.0)
                    .color(t.border),
                text_color: Some(t.text),
                ..container::Style::default()
            }
        })
        .into()
}

/// A group: its name over its rows.
fn group<'a>(title: &'a str, rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    column![
        label::body(title).font(typography::ui_strong()),
        Column::with_children(rows).spacing(4),
    ]
    .spacing(8)
    .into()
}

impl App {
    /// Opens the window with an empty search, its field focused.
    pub(crate) fn open_shortcuts(&mut self) -> Task<Message> {
        self.shortcuts_query.clear();
        self.dialog = Some(crate::app::Dialog::Shortcuts);
        iced::widget::operation::focus(SEARCH)
    }

    /// The window.
    pub(crate) fn shortcuts_view(&self) -> Element<'_, Message> {
        let query = fold_turkish(&self.shortcuts_query);
        let mut blocks: Vec<(usize, Element<'_, Message>)> = Vec::new();
        let mouse: Vec<Element<'_, Message>> = MOUSE_ACTIONS
            .iter()
            .filter(|(what, how)| {
                query.is_empty()
                    || fold_turkish(what).contains(&query)
                    || fold_turkish(how).contains(&query)
            })
            .map(|(what, how)| {
                row![label::body(*what).width(Fill), kbd((*how).to_owned())]
                    .spacing(12)
                    .align_y(Center)
                    .into()
            })
            .collect();
        if !mouse.is_empty() {
            blocks.push((mouse.len(), group("Fare", mouse)));
        }
        for (category, lines) in groups(&query) {
            let count = lines.len();
            let rows = lines
                .into_iter()
                .map(|l| {
                    let mut name = Row::new()
                        .spacing(8)
                        .align_y(Center)
                        .push(icon(l.icon).size(15.0))
                        .push(label::body(l.title));
                    if !l.aliases.is_empty() {
                        name = name.push(
                            label::mono_caption(l.aliases.join(", ")).style(style::text::muted),
                        );
                    }
                    let keys = l
                        .chords
                        .into_iter()
                        .fold(Row::new().spacing(4), |keys, k| keys.push(kbd(k)));
                    row![container(name).width(Fill), keys]
                        .spacing(12)
                        .align_y(Center)
                        .into()
                })
                .collect();
            blocks.push((count, group(category, rows)));
        }
        // Two columns, the groups dealt to the shorter one (the web's grid).
        let (mut left, mut right) = (Column::new().spacing(18), Column::new().spacing(18));
        let (mut left_rows, mut right_rows) = (0, 0);
        let empty = blocks.is_empty();
        for (count, block) in blocks {
            if left_rows <= right_rows {
                left = left.push(block);
                left_rows += count + 2;
            } else {
                right = right.push(block);
                right_rows += count + 2;
            }
        }
        let body: Element<'_, Message> = if empty {
            label::muted("Aramayla eşleşen kısayol yok.").into()
        } else {
            row![left.width(Fill), right.width(Fill)].spacing(28).into()
        };
        let search = text_input("Komut ya da tuş ara", &self.shortcuts_query)
            .id(SEARCH)
            .on_input(Message::ShortcutsSearch)
            .size(typography::body())
            .padding([5, 8])
            .style(style::field::input);
        let search = kentos_ui::widget::focus_ring(search);
        overlay::modal(
            Dialog::new("Fare ve klavye kısayolları")
                .hint("F1")
                .push(label::muted(
                    "Harf kısayolları klavyenizdeki harfe göre çalışır; Türkçe Q ve F düzeninde de aynı harfe basın. Komutları takma adıyla komut satırına yazıp Enter’a da basabilirsiniz.",
                ))
                .push(search)
                // Room for the scroll bar beside the right column.
                .scroll(container(body).padding(iced::Padding {
                    right: 12.0,
                    ..iced::Padding::ZERO
                }))
                .action(
                    iced::widget::button(iced::widget::text("Kapat"))
                        .on_press(Message::DialogClosed)
                        .style(style::button::secondary),
                )
                .width(880.0)
                .max_height(760.0),
            Message::DialogClosed,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chords_are_written_as_the_web_writes_them() {
        assert_eq!(chord("Delete"), "Del");
        assert_eq!(chord("Ctrl+Space"), "Ctrl+Boşluk");
        assert_eq!(chord("Shift+F3"), "Shift+F3");
    }

    #[test]
    fn the_search_finds_titles_keys_and_names() {
        let all = groups("");
        assert!(!all.is_empty(), "the catalog has keys");
        let save = groups(&fold_turkish("ctrl+s"));
        assert!(
            save.iter()
                .flat_map(|(_, lines)| lines)
                .any(|l| l.chords.iter().any(|k| k == "Ctrl+S")),
            "found by its key"
        );
        let snap = groups(&fold_turkish("kenet"));
        assert!(
            snap.iter()
                .flat_map(|(_, lines)| lines)
                .all(|l| fold_turkish(l.title).contains("KENET")
                    || l.aliases.iter().any(|a| fold_turkish(a).contains("KENET"))
                    || l.chords.iter().any(|k| fold_turkish(k).contains("KENET"))),
            "only what it matches"
        );
        assert!(groups("ZZZ-YOK").is_empty());
    }

    /// The window for the owner, whole and searched, dark and light, at both
    /// sizes; `.run/shots/kisayollar-*`.
    /// `cargo test -p kentos-desktop shortcuts::tests::screens -- --ignored --nocapture`
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        use iced::Size;
        use kentos_ui::snapshot::Snapshot;

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
            for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
                for (name, query) in [("tumu", ""), ("arama", "kenet")] {
                    let mut app = crate::files_testing::app_with_drawing();
                    let _ = app
                        .settings
                        .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                    app.apply_settings();
                    let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                    let mut update = |app: &mut App, message| {
                        let _ = app.update(message);
                    };
                    snapshot.settle(&mut app, App::view, &mut update);
                    let _ = app.run("help.shortcuts");
                    let _ = app.update(Message::ShortcutsSearch(query.to_owned()));
                    snapshot.settle(&mut app, App::view, &mut update);
                    let file = out.join(format!("kisayollar-{name}-{width}x{height}{suffix}.png"));
                    snapshot
                        .render(app.view(), &app.theme())
                        .save(&file)
                        .expect("writes the picture");
                    println!("{}", file.display());
                }
            }
        }
    }

    #[test]
    fn the_window_opens_with_an_empty_search() {
        let (mut app, _) = App::boot(None);
        app.shortcuts_query = "eski".into();
        let _ = app.run("help.shortcuts");
        assert_eq!(app.dialog, Some(crate::app::Dialog::Shortcuts));
        assert!(app.shortcuts_query.is_empty());
        let _ = app.update(Message::ShortcutsSearch("kayd".into()));
        assert_eq!(app.shortcuts_query, "kayd");
    }
}
