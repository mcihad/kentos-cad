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
//! `--sag-tikla`, `--tekerlek`, `--surukle`, `--bas`, `--birak`, `--tus`,
//! `--yaz`) ardından, verildikleri sırayla arayüze verilir; konumlar pencere koordinatıdır ve görüntüdeki
//! piksellerle aynıdır (ölçek 1 iken).

use iced::keyboard::key::Named;
use iced::{Point, Rectangle, Size};

use kentos_rc::attribute::query::Edit;
use kentos_rc::attribute::{Combinator, Date, ObjectId, Operator, Value};
use kentos_rc::snapshot::{Input, Snapshot};
use kentos_rc::spatial::model_space::{Backdrop, Event as ModelSpace};
use kentos_rc::spatial::{FeatureRef, LonLat, SelectionMode, Tool};
use kentos_rc::theme::typography::{Family, Mono, Typography};
use kentos_rc::theme::{Accent, Mode};
use kentos_rc::widget::docking;
use kentos_rc::widget::inspector::Event as Inspector;

use crate::app::{DRAWING_LAYER, Showcase, WINDOW_SIZE};
use crate::gallery::Page;
use crate::import::Source;
use crate::layer_tree::NodeId;
use crate::message::{DockPanel, Message, Pane, QueryPurpose, RibbonTab};
use crate::properties::{self, Section};
use crate::table::Column;

/// Senaryolar ve açıklamaları.
const SCENARIOS: [(&str, &str); 20] = [
    ("bos", "açılış durumu"),
    (
        "secim",
        "Marmara şehirleri öznitelikle seçilmiş, tablo nüfusa göre sıralı",
    ),
    (
        "agac",
        "katman ağacı: şehir alt katmanları açık, bir alt katman ve grup gizli",
    ),
    (
        "yol",
        "yol seçili, hız sınırı ve bakım tarihi değiştirilmiş; takvim ve listeler --tikla ile açılır",
    ),
    ("sorgu", "öznitelikle seç penceresi, iki koşul"),
    ("sorgu-hata", "öznitelikle seç penceresi, hatalı koşulla"),
    (
        "filtre",
        "yollar filtrelenmiş, haritadan varlık seçimi sürüyor",
    ),
    ("yardim", "kısayollar penceresi"),
    (
        "cizim",
        "çoklu çizgi sürüyor: istem ve seçenekleri; öneriler --tikla 800,851 --yaz ile açılır",
    ),
    (
        "gecmis",
        "komut geçmişi açık (F2): yazılan komutlar, yanıtlar ve bir hata",
    ),
    (
        "pencereler",
        "kayan pencereler: ölçüm sürüyor, koordinata git ve katman stili açık",
    ),
    (
        "bildirimler",
        "bildirimler: yinelenen bilgi, uyarı, kalıcı hata ve geri alınabilir silme",
    ),
    (
        "gorevler",
        "görevler paneli: süren, sıradaki, biten, başarısız ve iptal edilen işler",
    ),
    (
        "onay",
        "çizimleri temizle: onay kutusu; Enter onaylar, Esc vazgeçer",
    ),
    (
        "bos-durumlar",
        "katmanlar gizli, örnek veri silinmeye çalışılmış: boş harita, salt okunur şeridi, boş tablo",
    ),
    (
        "sihirbaz",
        "veri içe aktarma sihirbazı: istasyonlar.csv seçili; adımlar İleri ile, --adim ile",
    ),
    (
        "ozellikler",
        "Şehirler katmanının özellikleri: sembolizasyon, kaydedilmemiş değişiklik",
    ),
    (
        "yuva",
        "yuva yeniden düzenlenmiş: özellikler katmanların yanında sekme, görevler yüzen pencere",
    ),
    (
        "duzen",
        "Düzen 1 sekmesi: A3 kâğıtta harita çerçevesi ve antet; üç düzen açık",
    ),
    ("galeri", "galeri; sayfa --sayfa ile seçilir"),
];

/// Galeri sayfalarının komut satırı adları.
const PAGES: [(&str, Page); 11] = [
    ("renkler", Page::Colors),
    ("yazi", Page::Typography),
    ("ikonlar", Page::Icons),
    ("dugmeler", Page::Buttons),
    ("veri", Page::Data),
    ("cerceve", Page::Frame),
    ("yerlesim", Page::Layout),
    ("girdiler", Page::Inputs),
    ("geri-bildirim", Page::Feedback),
    ("oznitelikler", Page::Attributes),
    ("mekansal", Page::Spatial),
];

const USAGE: &str = "\
Kullanım: showcase snapshot <çıktı.png> [seçenekler]

Seçenekler:
  --senaryo <ad>        uygulamanın hazırlanacağı durum (varsayılan: bos)
  --sayfa <ad>          galeri senaryosunda sayfa; sihirbazda adım (1-4)
  --boyut <G>x<Y>       pencere boyutu (varsayılan: 1440x900)
  --olcek <katsayı>     piksel yoğunluğu (varsayılan: 1)
  --tema <ad>           koyu, acik, gece, karsitlik (varsayılan: koyu)
  --vurgu <renk>        mavi, turkuaz, yesil, kehribar, turuncu, pembe, mor, gri ya da #RRGGBB
  --zemin <ad>          harita zemini: tema, arduvaz, siyah, kagit
  --yazi <aile>         ibm-plex-sans, inter, plus-jakarta-sans
  --esaralikli <aile>   ibm-plex-mono, jetbrains-mono
  --punto <boyut>       gövde metninin boyutu, 11-18 (varsayılan: 13)

Girdiler (verildikleri sırayla):
  --imlec <x>,<y>       imleci taşır
  --tikla <x>,<y>       sol tık
  --sag-tikla <x>,<y>   sağ tık
  --tekerlek <x>,<y>,<satır>
  --surukle <x>,<y>,<x>,<y>   sol tuşla sürükler (ör. panel sekmesi, yuvanın kenarı)
  --bas <x>,<y>         sol tuşa basar ve tutar; ardından --imlec sürükler
  --birak <x>,<y>       tutulan sol tuşu bırakır
  --tus <ad>            asagi, yukari, sag, sol, enter, esc, sekme, bosluk
  --yaz <metin>         metin yazar";

/// Alt komutu çalıştırır.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let mut output = None;
    let mut scenario = "bos".to_owned();
    let mut page = None;
    let mut size = WINDOW_SIZE;
    let mut scale = 1.0;
    let mut mode = None;
    let mut accent = None;
    let mut backdrop = None;
    let mut inputs = Vec::new();
    let mut typography = Typography::DEFAULT;

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
            "--tema" => {
                let name = value()?;
                mode = Some(match name.as_str() {
                    "koyu" => Mode::Dark,
                    "acik" | "aydinlik" => Mode::Light,
                    "gece" => Mode::Night,
                    "karsitlik" => Mode::HighContrast,
                    other => return Err(format!("Bilinmeyen tema: {other}")),
                });
            }
            "--vurgu" => {
                let name = value()?;
                accent = Some(
                    Accent::parse(&name)
                        .ok_or_else(|| format!("Bilinmeyen vurgu rengi: {name}"))?,
                );
            }
            "--zemin" => {
                let name = value()?;
                backdrop = Some(
                    Backdrop::parse(&name)
                        .ok_or_else(|| format!("Bilinmeyen harita zemini: {name}"))?,
                );
            }
            "--yazi" => {
                let name = value()?;
                typography.family = Family::ALL
                    .into_iter()
                    .find(|family| key_of(family.name()) == name)
                    .ok_or_else(|| format!("Bilinmeyen yazı ailesi: {name}"))?;
            }
            "--esaralikli" => {
                let name = value()?;
                typography.mono = Mono::ALL
                    .into_iter()
                    .find(|mono| key_of(mono.name()) == name)
                    .ok_or_else(|| format!("Bilinmeyen eş aralıklı aile: {name}"))?;
            }
            "--punto" => typography.size = parse_number(&value()?)?,
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
            "--surukle" => {
                let value = value()?;
                let numbers: Vec<&str> = value.split(',').collect();

                let [x1, y1, x2, y2] = numbers[..] else {
                    return Err(format!("Sürükleme x,y,x,y biçiminde olmalı: {value}"));
                };

                inputs.push(Input::Drag(
                    Point::new(parse_number(x1)?, parse_number(y1)?),
                    Point::new(parse_number(x2)?, parse_number(y2)?),
                ));
            }
            "--bas" => inputs.push(Input::Press(parse_point(&value()?)?)),
            "--birak" => inputs.push(Input::Release(parse_point(&value()?)?)),
            "--tus" => inputs.push(Input::Key(parse_key(&value()?)?)),
            "--yaz" => inputs.push(Input::Type(value()?)),
            path if !path.starts_with("--") && output.is_none() => output = Some(path.to_owned()),
            other => return Err(format!("Bilinmeyen seçenek: {other}\n\n{USAGE}")),
        }
    }

    let output = output.ok_or_else(|| format!("Çıktı dosyası verilmedi.\n\n{USAGE}"))?;

    // Yazı ayarı uygulama kurulmadan verilir; uygulama onu okur.
    kentos_rc::theme::typography::set(typography);

    let mut app = Showcase::new();
    prepare(&mut app, &scenario, page.as_deref())?;

    if let Some(mode) = mode {
        let _ = app.update(Message::ThemeSelected(mode));
    }

    if let Some(accent) = accent {
        let _ = app.update(Message::AccentChanged(accent));
    }

    if let Some(backdrop) = backdrop {
        let _ = app.update(Message::BackdropChanged(backdrop));
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
        "yol" => {
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
        "cizim" | "gecmis" => {
            send(Message::CommandInput("merhaba".to_owned()));
            send(Message::CommandSubmitted);
            send(Message::CommandRun("OLC".to_owned()));

            for point in [LonLat::new(28.98, 41.01), LonLat::new(32.85, 39.93)] {
                send(Message::ModelSpace(ModelSpace::PointPicked(point)));
            }

            send(Message::CommandRun("CCIZGI".to_owned()));

            for point in [
                LonLat::new(32.85, 39.93),
                LonLat::new(35.48, 38.72),
                LonLat::new(37.02, 39.75),
            ] {
                send(Message::ModelSpace(ModelSpace::PointPicked(point)));
            }

            send(Message::ModelSpace(ModelSpace::CursorMoved(LonLat::new(
                38.4, 40.6,
            ))));

            if scenario == "gecmis" {
                send(Message::CommandHistoryToggled);
            }
        }
        "pencereler" => {
            send(Message::CommandRun("OLC".to_owned()));

            for point in [
                LonLat::new(27.14, 38.42),
                LonLat::new(29.06, 40.19),
                LonLat::new(32.85, 39.93),
                LonLat::new(35.48, 38.72),
            ] {
                send(Message::ModelSpace(ModelSpace::PointPicked(point)));
            }

            send(Message::PaneToggled(Pane::GoTo));
            send(Message::StyleOpened(2));
            send(Message::ModelSpace(ModelSpace::CursorMoved(LonLat::new(
                36.2, 37.1,
            ))));
        }
        "bildirimler" => {
            let ankara = LonLat::new(32.85, 39.93);

            send(Message::CopyCoordinates(ankara));
            send(Message::CopyCoordinates(ankara));
            send(Message::SelectFeature(
                FeatureRef::new(1, ObjectId(2)),
                SelectionMode::New,
            ));
            send(Message::DeleteSelection);
            send(Message::ToolSelected(Tool::Point));

            for point in [LonLat::new(30.52, 39.78), LonLat::new(34.63, 36.8)] {
                send(Message::ModelSpace(ModelSpace::PointPicked(point)));
            }

            send(Message::ToolSelected(Tool::Select));
            send(Message::SelectNode(NodeId::Layer(DRAWING_LAYER)));
            send(Message::DeleteSelection);
        }
        "gorevler" => {
            let mut run = |message: Message, ticks: usize| {
                let _ = app.update(message);

                for _ in 0..ticks {
                    let _ = app.update(Message::JobTick);
                }
            };

            // Sırayla: DXF başarısız olur, PNG iptal edilir, dizin biter,
            // GeoJSON sürer, PDF sırada bekler.
            run(Message::ExportPressed("DXF"), 21);
            run(Message::ExportPressed("PNG"), 4);
            run(Message::JobCancelled(2), 0);
            run(Message::IndexRequested, 29);
            run(Message::ExportPressed("GeoJSON"), 15);
            run(Message::ExportPressed("PDF"), 0);
            run(Message::PanelShown(DockPanel::Tasks), 0);

            app.toasts.clear();
            return Ok(());
        }
        "onay" => {
            send(Message::ToolSelected(Tool::Point));

            for point in [
                LonLat::new(30.52, 39.78),
                LonLat::new(34.63, 36.8),
                LonLat::new(38.3, 38.35),
            ] {
                send(Message::ModelSpace(ModelSpace::PointPicked(point)));
            }

            send(Message::ToolSelected(Tool::Select));
            send(Message::ClearDrawings);
        }
        "bos-durumlar" => {
            send(Message::SelectFeature(
                FeatureRef::new(1, ObjectId(2)),
                SelectionMode::New,
            ));
            send(Message::DeleteSelection);
            send(Message::HideAllLayers);
            send(Message::LayerActivated(DRAWING_LAYER));
        }
        "sihirbaz" => {
            send(Message::ImportOpened);
            send(Message::ImportSource(Source::Stations));

            let steps = page.map_or(Ok(0), |step| {
                step.parse::<usize>()
                    .map_err(|_| format!("Adım bir sayı olmalı: {step}"))
            })?;

            for _ in 1..steps.max(1) {
                send(Message::ImportNext);
            }
        }
        "ozellikler" => {
            send(Message::PropertiesOpened(1));
            send(Message::PropertiesSection(Section::Symbology));
            send(Message::PropertiesEdited(properties::Edit::Opacity(0.8)));
        }
        "yuva" => {
            let dock = |event| Message::Dock(event);

            send(dock(docking::Event::Moved(
                DockPanel::Details,
                docking::Target::Tab(docking::Slot::Docked(docking::Side::Right, 0), 1),
            )));
            send(dock(docking::Event::Moved(
                DockPanel::Tasks,
                docking::Target::Float(Rectangle::new(
                    Point::new(520.0, 150.0),
                    Size::new(380.0, 250.0),
                )),
            )));
            send(Message::ExportPressed("GeoJSON"));

            for _ in 0..12 {
                send(Message::JobTick);
            }

            app.toasts.clear();
            return Ok(());
        }
        "duzen" => {
            send(Message::SheetAdded);
            send(Message::SheetSelected(1));
        }
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

/// Ailenin komut satırındaki adı ("IBM Plex Sans" → "ibm-plex-sans").
fn key_of(name: &str) -> String {
    name.to_ascii_lowercase().replace(' ', "-")
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
            let page = match name {
                "galeri" => Some("veri"),
                "sihirbaz" => Some("3"),
                _ => None,
            };

            let mut app = Showcase::new();
            assert_eq!(prepare(&mut app, name, page), Ok(()), "{name}");
        }

        assert!(prepare(&mut Showcase::new(), "sihirbaz", Some("iki")).is_err());

        assert!(prepare(&mut Showcase::new(), "yok", None).is_err());
    }
}
