//! Ekransız görüntü alt komutu: vitrini pencere açmadan çizip PNG'ye yazar.
//! Ekran kapalı ya da kilitliyken de çalışır.
//!
//! ```sh
//! cargo run -p showcase -- snapshot ekran.png
//! cargo run -p showcase -- snapshot agac.png --senaryo agac --sag-tikla 1250,330
//! cargo run -p showcase -- snapshot galeri.png --senaryo galeri --sayfa veri --boyut 1440x2400
//! ```
//!
//! Senaryo uygulamayı mesajlarla hazırlar. Girdiler (`--imlec`, `--tikla`,
//! `--sag-tikla`, `--tekerlek`, `--tus`, `--yaz`) ardından, verildikleri
//! sırayla arayüze verilir; konumlar pencere koordinatıdır ve görüntüdeki
//! piksellerle aynıdır (ölçek 1 iken).

use iced::keyboard::key::Named;
use iced::{Point, Size};

use kentos_rc::attribute::query::Edit;
use kentos_rc::attribute::{Combinator, Date, ObjectId, Operator, Value};
use kentos_rc::snapshot::{Input, Snapshot};
use kentos_rc::spatial::{FeatureRef, SelectionMode};
use kentos_rc::widget::inspector::Event as Inspector;

use crate::app::{Showcase, WINDOW_SIZE};
use crate::gallery::Page;
use crate::layer_tree::NodeId;
use crate::message::{Message, QueryPurpose, RibbonTab};
use crate::table::Column;

/// Senaryolar ve açıklamaları.
const SCENARIOS: [(&str, &str); 10] = [
    ("bos", "açılış durumu"),
    (
        "secim",
        "Marmara şehirleri öznitelikle seçilmiş, tablo nüfusa göre sıralı",
    ),
    (
        "agac",
        "katman ağacı: şehir alt katmanları açık, bir alt katman ve grup gizli",
    ),
    ("takvim", "yol seçili, nesne inceleyicide takvim açık"),
    ("nesne", "yol seçili, nesne seçici listesi açık"),
    ("sorgu", "öznitelikle seç penceresi, iki koşul"),
    ("sorgu-hata", "öznitelikle seç penceresi, hatalı koşulla"),
    (
        "filtre",
        "yollar filtrelenmiş, haritadan varlık seçimi sürüyor",
    ),
    ("yardim", "kısayollar penceresi"),
    ("galeri", "galeri; sayfa --sayfa ile seçilir"),
];

/// Galeri sayfalarının komut satırı adları.
const PAGES: [(&str, Page); 8] = [
    ("renkler", Page::Colors),
    ("yazi", Page::Typography),
    ("ikonlar", Page::Icons),
    ("dugmeler", Page::Buttons),
    ("veri", Page::Data),
    ("cerceve", Page::Frame),
    ("oznitelikler", Page::Attributes),
    ("mekansal", Page::Spatial),
];

const USAGE: &str = "\
Kullanım: showcase snapshot <çıktı.png> [seçenekler]

Seçenekler:
  --senaryo <ad>        uygulamanın hazırlanacağı durum (varsayılan: bos)
  --sayfa <ad>          galeri senaryosunda sayfa
  --boyut <G>x<Y>       pencere boyutu (varsayılan: 1440x900)
  --olcek <katsayı>     piksel yoğunluğu (varsayılan: 1)
  --tema acik|koyu      tema (varsayılan: koyu)

Girdiler (verildikleri sırayla):
  --imlec <x>,<y>       imleci taşır
  --tikla <x>,<y>       sol tık
  --sag-tikla <x>,<y>   sağ tık
  --tekerlek <x>,<y>,<satır>
  --tus <ad>            asagi, yukari, sag, sol, enter, esc, sekme, bosluk
  --yaz <metin>         metin yazar";

/// Alt komutu çalıştırır.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let mut output = None;
    let mut scenario = "bos".to_owned();
    let mut page = None;
    let mut size = WINDOW_SIZE;
    let mut scale = 1.0;
    let mut light = false;
    let mut inputs = Vec::new();

    while let Some(argument) = args.next() {
        let mut value = || {
            args.next()
                .ok_or_else(|| format!("{argument} bir değer bekliyor.\n\n{USAGE}"))
        };

        match argument.as_str() {
            "--yardim" | "--help" | "-h" => {
                println!("{USAGE}\n\nSenaryolar:");

                for (name, description) in SCENARIOS {
                    println!("  {name:<12} {description}");
                }

                println!(
                    "\nGaleri sayfaları: {}",
                    PAGES.map(|(name, _)| name).join(", ")
                );
                return Ok(());
            }
            "--senaryo" => scenario = value()?,
            "--sayfa" => page = Some(value()?),
            "--boyut" => size = parse_size(&value()?)?,
            "--olcek" => scale = parse_number(&value()?)?,
            "--tema" => light = value()? == "acik",
            "--imlec" => inputs.push(Input::Move(parse_point(&value()?)?)),
            "--tikla" => inputs.push(Input::Click(parse_point(&value()?)?)),
            "--sag-tikla" => inputs.push(Input::RightClick(parse_point(&value()?)?)),
            "--tekerlek" => {
                let value = value()?;
                let (point, lines) = value
                    .rsplit_once(',')
                    .ok_or_else(|| format!("Tekerlek x,y,satır biçiminde olmalı: {value}"))?;

                inputs.push(Input::Scroll(parse_point(point)?, parse_number(lines)?));
            }
            "--tus" => inputs.push(Input::Key(parse_key(&value()?)?)),
            "--yaz" => inputs.push(Input::Type(value()?)),
            path if !path.starts_with("--") && output.is_none() => output = Some(path.to_owned()),
            other => return Err(format!("Bilinmeyen seçenek: {other}\n\n{USAGE}")),
        }
    }

    let output = output.ok_or_else(|| format!("Çıktı dosyası verilmedi.\n\n{USAGE}"))?;

    let mut app = Showcase::new();
    prepare(&mut app, &scenario, page.as_deref())?;

    if light {
        let _ = app.update(Message::ToggleTheme);
    }

    let mut snapshot = Snapshot::new(size)
        .map_err(|error| error.to_string())?
        .scale(scale);
    let mut update = |app: &mut Showcase, message: Message| {
        let _ = app.update(message);
    };

    snapshot.settle(&mut app, Showcase::view, &mut update);

    for input in inputs {
        snapshot.input(&mut app, Showcase::view, &mut update, input);
    }

    let image = snapshot.render(app.view(), &app.theme());

    image
        .save(&output)
        .map_err(|error| format!("{output} yazılamadı: {error}"))?;

    println!(
        "{output}: {}×{} piksel, çizici {}",
        image.width,
        image.height,
        snapshot.renderer_name()
    );

    Ok(())
}

/// Senaryonun durumunu mesajlarla kurar.
fn prepare(app: &mut Showcase, scenario: &str, page: Option<&str>) -> Result<(), String> {
    let mut send = |message: Message| {
        let _ = app.update(message);
    };

    let road = FeatureRef::new(3, ObjectId(2));

    match scenario {
        "bos" => {}
        "secim" => {
            send(Message::QueryOpened(QueryPurpose::Select));
            send(Message::QueryEdited(Edit::Field(0, 1)));
            send(Message::QueryEdited(Edit::Value(0, "Marmara".into())));
            send(Message::QueryApplied);
            send(Message::TableSort(Column::Field(2)));
            send(Message::TableSort(Column::Field(2)));
        }
        "agac" => {
            send(Message::TreeToggled(NodeId::Layer(1)));
            send(Message::TreeChecked(NodeId::Sublayer(3, 2), false));
            send(Message::TreeChecked(NodeId::Group(2), false));
            send(Message::TreeSelected(NodeId::Layer(1)));
        }
        "takvim" | "nesne" => {
            send(Message::LayerActivated(3));
            send(Message::TableRowPressed(road));
            send(Message::Inspector(Inspector::Set {
                id: 3,
                value: Value::Real(120.0),
            }));
            send(Message::Inspector(Inspector::Set {
                id: 6,
                value: Date::new(2026, 9, 8).map_or(Value::Null, Value::Date),
            }));
            send(Message::Inspector(Inspector::Expand(Some(
                if scenario == "takvim" { 6 } else { 4 },
            ))));
        }
        "sorgu" | "sorgu-hata" => {
            send(Message::QueryOpened(QueryPurpose::Select));
            send(Message::QueryEdited(Edit::Field(0, 1)));
            send(Message::QueryEdited(Edit::Value(0, "Marmara".into())));
            send(Message::QueryEdited(Edit::Add));
            send(Message::QueryEdited(Edit::Field(1, 2)));
            send(Message::QueryEdited(Edit::Operator(1, Operator::Greater)));
            send(Message::QueryEdited(Edit::Value(1, "2.000.000".into())));

            if scenario == "sorgu-hata" {
                send(Message::QueryEdited(Edit::Add));
                send(Message::QueryEdited(Edit::Field(2, 3)));
                send(Message::QueryEdited(Edit::Operator(
                    2,
                    Operator::LessOrEqual,
                )));
                send(Message::QueryEdited(Edit::Value(2, "4O".into())));
            }

            send(Message::QueryEdited(Edit::Combinator(Combinator::Any)));
            send(Message::QueryModeSelected(SelectionMode::Add));
        }
        "filtre" => {
            send(Message::ZoomToLayer(3));
            send(Message::QueryOpened(QueryPurpose::Filter));
            send(Message::QueryEdited(Edit::Field(0, 1)));
            send(Message::QueryEdited(Edit::Value(0, "Otoyol".into())));
            send(Message::QueryApplied);
            send(Message::TableSort(Column::Field(0)));
            send(Message::TableRowPressed(road));
            send(Message::Inspector(Inspector::Pick(5)));
        }
        "yardim" => send(Message::HelpToggled),
        "galeri" => {
            let page = match page {
                Some(name) => PAGES
                    .iter()
                    .find(|(candidate, _)| *candidate == name)
                    .map(|(_, page)| *page)
                    .ok_or_else(|| format!("Bilinmeyen galeri sayfası: {name}"))?,
                None => Page::Colors,
            };

            send(Message::RibbonTabSelected(RibbonTab::Gallery));
            send(Message::GalleryPageSelected(page));
        }
        other => {
            return Err(format!(
                "Bilinmeyen senaryo: {other}. Senaryolar: {}",
                SCENARIOS.map(|(name, _)| name).join(", ")
            ));
        }
    }

    Ok(())
}

fn parse_number(text: &str) -> Result<f32, String> {
    text.trim()
        .parse()
        .map_err(|_| format!("Sayı bekleniyordu: {text}"))
}

fn parse_point(text: &str) -> Result<Point, String> {
    let (x, y) = text
        .split_once(',')
        .ok_or_else(|| format!("Konum x,y biçiminde olmalı: {text}"))?;

    Ok(Point::new(parse_number(x)?, parse_number(y)?))
}

fn parse_size(text: &str) -> Result<Size, String> {
    let (width, height) = text
        .split_once('x')
        .ok_or_else(|| format!("Boyut GxY biçiminde olmalı: {text}"))?;

    Ok(Size::new(parse_number(width)?, parse_number(height)?))
}

fn parse_key(name: &str) -> Result<Named, String> {
    Ok(match name {
        "asagi" => Named::ArrowDown,
        "yukari" => Named::ArrowUp,
        "sag" => Named::ArrowRight,
        "sol" => Named::ArrowLeft,
        "enter" => Named::Enter,
        "esc" => Named::Escape,
        "sekme" => Named::Tab,
        "bosluk" => Named::Space,
        other => return Err(format!("Bilinmeyen tuş: {other}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_are_parsed() {
        assert_eq!(parse_point("10,20.5"), Ok(Point::new(10.0, 20.5)));
        assert_eq!(parse_size("1440x900"), Ok(Size::new(1440.0, 900.0)));
        assert_eq!(parse_key("esc"), Ok(Named::Escape));
        assert!(parse_point("10").is_err());
    }

    #[test]
    fn every_scenario_prepares_the_app() {
        for (name, _) in SCENARIOS {
            let mut app = Showcase::new();
            assert_eq!(prepare(&mut app, name, Some("veri")), Ok(()), "{name}");
        }

        assert!(prepare(&mut Showcase::new(), "yok", None).is_err());
    }
}
