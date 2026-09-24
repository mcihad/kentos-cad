//! Durum çubuğu: imleç koordinatı, seçim, görüntüleme anahtarları, ölçek ve
//! koordinat sistemi.

use iced::widget::{Row, row, space};
use iced::{Center, Element};

use kentos_rc::icon::{Icon, Tone, icon};
use kentos_rc::label;
use kentos_rc::spatial::{LonLat, format};
use kentos_rc::style;
use kentos_rc::widget::Menu;
use kentos_rc::widget::StatusBar;
use kentos_rc::widget::progress;
use kentos_rc::widget::status_bar::{Readout, Toggle};

use crate::app::Showcase;
use crate::message::{CoordinateFormat, Message, Pane, Setting};

/// Ölçek menüsündeki standart harita ölçekleri: halihazır haritalardan
/// (1:1.000, 1:5.000) topoğrafik paftalara ve ülke haritalarına.
const SCALES: [f64; 12] = [
    1_000.0,
    2_000.0,
    5_000.0,
    10_000.0,
    25_000.0,
    50_000.0,
    100_000.0,
    250_000.0,
    500_000.0,
    1_000_000.0,
    2_500_000.0,
    5_000_000.0,
];

/// Koordinat göstergesinin genişliği; en uzun biçim (DMS) sığar ve imleç
/// hareket ettikçe çubuk kıpırdamaz.
const COORDINATES_WIDTH: f32 = 214.0;

impl Showcase {
    pub(super) fn status_bar(&self) -> Element<'_, Message> {
        let mut bar = StatusBar::new()
            .push(self.coordinates())
            .separator()
            .push(self.selection_readout())
            .spacer();

        if let Some(jobs) = self.jobs_readout() {
            bar = bar.push(jobs).separator();
        }

        let bar = Setting::ALL.into_iter().fold(bar, |bar, setting| {
            bar.push(
                Toggle::new(setting.label(), self.setting(setting))
                    .icon(setting.icon())
                    .shortcut(setting.shortcut())
                    .on_press(Message::Toggle(setting)),
            )
        });

        bar.separator()
            .push(self.scale_readout())
            .separator()
            .push(
                Readout::new(label::mono_caption("EPSG:3857"))
                    .icon(Icon::Globe)
                    .tip("WGS 84 / Pseudo-Mercator: metre, Web haritalarının izdüşümü"),
            )
            .into()
    }

    /// İmlecin koordinatı; imleç model alanının dışındayken son konum soluk
    /// gösterilir. Menüden biçim seçilir ve koordinat kopyalanır.
    fn coordinates(&self) -> Element<'_, Message> {
        let live = self.cursor.is_some();
        let location = self.cursor.or(self.last_cursor);
        let format = self.coordinate_format;

        let value = match location {
            Some(location) => coordinate_parts(location, format, live),
            None => label::caption("İmleç model alanında değil").into(),
        };

        Readout::new(value)
            .icon(Icon::Target)
            .width(COORDINATES_WIDTH)
            .menu(move || {
                let example = LonLat::new(28.9784, 41.0082);

                CoordinateFormat::ALL
                    .into_iter()
                    .fold(Menu::new().header("Koordinat biçimi"), |menu, option| {
                        menu.check(
                            option.label(),
                            option == format,
                            Message::CoordinateFormatSelected(option),
                        )
                        .shortcut(sample(location.unwrap_or(example), option))
                    })
                    .separator()
                    .item("Koordinatı kopyala", location.map(Message::CopyCoordinates))
                    .icon(Icon::Copy)
            })
            .into()
    }

    /// Arka plandaki işler: süren iş, yüzdesi ve sıradakiler; iş yokken
    /// başarısız iş sayısı. Tıklanınca görevler penceresi açılır.
    fn jobs_readout(&self) -> Option<Element<'_, Message>> {
        let open = Message::PaneToggled(Pane::Tasks);

        let content: Element<'_, Message> = if let Some(job) = self.jobs.running() {
            let mut content = row![
                progress::spinner().size(12.0),
                label::caption(job.activity()).style(style::text::default),
            ]
            .spacing(6)
            .align_y(Center);

            if let Some(done) = job.progress() {
                content = content.push(label::mono_caption(format!("%{:.0}", done * 100.0)));
            }

            if self.jobs.queued() > 0 {
                content = content.push(label::caption(format!("{} sırada", self.jobs.queued())));
            }

            content.into()
        } else if self.jobs.failed() > 0 {
            row![
                icon(Icon::Error).size(13.0).tone(Tone::Danger),
                label::caption(format!("{} iş başarısız", self.jobs.failed()))
                    .style(style::text::danger),
            ]
            .spacing(6)
            .align_y(Center)
            .into()
        } else {
            return None;
        };

        Some(
            Readout::new(content)
                .on_press(open)
                .tip("Görevler penceresini açar")
                .into(),
        )
    }

    /// Seçili öğe sayısı; menüsü seçimle yapılacak işleri sunar.
    fn selection_readout(&self) -> Element<'_, Message> {
        let count = self.selection.len();

        if count == 0 {
            return Readout::new(label::caption("Seçim yok"))
                .icon(Icon::Select)
                .into();
        }

        let layers = {
            let mut layers: Vec<usize> = self.selection.iter().map(|item| item.layer).collect();
            layers.sort_unstable();
            layers.dedup();
            layers.len()
        };

        let summary = if layers > 1 {
            format!("{count} seçili, {layers} katman")
        } else {
            format!("{count} seçili")
        };

        Readout::new(label::body(summary))
            .icon(Icon::Select)
            .menu(|| {
                Menu::new()
                    .item("Seçime odaklan", Message::FocusSelection)
                    .icon(Icon::Target)
                    .item("Seçimi tersine çevir", Message::InvertSelection)
                    .icon(Icon::InvertSelection)
                    .item("Tabloda yalnızca seçilenler", Message::TableSelectedOnly)
                    .icon(Icon::Table)
                    .separator()
                    .item("Seçimi kaldır", Message::ClearSelection)
                    .icon(Icon::ClearSelection)
                    .shortcut("Esc")
            })
            .into()
    }

    /// Görünümün ölçeği; menüden standart ölçeklerden biri seçilir.
    fn scale_readout(&self) -> Element<'_, Message> {
        let viewport = self.viewport;
        let current = viewport.scale_denominator();

        Readout::new(label::mono(format!("1:{}", format::integer(current))))
            .menu(move || {
                let menu = Menu::new().header(format!(
                    "Yakınlaştırma düzeyi {}",
                    format::pretty((viewport.zoom * 100.0).round() / 100.0)
                ));

                SCALES
                    .into_iter()
                    .filter(|&scale| {
                        let zoom = viewport.zoom_for_scale(scale);
                        (kentos_rc::spatial::projection::MIN_ZOOM
                            ..=kentos_rc::spatial::projection::MAX_ZOOM)
                            .contains(&zoom)
                    })
                    .fold(menu, |menu, scale| {
                        menu.check(
                            format!("1:{}", format::integer(scale)),
                            ((current - scale) / scale).abs() < 0.005,
                            Message::ScaleSelected(scale),
                        )
                    })
                    .separator()
                    .item("Tümünü gör", Message::FitAll)
                    .icon(Icon::ZoomExtents)
            })
            .into()
    }
}

/// Koordinatın iki bileşeni: değerler eş aralıklı, yön harfleri ve eksen
/// adları sönük. İmleç model alanının dışındaysa değerler de sönüktür.
fn coordinate_parts<'a>(
    location: LonLat,
    format: CoordinateFormat,
    live: bool,
) -> Element<'a, Message> {
    let value = |text: String| {
        label::mono(text).style(if live {
            style::text::default
        } else {
            style::text::muted
        })
    };

    let [first, second] = parts(location, format);

    let component = |(prefix, number, suffix): (&'static str, String, &'static str)| {
        let mut part = Row::new().spacing(3).align_y(Center);

        if !prefix.is_empty() {
            part = part.push(label::caption(prefix));
        }

        part = part.push(value(number));

        if !suffix.is_empty() {
            part = part.push(label::caption(suffix));
        }

        part
    };

    row![
        component(first),
        space::horizontal().width(10),
        component(second)
    ]
    .align_y(Center)
    .into()
}

/// Koordinatın biçime göre iki bileşeni: önek, sayı, sonek.
fn parts(location: LonLat, format: CoordinateFormat) -> [(&'static str, String, &'static str); 2] {
    let hemisphere =
        |value: f64, positive, negative| if value >= 0.0 { positive } else { negative };

    match format {
        CoordinateFormat::Decimal => [
            (
                "",
                format!("{:.5}°", location.lat.abs()),
                hemisphere(location.lat, "K", "G"),
            ),
            (
                "",
                format!("{:.5}°", location.lon.abs()),
                hemisphere(location.lon, "D", "B"),
            ),
        ],
        CoordinateFormat::Dms => {
            let dms = |value: f64| {
                let text = format::dms(value, "", "");
                text.trim_end().to_owned()
            };

            [
                ("", dms(location.lat), hemisphere(location.lat, "K", "G")),
                ("", dms(location.lon), hemisphere(location.lon, "D", "B")),
            ]
        }
        CoordinateFormat::Projected => {
            let (x, y) = location.web_mercator();

            [
                ("X", format::integer(x), ""),
                ("Y", format::integer(y), "m"),
            ]
        }
    }
}

/// Koordinatın biçimdeki tek satırlık örneği (menüde biçimin yanında).
fn sample(location: LonLat, format: CoordinateFormat) -> String {
    let [(prefix, number, suffix), _] = parts(location, format);

    [prefix, number.as_str(), suffix]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ISTANBUL: LonLat = LonLat::new(28.9784, 41.0082);

    #[test]
    fn coordinate_formats() {
        assert_eq!(sample(ISTANBUL, CoordinateFormat::Decimal), "41.00820° K");
        assert_eq!(sample(ISTANBUL, CoordinateFormat::Dms), "41°00'29.5\" K");
        assert_eq!(sample(ISTANBUL, CoordinateFormat::Projected), "X 3.225.861");

        let [_, (_, y, unit)] = parts(ISTANBUL, CoordinateFormat::Projected);
        assert_eq!((y.as_str(), unit), ("5.013.551", "m"));

        let [(_, _, south), (_, _, west)] =
            parts(LonLat::new(-77.03, -12.05), CoordinateFormat::Decimal);
        assert_eq!((south, west), ("G", "B"));
    }
}
