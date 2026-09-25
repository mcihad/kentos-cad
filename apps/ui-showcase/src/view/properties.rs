//! Katman özellikleri penceresi: genel, kaynak, sembolizasyon, etiketler ve
//! alanlar.

use iced::widget::{
    Column, button, checkbox, column, container, pick_list, row, slider, space, text_input,
};
use iced::{Center, Element, Fill, Right};

use kentos_ui::label;
use kentos_ui::spatial::{Layer, LayerKind};
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::{PropertiesDialog, Segmented, overlay, swatch};

use super::panes::COLORS;
use crate::app::Showcase;
use crate::message::Message;
use crate::properties::{Edit, LayerProperties, Section};

/// Form satırlarındaki ad sütunu (12 piksellik metne göre).
const NAME_WIDTH: f32 = 132.0;

/// Etiketlerin görünmeye başladığı yakınlaştırma seçenekleri.
const LABEL_ZOOMS: [Option<f64>; 6] =
    [None, Some(5.0), Some(6.0), Some(7.0), Some(8.0), Some(10.0)];

impl Showcase {
    pub(super) fn layer_properties<'a>(
        &'a self,
        properties: &'a LayerProperties,
    ) -> Element<'a, Message> {
        let Some(layer) = self.layers.get(properties.layer) else {
            return space::vertical().into();
        };

        let section = properties.section;

        let body = match section {
            Section::General => self.general_section(properties, layer),
            Section::Source => self.source_section(properties.layer, layer),
            Section::Symbology => symbology_section(properties, layer),
            Section::Labels => labels_section(properties, layer),
            Section::Fields => fields_section(layer),
        };

        let dialog = Section::ALL.into_iter().fold(
            PropertiesDialog::new("Katman özellikleri").subtitle(format!(
                "{}, {} katmanı",
                layer.name,
                layer.kind.label().to_lowercase()
            )),
            |dialog, item| {
                dialog.section(
                    item.icon(),
                    item.label(),
                    item == section,
                    Message::PropertiesSection(item),
                )
            },
        );

        let valid = properties
            .draft
            .problem(&self.layers, properties.layer)
            .is_none();

        let mut dialog = dialog
            .body(section.label(), body)
            .description(section.description())
            .dirty(properties.is_dirty())
            .on_cancel(Message::PropertiesClosed);

        // Ad geçersizken taslak yazılamaz.
        if valid {
            dialog = dialog
                .on_apply(Message::PropertiesApplied)
                .on_accept(Message::PropertiesAccepted);
        }

        overlay::blocking(dialog)
    }

    fn general_section<'a>(
        &'a self,
        properties: &'a LayerProperties,
        layer: &'a Layer,
    ) -> Element<'a, Message> {
        let draft = &properties.draft;
        let problem = draft.problem(&self.layers, properties.layer);

        let mut name = column![
            text_input("Katman adı", &draft.name)
                .on_input(|name| Message::PropertiesEdited(Edit::Name(name)))
                .font(typography::ui())
                .size(typography::body())
                .padding([4, 8])
                .style(style::field::validated(problem.is_some())),
        ]
        .spacing(4);

        if let Some(problem) = problem {
            name = name.push(label::caption(problem).style(style::text::danger));
        }

        column![
            form_row("Ad", name),
            form_row("Tür", label::body(layer.kind.label())),
            form_row(
                "Görünür",
                checkbox(draft.visible)
                    .label("Haritada çiz")
                    .size(13.0)
                    .font(typography::ui())
                    .text_size(typography::body())
                    .on_toggle(|visible| Message::PropertiesEdited(Edit::Visible(visible))),
            ),
            form_row(
                "Opaklık",
                row![
                    slider(0.0..=1.0, draft.opacity, |opacity| {
                        Message::PropertiesEdited(Edit::Opacity(opacity))
                    })
                    .step(0.05_f32),
                    label::mono_caption(format!("{:>3.0}%", draft.opacity * 100.0))
                        .style(style::text::default)
                        .width(40)
                        .align_x(Right),
                ]
                .spacing(10)
                .align_y(Center),
            ),
        ]
        .spacing(12)
        .into()
    }

    fn source_section<'a>(&'a self, index: usize, layer: &'a Layer) -> Element<'a, Message> {
        let extent = layer.bounds().map_or_else(
            || "Öğe yok".to_owned(),
            |bounds| {
                format!(
                    "{:.2}° – {:.2}° K, {:.2}° – {:.2}° D",
                    bounds.south_west.lat,
                    bounds.north_east.lat,
                    bounds.south_west.lon,
                    bounds.north_east.lon
                )
            },
        );

        let rows = [
            (
                "Kaynak",
                self.sources.get(index).cloned().unwrap_or_default(),
            ),
            ("Geometri", layer.kind.label().to_owned()),
            ("Öğe sayısı", layer.features.len().to_string()),
            ("Alan sayısı", layer.schema.len().to_string()),
            ("Kapsam", extent),
            ("Koordinat sistemi", "EPSG:4326 WGS 84".to_owned()),
            ("Gösterim", "EPSG:3857 Web Mercator".to_owned()),
        ];

        Column::with_children(rows.map(|(key, value)| form_row(key, label::body(value))))
            .spacing(10)
            .into()
    }
}

fn symbology_section<'a>(
    properties: &'a LayerProperties,
    layer: &'a Layer,
) -> Element<'a, Message> {
    let draft = &properties.draft;

    let swatches = row(COLORS.iter().map(|&color| {
        button(
            container(space::horizontal())
                .width(18)
                .height(18)
                .style(style::container::swatch(color)),
        )
        .on_press(Message::PropertiesEdited(Edit::Color(color)))
        .padding(2)
        .style(style::button::swatch(
            draft.color.into_rgba8() == color.into_rgba8(),
        ))
        .into()
    }))
    .spacing(4);

    let mut content = column![form_row("Renk", swatches)].spacing(12);

    if layer.kind != LayerKind::Point {
        const STROKES: [u8; 4] = [1, 2, 3, 4];

        let stroke = STROKES
            .into_iter()
            .find(|&width| (f32::from(width) - draft.stroke).abs() < 0.26)
            .unwrap_or(0);

        content = content.push(form_row(
            "Çizgi kalınlığı",
            Segmented::new(STROKES, stroke, |width| {
                Message::PropertiesEdited(Edit::Stroke(f32::from(width)))
            })
            .width(220.0),
        ));
    }

    if !layer.sublayers.is_empty() {
        let field = layer
            .sublayer_field
            .and_then(|field| layer.schema.get(field))
            .map_or("", |field| field.name.as_str());

        let rows = layer.sublayers.iter().enumerate().map(|(index, sublayer)| {
            row![
                swatch(sublayer.color),
                label::body(sublayer.name.as_str()).width(Fill),
                label::mono_caption(layer.sublayer_features(index).count().to_string()),
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        });

        content = content.push(form_block(
            "Alt katmanlar",
            column![
                label::caption(format!(
                    "Öğeler \"{field}\" alanına göre renklenir; bu renk hiçbir alt katmana uymayanlar içindir."
                )),
                Column::with_children(rows).spacing(6),
            ]
            .spacing(8),
        ));
    }

    content.into()
}

fn labels_section<'a>(properties: &'a LayerProperties, layer: &'a Layer) -> Element<'a, Message> {
    let draft = &properties.draft;

    let fields: Vec<FieldChoice> = std::iter::once(FieldChoice(None, "Etiket yok".to_owned()))
        .chain(
            layer
                .schema
                .iter()
                .enumerate()
                .map(|(index, field)| FieldChoice(Some(index), field.name.clone())),
        )
        .collect();
    let current = fields
        .iter()
        .find(|choice| choice.0 == draft.label_field)
        .cloned();

    let zooms: Vec<ZoomChoice> = LABEL_ZOOMS.into_iter().map(ZoomChoice).collect();

    let example = draft
        .label_field
        .and_then(|field| {
            let feature = layer.features.first()?;
            Some(layer.schema.get(field)?.format(feature.value(field)))
        })
        .filter(|text| !text.is_empty());

    column![
        form_row(
            "Etiket alanı",
            pick_list(fields, current, |choice: FieldChoice| {
                Message::PropertiesEdited(Edit::LabelField(choice.0))
            })
            .font(typography::ui())
            .text_size(typography::body())
            .padding([3, 8])
            .width(typography::scaled(220.0))
            .style(style::field::pick_list)
            .menu_style(style::field::menu),
        ),
        form_row(
            "Göründüğü ölçek",
            pick_list(
                zooms,
                Some(ZoomChoice(draft.label_zoom)),
                |choice: ZoomChoice| { Message::PropertiesEdited(Edit::LabelZoom(choice.0)) }
            )
            .font(typography::ui())
            .text_size(typography::body())
            .padding([3, 8])
            .width(typography::scaled(220.0))
            .style(style::field::pick_list)
            .menu_style(style::field::menu),
        ),
        form_row(
            "Örnek",
            match example {
                Some(text) => label::strong(text),
                None => label::muted("Etiket gösterilmez."),
            },
        ),
    ]
    .spacing(12)
    .into()
}

fn fields_section<'a>(layer: &'a Layer) -> Element<'a, Message> {
    let header = row![
        label::caption("Ad").width(typography::scaled(NAME_WIDTH)),
        label::caption("Tür ve kısıtlar").width(Fill),
    ]
    .spacing(12);

    let rows = layer.schema.iter().map(|field| {
        let mut notes = vec![field.summary()];

        if field.required {
            notes.push("zorunlu".to_owned());
        }

        if !field.editable {
            notes.push("salt okunur".to_owned());
        }

        row![
            label::mono(field.name.as_str()).width(typography::scaled(NAME_WIDTH)),
            label::body(notes.join(", ")).width(Fill),
        ]
        .spacing(12)
        .into()
    });

    column![header, Column::with_children(rows).spacing(8)]
        .spacing(8)
        .into()
}

/// Form satırı: solda ad, sağda denetim.
fn form_row<'a>(name: &'a str, control: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    row![
        label::muted(name).width(typography::scaled(NAME_WIDTH)),
        container(control).width(Fill),
    ]
    .spacing(12)
    .align_y(Center)
    .into()
}

/// Çok satırlı denetimin form satırı: ad üste hizalı.
fn form_block<'a>(name: &'a str, control: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    row![
        label::muted(name).width(typography::scaled(NAME_WIDTH)),
        container(control).width(Fill),
    ]
    .spacing(12)
    .into()
}

/// Etiket alanı seçeneği: alanın sırası ve adı.
#[derive(Debug, Clone, PartialEq)]
struct FieldChoice(Option<usize>, String);

impl std::fmt::Display for FieldChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}

/// Etiketlerin görünmeye başladığı yakınlaştırma.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ZoomChoice(Option<f64>);

impl std::fmt::Display for ZoomChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            None => f.write_str("Her ölçekte"),
            Some(zoom) => write!(f, "{zoom:.0}. yakınlaştırmadan itibaren"),
        }
    }
}
