//! Veri içe aktarma sihirbazı: kaynak dosya, sütunlar, koordinat sistemi ve
//! özet.

use iced::widget::text::Wrapping;
use iced::widget::{Column, Row, column, container, pick_list, row, space, text_input};
use iced::{Center, Element, Fill};

use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::wizard::{self, Wizard};
use kentos_ui::widget::{Banner, EmptyState, overlay};

use crate::app::Showcase;
use crate::import::{self, CSV_COLUMNS, ImportWizard, PARK_FIELDS, SYSTEMS, Source};
use crate::message::Message;

/// Form satırlarındaki ad sütunu (12 piksellik metne göre).
const NAME_WIDTH: f32 = 128.0;

impl Showcase {
    pub(super) fn import_wizard<'a>(&'a self, wizard: &'a ImportWizard) -> Element<'a, Message> {
        let ready = wizard.is_ready(&self.layers);

        let body = match wizard.step {
            0 => source_step(wizard),
            1 => columns_step(wizard),
            2 => system_step(wizard),
            _ => summary_step(wizard, wizard.problem(&self.layers)),
        };

        let hint = wizard
            .problem(&self.layers)
            .unwrap_or_else(|| note(wizard).to_owned());

        overlay::blocking(
            Wizard::new("Veri içe aktar", import::STEPS)
                .current(wizard.step)
                .body(body)
                .hint(hint)
                .back(Message::ImportBack)
                .next(ready.then_some(Message::ImportNext))
                .finish("İçe aktar", ready.then_some(Message::ImportNext))
                .on_cancel(Message::ImportClosed),
        )
    }
}

/// Adım tamamken alt çubuktaki not.
fn note(wizard: &ImportWizard) -> &'static str {
    match (wizard.step, wizard.source) {
        (0, None) => "İçe aktarılacak dosyayı seçin.",
        (0, Some(_)) => "Dosya okunabiliyor; sütunlara geçin.",
        (1, _) => "Koordinatlar seçilen sütunlardan okunur.",
        (2, _) => "Değerler seçilen koordinat sisteminde yorumlanır.",
        _ => "İçe aktarma arka planda sürer; bitince katman ağacın en üstüne eklenir.",
    }
}

/// 1. adım: dosya listesi ve seçili dosyanın önizlemesi.
fn source_step<'a>(wizard: &'a ImportWizard) -> Element<'a, Message> {
    let files = Column::with_children(Source::ALL.into_iter().map(|source| {
        let failed = source.error().is_some();

        wizard::choice(
            row![
                icon(Icon::Document).size(16.0).tone(Tone::Muted),
                column![
                    label::strong(source.file()),
                    label::caption(format!("{}, {}", source.format(), source.size())),
                ]
                .spacing(1)
                .width(Fill),
                label::caption(source.summary()).style(if failed {
                    style::text::danger
                } else {
                    style::text::muted
                }),
            ]
            .spacing(10)
            .align_y(Center),
            wizard.source == Some(source),
            Message::ImportSource(source),
        )
    }))
    .spacing(4)
    .width(Fill);

    let preview: Element<'a, Message> = match wizard.source {
        None => EmptyState::new(Icon::Document, "Önizleme")
            .description("Soldan bir dosya seçin; ilk satırları burada görünür.")
            .into(),
        Some(source) => match source.error() {
            Some(reason) => EmptyState::error(format!("{} okunamadı", source.file()))
                .description(reason)
                .into(),
            None => column![
                label::caption("İlk satırlar"),
                container(
                    label::mono_caption(preview_text(source))
                        .style(style::text::default)
                        .wrapping(Wrapping::None),
                )
                .padding([8, 10])
                .width(Fill)
                .clip(true)
                .style(style::container::field_box),
            ]
            .spacing(6)
            .into(),
        },
    };

    row![
        column![label::caption("Belgeler › Veri"), files]
            .spacing(6)
            .width(Fill),
        container(preview).width(Fill).height(Fill),
    ]
    .spacing(16)
    .into()
}

/// Dosyanın ilk satırları, olduğu gibi.
fn preview_text(source: Source) -> String {
    match source {
        Source::Parks => "{\"type\": \"FeatureCollection\",\n \"features\": [\n  {\"type\": \"Feature\",\n   \"properties\": {\"ad\": \"Kaz Dağı\",\n     \"il\": \"Balıkesir\", \"kurulus\": 1994},\n   \"geometry\": {\"type\": \"Polygon\", …"
            .to_owned(),
        _ => std::iter::once(CSV_COLUMNS.join(","))
            .chain((0..5).map(|row| {
                (0..CSV_COLUMNS.len())
                    .map(|column| ImportWizard::sample(row, column))
                    .collect::<Vec<_>>()
                    .join(",")
            }))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// 2. adım: CSV'de koordinat sütunları ve önizleme; GeoJSON'da alanlar.
fn columns_step<'a>(wizard: &'a ImportWizard) -> Element<'a, Message> {
    let Some(source) = wizard.source else {
        return space::vertical().into();
    };

    if !source.has_columns() {
        let fields = Column::with_children(PARK_FIELDS.map(|(name, kind)| {
            row![
                label::mono(name).width(typography::scaled(NAME_WIDTH)),
                label::muted(kind),
            ]
            .spacing(12)
            .into()
        }))
        .spacing(6);

        return column![
            Banner::info("Geometri dosyanın içinde; koordinat sütunu seçmek gerekmez."),
            row![
                label::muted("Geometri").width(typography::scaled(NAME_WIDTH)),
                label::body(format!("{} ({} öğe)", source.geometry(), source.count())),
            ]
            .spacing(12),
            label::caption("Öznitelikler"),
            fields,
        ]
        .spacing(10)
        .into();
    }

    let picker = |current: Option<usize>, on_select: fn(usize) -> Message| {
        pick_list(
            CSV_COLUMNS.to_vec(),
            current.map(|column| CSV_COLUMNS[column]),
            move |name: &str| {
                on_select(
                    CSV_COLUMNS
                        .iter()
                        .position(|column| *column == name)
                        .unwrap_or(0),
                )
            },
        )
        .font(typography::mono())
        .text_size(typography::body())
        .padding([3, 8])
        .width(typography::scaled(180.0))
        .style(style::field::pick_list)
        .menu_style(style::field::menu)
    };

    let mut content = column![
        row![
            label::muted("Boylam (X)").width(typography::scaled(NAME_WIDTH)),
            picker(wizard.x, Message::ImportX),
            space::horizontal().width(24),
            label::muted("Enlem (Y)"),
            picker(wizard.y, Message::ImportY),
        ]
        .spacing(12)
        .align_y(Center),
    ]
    .spacing(12);

    if wizard.swapped() {
        content = content.push(Banner::warning(
            "Sütunlar yer değiştirmiş görünüyor: boylam doğuya (X), enlem kuzeye (Y) doğru artar.",
        ));
    }

    // Önizleme: seçilen sütunlar vurgulanır.
    let cell = move |text: String, column: usize, header: bool| -> Element<'a, Message> {
        let chosen = Some(column) == wizard.x || Some(column) == wizard.y;
        let text = if header {
            label::mono_caption(text)
        } else {
            label::mono_caption(text).style(if chosen {
                style::text::accent
            } else {
                style::text::default
            })
        };

        container(text.wrapping(Wrapping::None))
            .width(Fill)
            .clip(true)
            .into()
    };

    let mut table =
        Column::new()
            .spacing(4)
            .push(Row::with_children(CSV_COLUMNS.iter().enumerate().map(
                |(column, name)| {
                    let marker = if Some(column) == wizard.x {
                        " (X)"
                    } else if Some(column) == wizard.y {
                        " (Y)"
                    } else {
                        ""
                    };

                    cell(format!("{name}{marker}"), column, true)
                },
            )));

    for row in 0..4 {
        table = table
            .push(Row::with_children((0..CSV_COLUMNS.len()).map(|column| {
                cell(ImportWizard::sample(row, column), column, false)
            })));
    }

    content
        .push(label::caption("Önizleme: ilk dört kayıt"))
        .push(
            container(table)
                .padding([8, 10])
                .width(Fill)
                .style(style::container::field_box),
        )
        .into()
}

/// 3. adım: koordinat sistemi.
fn system_step<'a>(wizard: &'a ImportWizard) -> Element<'a, Message> {
    let banner = match wizard.source {
        Some(source) if source.has_columns() => Banner::warning(
            "Dosyada koordinat sistemi bilgisi yok. Değerler derece aralığında; WGS 84 önerildi.",
        ),
        _ => Banner::info("GeoJSON koordinatları her zaman WGS 84'tür (RFC 7946)."),
    };

    let systems = Column::with_children(SYSTEMS.iter().enumerate().map(
        |(index, (code, name, description, _))| {
            wizard::choice(
                column![
                    row![label::mono(*code), label::strong(*name)]
                        .spacing(10)
                        .align_y(Center),
                    label::caption(*description),
                ]
                .spacing(2),
                wizard.system == index,
                Message::ImportSystem(index),
            )
        },
    ))
    .spacing(4);

    column![banner, systems].spacing(10).into()
}

/// 4. adım: katman adı ve özet.
fn summary_step<'a>(wizard: &'a ImportWizard, problem: Option<String>) -> Element<'a, Message> {
    let Some(source) = wizard.source else {
        return space::vertical().into();
    };

    let invalid = problem.is_some();
    let mut name = column![
        text_input("Katman adı", &wizard.name)
            .on_input(Message::ImportName)
            .on_submit(Message::ImportNext)
            .font(typography::ui())
            .size(typography::body())
            .padding([4, 8])
            .style(style::field::validated(invalid)),
    ]
    .spacing(4)
    .width(Fill);

    if let Some(problem) = problem {
        name = name.push(label::caption(problem).style(style::text::danger));
    }

    let (code, system, _, _) = SYSTEMS[wizard.system];
    let columns = match (wizard.x, wizard.y) {
        (Some(x), Some(y)) if source.has_columns() => {
            format!("X: {}, Y: {}", CSV_COLUMNS[x], CSV_COLUMNS[y])
        }
        _ => "Dosyanın geometrisi".to_owned(),
    };

    let rows = [
        ("Dosya", source.file().to_owned()),
        ("Biçim", format!("{}, {}", source.format(), source.size())),
        (
            "Öğe",
            format!("{} {}", source.count(), source.geometry().to_lowercase()),
        ),
        ("Koordinatlar", columns),
        ("Koordinat sistemi", format!("{code} {system}")),
    ];

    let summary = Column::with_children(rows.map(|(key, value)| {
        row![
            label::muted(key).width(typography::scaled(NAME_WIDTH)),
            label::body(value),
        ]
        .spacing(12)
        .into()
    }))
    .spacing(6);

    column![
        row![
            label::muted("Katman adı").width(typography::scaled(NAME_WIDTH)),
            name,
        ]
        .spacing(12),
        summary,
    ]
    .spacing(16)
    .into()
}
