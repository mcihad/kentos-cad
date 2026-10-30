//! `kentos-launcher snapshot çıktı.png [seçenekler]`: the window drawn
//! without opening one (KentOS UI's headless renderer), for visual checks
//! and documentation. The data is fixed, so that pictures are alike on every
//! machine: the bundled apps, KentOS found at `/opt/kentos/bin`, a fixed list
//! of recent drawings said from a fixed now, nothing running; no file of
//! the user's is read or written.

use std::path::{Path, PathBuf};

use iced::widget::{Column, Row, column, container, row, text, themer};
use iced::{Alignment, Element, Fill, Point, Size};
use kentos_launch::Module;
use kentos_launch::recent::Recent;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::snapshot::{Input, Snapshot};
use kentos_ui::theme::typography::{self, Typography};
use kentos_ui::theme::{Accent, Mode, Tokens};
use kentos_ui::widget::Severity;

use crate::app::{Dialog, Launcher, Message, Page};
use crate::draft::Draft;
use crate::glyphs::{self, Glyph};
use crate::locations::Locations;
use crate::process::Processes;
use crate::recent;
use crate::registry::{Folders, Registry};
use crate::resolve::Finder;
use crate::tones::Tone;
use crate::view;

const USAGE: &str = "\
Kullanım: kentos-launcher snapshot <çıktı.png> [seçenekler]

Seçenekler:
  --sayfa <ad>       uygulamalar, son, guncellemeler, indirmeler, ayarlar, ekle, ikonlar
                     (varsayılan: uygulamalar; ekle: Uygulama ekle penceresi açık;
                     ikonlar: karoların ve simgelerin açık ve koyu sayfası)
  --tema <ad>        koyu, acik (varsayılan: koyu)
  --boyut <G>x<Y>    pencere boyutu (varsayılan: 1440x900)
  --yazi <piksel>    gövde metninin boyutu, 11-18 (varsayılan: 13)
  --olcek <katsayı>  piksel yoğunluğu (varsayılan: 1)
  --secili <kimlik>  başlatılacak kart (ör. kentos-harita)
  --bildirimler      bildirimler açık, iki örnek bildirimle
  --imlec <x>,<y>    imleci taşır (üzerine gelinen kart)";

/// The fixed now the recent drawings are said from: 30 October 2026, 09:00 UTC.
const NOW: u64 = 1_793_350_800_000;
/// The fixed home the rows write as `~`.
const HOME: &str = "/home/kullanici";

/// KentOS installed as the snapshot says: `kentos-cad` at `/opt/kentos/bin`, the modules' own programs not.
struct Installed;

impl Finder for Installed {
    fn find(&self, name: &str) -> Option<PathBuf> {
        (name == crate::resolve::KENTOS_CAD).then(|| PathBuf::from("/opt/kentos/bin").join(name))
    }
}

/// The launcher with the snapshot's data.
pub fn sample() -> Launcher {
    let mut launcher = Launcher::new(
        Folders {
            beside: Some(PathBuf::from("/opt/kentos/bin/uygulamalar")),
            user: Some(Path::new(HOME).join(".local/share/kentos/uygulamalar")),
        },
        Locations::memory(),
        Box::new(Installed),
        Processes::quiet(),
        None,
    );
    launcher.set_registry(Registry::builtin());
    launcher.now = NOW;
    let minutes = |m: u64| NOW - m * 60_000;
    let list = [
        (
            format!("{HOME}/Projeler/Konut/konut-3-1.kcad"),
            minutes(2),
            "334 nesne · Yerel",
            Some(Module::Architecture),
        ),
        (
            format!("{HOME}/Projeler/Kızılay/imar-plani.kcad"),
            minutes(3 * 60 + 10),
            "1.284 nesne · TUREF / TM33",
            Some(Module::Map),
        ),
        (
            format!("{HOME}/Projeler/Vadi/havza.kcad"),
            minutes(26 * 60),
            "8 nesne · TUREF / TM36",
            Some(Module::Gis),
        ),
        (
            format!("{HOME}/Projeler/Kızılay/aplikasyon.dxf"),
            minutes(4 * 24 * 60),
            "54 nesne · TUREF / TM33",
            None,
        ),
        (
            "/media/arsiv/eski-pafta.kcad".to_owned(),
            minutes(64 * 24 * 60),
            "2.917 nesne · ED50 / 3° 33",
            Some(Module::Map),
        ),
    ];
    let list: Vec<Recent> = list
        .into_iter()
        .map(|(path, at, info, module)| {
            let path = PathBuf::from(path);
            Recent {
                name: path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                path,
                at,
                info: info.to_owned(),
                module,
            }
        })
        .collect();
    launcher.set_recent(recent::rows(
        &list,
        NOW,
        |p| !p.starts_with("/media"),
        Some(Path::new(HOME)),
    ));
    launcher
}

/// Runs the command.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let out = args.next().filter(|a| !a.starts_with("--")).ok_or(USAGE)?;
    let mut page = "uygulamalar".to_owned();
    let mut mode = Mode::Dark;
    let mut size = Size::new(1440.0, 900.0);
    let mut text_size = Typography::DEFAULT.size;
    let mut scale = 1.0;
    let mut selected = None;
    let mut notices = false;
    let mut inputs = Vec::new();
    while let Some(arg) = args.next() {
        let mut value = || {
            args.next()
                .ok_or_else(|| format!("{arg} bir değer bekliyor.\n\n{USAGE}"))
        };
        match arg.as_str() {
            "--sayfa" => page = value()?,
            "--tema" => {
                mode = match value()?.as_str() {
                    "acik" | "açık" => Mode::Light,
                    "koyu" => Mode::Dark,
                    other => return Err(format!("{other}: tema koyu ya da acik olur.")),
                }
            }
            "--boyut" => {
                let v = value()?;
                size = v
                    .split_once('x')
                    .and_then(|(w, h)| Some(Size::new(w.parse().ok()?, h.parse().ok()?)))
                    .ok_or(format!("{v}: boyut 1440x900 biçiminde olur."))?;
            }
            "--yazi" => {
                let v = value()?;
                text_size = v
                    .parse::<f32>()
                    .ok()
                    .filter(|s| Typography::SIZES.contains(s))
                    .ok_or(format!(
                        "{v}: yazı boyutu 11 ile 18 arasında bir sayı olur."
                    ))?;
            }
            "--olcek" => {
                let v = value()?;
                scale = v
                    .parse::<f32>()
                    .ok()
                    .filter(|s| *s > 0.0 && *s <= 4.0)
                    .ok_or(format!("{v}: ölçek 0 ile 4 arasında bir sayı olur."))?;
            }
            "--secili" => selected = Some(value()?),
            "--bildirimler" => notices = true,
            "--imlec" => {
                let v = value()?;
                let point = v
                    .split_once(',')
                    .and_then(|(x, y)| {
                        Some(Point::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
                    })
                    .ok_or(format!("{v}: nokta x,y piksel biçiminde olur."))?;
                inputs.push(Input::Move(point));
            }
            "--yardim" | "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => return Err(format!("{other}: bilinmeyen seçenek.\n\n{USAGE}")),
        }
    }
    typography::set(Typography {
        size: text_size,
        ..Typography::DEFAULT
    });
    kentos_ui::theme::motion::set_reduced(true);

    let mut snapshot = Snapshot::new(size).map_err(|e| e.to_string())?.scale(scale);
    if page == "ikonlar" {
        let theme = kentos_ui::theme::theme(Mode::Dark, Accent::default());
        let image = snapshot.render(icon_sheet(), &theme);
        return image
            .save(&out)
            .map_err(|e| format!("{out}: yazılamadı ({e})"));
    }

    let mut launcher = sample();
    launcher.settings.theme = match mode {
        Mode::Light => crate::settings::ThemeChoice::Light,
        _ => crate::settings::ThemeChoice::Dark,
    };
    launcher.settings.text_size = text_size;
    launcher.page = match page.as_str() {
        "uygulamalar" | "ekle" => Page::Apps,
        "son" => Page::Recent,
        "guncellemeler" => Page::Updates,
        "indirmeler" => Page::Downloads,
        "ayarlar" => Page::Settings,
        other => return Err(format!("{other}: böyle bir sayfa yok.\n\n{USAGE}")),
    };
    if let Some(id) = selected {
        if launcher.registry.get(&id).is_none() {
            return Err(format!("{id}: böyle bir uygulama yok."));
        }
        launcher.selected = Some(id);
    }
    if page == "ekle" {
        let mut draft = Draft::new();
        draft.name = "Parsel raporu".into();
        draft.note = "Python betiği: ada ve parsel raporu".into();
        draft.program = "python3".into();
        draft.args = "rapor.py --ada 1244 {dosya}".into();
        draft.tone = Tone::Orange;
        draft.glyph = Glyph::Terminal;
        launcher.dialog = Some(Dialog::Edit(Box::new(draft)));
    }
    if notices {
        launcher.notices.push(
            Severity::Warning,
            "Uygulama yok sayıldı: “launch.kind” “exe”; program, url ya da kentos olur.".to_owned(),
            Some(format!(
                "{HOME}/.local/share/kentos/uygulamalar/eski-arac.json"
            )),
            NOW - 5 * 60_000,
        );
        launcher.notices.push(
            Severity::Error,
            "Parsel raporu 1 çıkış koduyla kapandı. Son satırı: “ModuleNotFoundError: No module named 'pandas'”.".to_owned(),
            None,
            NOW - 60_000,
        );
        // Opened, as the bell opens it: what it shows is read.
        launcher.notices_open = true;
        launcher.notices.read_all();
    }

    let mut update = |launcher: &mut Launcher, message: Message| {
        // Nothing the snapshot runs starts a process or writes a file: the
        // update's tasks (dialogs, the system's theme) are not run.
        let _ = launcher.update(message);
        launcher.now = NOW;
    };
    snapshot.settle(&mut launcher, view::view, &mut update);
    for input in inputs {
        snapshot.input(&mut launcher, view::view, &mut update, input);
    }
    let theme = launcher.theme();
    snapshot
        .render(view::view(&launcher), &theme)
        .save(&out)
        .map_err(|e| format!("{out}: yazılamadı ({e})"))
}

/// The tiles' glyphs and the launcher's own marks, light and dark, at the
/// sizes they are drawn at: for choosing and for review.
fn icon_sheet<'a>() -> Element<'a, Message> {
    let half = |mode: Mode| -> Element<'a, Message> {
        let theme = kentos_ui::theme::theme(mode, Accent::default());
        let title = if mode == Mode::Dark { "Koyu" } else { "Açık" };
        let mut rows = Column::new().spacing(10);
        let label = |s: &'a str, w: f32| container(text(s).size(12)).center_x(w);
        let header = row![
            text("").width(130),
            label("16", 28.0),
            label("20", 32.0),
            label("32", 44.0),
            label("48", 60.0),
            label("karo 32", 48.0),
            label("40", 56.0),
            label("48", 64.0),
            label("56", 72.0),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        rows = rows.push(header);
        let modules = [
            (Glyph::Mimari, Tone::Purple, "mimari · KentOS Mimari"),
            (Glyph::Harita, Tone::Blue, "harita · KentOS Harita"),
            (Glyph::Cbs, Tone::Green, "cbs · KentOS GIS"),
            (Glyph::Yol, Tone::Yellow, "yol · KentOS YOL"),
            (Glyph::Rapor, Tone::Orange, "rapor · KentOS Rapor"),
            (Glyph::Uygulama, Tone::Gray, "uygulama"),
            (Glyph::Terminal, Tone::Teal, "terminal"),
            (Glyph::Web, Tone::Blue, "web"),
            (Glyph::Tablo, Tone::Green, "tablo"),
            (Glyph::Grafik, Tone::Red, "grafik"),
            (Glyph::Klasor, Tone::Gray, "klasor"),
        ];
        for (glyph, tone, name) in modules {
            let mut line = Row::new().spacing(12).align_y(Alignment::Center);
            line = line.push(text(name).size(12).width(130));
            for size in [16.0, 20.0, 32.0, 48.0] {
                line = line
                    .push(container(icon(Icon::Svg(glyph.svg())).size(size)).center_x(size + 12.0));
            }
            for side in [32.0, 40.0, 48.0, 56.0] {
                line = line.push(container(view::tile(glyph, tone, side)).center_x(side + 16.0));
            }
            rows = rows.push(line);
        }
        for (markup, name) in [
            (glyphs::BELL, "zil · Bildirimler"),
            (glyphs::SETTINGS, "ayarlar · Ayarlar"),
            (glyphs::APPS, "uygulamalar · gezinme"),
            (glyphs::EXTERNAL, "dışarıda aç · web"),
        ] {
            let mut line = Row::new().spacing(12).align_y(Alignment::Center);
            line = line.push(text(name).size(12).width(130));
            for size in [16.0, 20.0, 32.0, 48.0] {
                line =
                    line.push(container(icon(Icon::Svg(markup)).size(size)).center_x(size + 12.0));
            }
            rows = rows.push(line);
        }
        let panel = container(
            column![text(title).size(16).font(typography::ui_strong()), rows].spacing(16),
        )
        .padding(24)
        .width(Fill)
        .height(Fill)
        .style(move |theme: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(Tokens::of(theme).surface)),
            text_color: Some(Tokens::of(theme).text),
            ..iced::widget::container::Style::default()
        });
        themer(Some(theme), panel).into()
    };
    row![half(Mode::Light), half(Mode::Dark)]
        .height(Fill)
        .into()
}
