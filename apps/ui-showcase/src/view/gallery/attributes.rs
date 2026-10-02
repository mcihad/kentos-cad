//! Öznitelikler sayfası: nesne inceleyici, öznitelik tablosu, sorgu
//! oluşturucu, parçalı seçim ve alan türleri.
//!
//! Örnekler aynı yapı envanterini paylaşır: tabloda tıklanan kayıt
//! inceleyicide açılır, sorguya uyan kayıtlar tabloda vurgulanır.

use iced::widget::text::Wrapping;
use iced::widget::{column, container, row};
use iced::{Center, Element, Fill};

use kentos_ui::attribute::{DateTime, Field, FieldKind, ObjectId, Value, text};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::spatial::SelectionMode;
use kentos_ui::style;
use kentos_ui::widget::color::ColorPicker;
use kentos_ui::widget::inspector::Subject;
use kentos_ui::widget::number::{self, units};
use kentos_ui::widget::table::{self, Table};
use kentos_ui::widget::{
    Choice, DatePicker, Inspector, NumberInput, QueryBuilder, Segmented, Select, TimePicker,
    Toolbar,
};

use super::{entry, pressed};
use crate::app::{Showcase, TIME_ZONE};
use crate::gallery::Demo;
use crate::message::Message;

/// İnceleyicinin kategorileri: başlık ve kapsadığı alanlar.
const CATEGORIES: [(&str, std::ops::Range<usize>); 4] = [
    ("Genel", 0..6),
    ("Tarihler", 6..9),
    ("Konum", 9..11),
    ("Açıklama", 11..12),
];

impl Showcase {
    pub(super) fn attributes_page(&self) -> Vec<Element<'_, Message>> {
        vec![
            entry(
                "Nesne inceleyici",
                "kentos_ui::widget::Inspector",
                "ArcGIS'teki öznitelik bölmesi ve CAD'deki Özellikler paleti gibi: her alan \
                 türü kendi düzenleyicisiyle. Üstte arama, kategorili ya da alfabetik görünüm \
                 ve boş alanları gizleme; kategoriler başlığa tıklanarak daralır. Değişen \
                 alanlar solda çizgiyle işaretlenir ve ↺ ile ilk değerine döner. Takvim, saat \
                 ve listeler açılır panelde açılır. Alttaki yardım, imlecin üzerindeki alanın \
                 türünü, kısıtlarını ve açıklamasını gösterir.",
                row![
                    container(self.demo_inspector()).width(440),
                    container(editor_table()).width(Fill),
                ]
                .spacing(16),
                Some(
                    "Inspector::new(&state, Message::Inspector)\n    \
                     .category(\"Genel\")\n    \
                     .field(0, &schema[0], &values[0])\n    \
                     .object(9, &schema[9], &values[9], candidates, true)\n\n\
                     // update\n\
                     if let Some(Action::Change { id, value }) = state.update(event) { … }",
                ),
            ),
            entry(
                "Çoklu seçim ve özel satırlar",
                "kentos_ui::widget::inspector::{Subject, Inspector::varies, Inspector::row}",
                "3B şehir modelinde üç yapı seçili. Değerleri farklı alanlar “Çeşitli” yazar; \
                 evet/hayır alanında hiçbir parça seçili değildir. Yazılan ya da seçilen değer \
                 üç yapıya da uygulanır ve alan ortak olur. Konum, dönüklük ve cephe rengi \
                 uygulamanın kendi düzenleyicileriyle eklenen satırlardır. Sayı alanlarının \
                 adını yana sürükleyin: değer adım adım değişir, altında verniyer kayar. Ad \
                 sütununun kenarı sürüklenir; çift tık varsayılana döndürür. Satıra sağ \
                 tıklayın: İlk değerine döndür, Boş bırak, Değeri kopyala.",
                row![
                    container(self.demo_selection()).width(440),
                    container(scenario_notes()).width(Fill),
                ]
                .spacing(16),
                Some(
                    "Inspector::new(&state, Message::Building)\n    \
                     .subject(Subject::new(\"3 yapı\").icon(Icon::Cube).detail(\"Çoklu seçim\"))\n    \
                     .field(2, &schema[2], &values[2])\n    \
                     .varies(2)                       // değerler farklı: “Çeşitli”\n    \
                     .row(\"Konum\", Some(Icon::Crosshair), number::vector(..))",
                ),
            ),
            entry(
                "Tarih ve saat seçicileri",
                "kentos_ui::widget::DatePicker, TimePicker",
                "Takvim başlığa tıklanınca ay, sonra yıl görünümüne geçer; hafta numaraları \
                 ISO 8601'e göredir, bugün kenarla, hafta sonu sönük gösterilir. Tarih ve \
                 saatte takvimle saat ızgarası yan yana açılır. Panelde gezinmek uygulamaya \
                 mesaj üretmez; yalnızca seçilen değer bildirilir. Bir uygulamanın metin \
                 girişine de eklenebilir.",
                self.demo_pickers(),
                Some(
                    "DatePicker::date(value, Message::DatePicked)\n    \
                     .anchor(text_input(\"GG.AA.YYYY\", &draft).on_input(Message::DateTyped))\n    \
                     .now(DateTime::now(180))\n\n\
                     DatePicker::date_time(moment, Message::MomentPicked)\n\
                     TimePicker::new(time, Message::TimePicked)",
                ),
            ),
            entry(
                "Seçim kutusu",
                "kentos_ui::widget::Select",
                "Aranabilir açılır liste: seçeneklerde renk örneği, ikon ve sağda ayrıntı. \
                 Uzun listelerde arama kutusu kendiliğinden çıkar ve açılışta odaklanır. \
                 Listenin altına komutlar eklenebilir.",
                self.demo_select(),
                Some(
                    "Select::new(\n    REGIONS.map(|(name, color)| Choice::new(name).color(color)),\n    \
                     selected,\n    Message::RegionPicked,\n)\n.clear(Message::RegionCleared)",
                ),
            ),
            entry(
                "Öznitelik tablosu",
                "kentos_ui::widget::Table, Toolbar",
                "Araç çubuğu ve yatay kayan, sıralanabilir tablo. Başlığa tıklayarak \
                 sıralayın; satıra tıklayınca kayıt yukarıdaki inceleyicide açılır ve \
                 kenarıyla işaretlenir. Aşağıdaki sorguya uyan satırlar seçili görünür.",
                self.demo_table(),
                Some(
                    "Table::new([\n    \
                     table::Column::new(\"Kat\").width(56).align_right()\n        \
                     .sortable(order, Message::Sort(2)),\n])\n\
                     .extend(rows)\n.horizontal()\n.height(196)",
                ),
            ),
            entry(
                "Sorgu oluşturucu",
                "kentos_ui::widget::QueryBuilder",
                "Öznitelikle seç ve tablo filtresi için koşullar. İşleçler alan türüne göre \
                 değişir; seçenekli alanlarda değer açılır listeden seçilir. Metinler Türkçe \
                 harf ve büyük/küçük harf duyarsız karşılaştırılır.",
                self.demo_query(),
                Some(
                    "QueryBuilder::new(&schema, &query, Message::QueryEdited)\n\n\
                     // update\n\
                     query.apply(edit, &schema);\n\
                     let found = records.filter(|values| query.matches(&schema, values));",
                ),
            ),
            entry(
                "Parçalı seçim",
                "kentos_ui::widget::Segmented",
                "Birbirini dışlayan birkaç seçenek; seçim yöntemi ve koşulların birleşimi \
                 gibi. Seçenekler Display ile yazılır. Seçili parça gömük yuvanın içinde \
                 kalkık durur, yazısı yarı kalındır ama genişliği değişmez; seçili olmayan \
                 komşuları ince bir çizgi ayırır. İkon, yalnız ikon, sıkışık ve vurgulu \
                 türleri vardır; hiçbir parçası seçili olmayan seçim çoklu seçimde \
                 farklı değerleri gösterir.",
                self.demo_segmented(),
                Some(
                    "Segmented::new(SelectionMode::ALL, mode, Message::ModeSelected)\n    \
                     .icons([Icon::Select, Icon::Plus, Icon::Minus, Icon::Svg(INTERSECT)])\n    \
                     .width(Fill)\n\n\
                     Segmented::new(View::ALL, view, Message::ViewChosen).accent()\n\
                     Segmented::optional(Hazard::ALL, None, Message::Hazard).compact()",
                ),
            ),
            entry(
                "Alan türleri",
                "kentos_ui::attribute",
                "Alanın türü değerin nasıl yazılacağını, çözümleneceğini ve denetleneceğini \
                 belirler. Sayılar ve tarihler Türkçe yazılır, metinler Türk alfabesine göre \
                 sıralanır. Sonuçlar canlı hesaplanır.",
                field_examples(),
                None,
            ),
        ]
    }

    fn demo_inspector(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;
        let record = gallery.record;

        let name = gallery
            .value(record, 0)
            .as_str()
            .unwrap_or("Adsız yapı")
            .to_owned();

        let mut inspector = Inspector::new(&gallery.inspector, |event| {
            Message::Gallery(Demo::Inspector(event))
        })
        .now(DateTime::now(TIME_ZONE))
        .subject(
            Subject::new(name)
                .icon(Icon::Polygon)
                .layer(iced::Color::from_rgb8(0x5a, 0x9c, 0xf0), "Yapılar")
                .detail(format!("#{}", record + 1))
                .action(Icon::Target, "Haritada göster", pressed("Haritada göster")),
        );

        for (title, fields) in CATEGORIES {
            inspector = inspector.category(title);

            for index in fields {
                let Some(field) = gallery.schema.get(index) else {
                    continue;
                };

                let value = gallery.value(record, index);

                inspector = match &field.kind {
                    FieldKind::Object { target } => {
                        inspector.object(index, field, value, self.candidates(target), true)
                    }
                    _ => inspector.field(index, field, value),
                };
            }
        }

        inspector
            .category("Kayıt")
            .figure(
                "Satır",
                format!("{} / {}", record + 1, gallery.records.len()),
            )
            .into()
    }

    /// Çoklu seçim: üç yapının ortak alanları, “Çeşitli” değerler ve
    /// uygulamanın kendi düzenleyicileriyle satırlar.
    fn demo_selection(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;
        let theme = self.theme();
        let schema = &gallery.building_schema;
        let values = &gallery.building_values;

        let mut inspector = Inspector::new(&gallery.building, |event| {
            Message::Gallery(Demo::Building(event))
        })
        .now(DateTime::now(TIME_ZONE))
        .subject(
            Subject::new("3 yapı")
                .icon(Icon::Cube)
                .layer(
                    iced::Color::from_rgb8(0xd9, 0x8c, 0x4a),
                    "3B şehir › Yapılar",
                )
                .detail("Çoklu seçim")
                .action(
                    Icon::ZoomExtents,
                    "Seçime yakınlaştır",
                    pressed("Seçime yakınlaştır"),
                ),
        );

        for (title, fields) in [("Yapı", 0..5), ("Afet senaryosu", 5..9)] {
            inspector = inspector.category(title);

            for index in fields {
                if let (Some(field), Some(value)) = (schema.get(index), values.get(index)) {
                    inspector = inspector.field(index, field, value);

                    if gallery.building_varied.contains(&index) {
                        inspector = inspector.varies(index);
                    }
                }
            }
        }

        let position = gallery.position;
        let on_position = move |axis: usize| {
            move |value: f64| {
                let mut position = position;
                position[axis] = value;
                Message::Gallery(Demo::Position(position))
            }
        };

        inspector
            .category("Yerleşim")
            .row(
                "Konum",
                Some(Icon::Crosshair),
                row![
                    NumberInput::new(position[0], on_position(0))
                        .label("Y")
                        .tone(number::axis_color(0, &theme))
                        .units(units::LENGTH)
                        .step(0.01)
                        .inline(),
                    NumberInput::new(position[1], on_position(1))
                        .label("X")
                        .tone(number::axis_color(1, &theme))
                        .units(units::LENGTH)
                        .step(0.01)
                        .inline(),
                ]
                .spacing(4),
            )
            .row(
                "Dönüklük",
                Some(Icon::Svg(TURN)),
                NumberInput::new(gallery.rotation, |degrees| {
                    Message::Gallery(Demo::Rotation(degrees))
                })
                .units(units::ANGLE)
                .step(0.5)
                .inline(),
            )
            .row(
                "Cephe rengi",
                Some(Icon::Drop),
                ColorPicker::new(gallery.fill, |color| Message::Gallery(Demo::Fill(color)))
                    .width(Fill)
                    .inline(),
            )
            .category("Ölçüler")
            .figure("Taban alanı", "1.248,60 m²")
            .figure("Hacim", "47.446,80 m³")
            .into()
    }

    /// Uygulamanın metin girişi olmadan, kendi başına kullanılan seçiciler.
    fn demo_pickers(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;
        let now = DateTime::now(TIME_ZONE);

        let picker = |name: &'static str, picker: Element<'static, Message>| {
            column![label::caption(name), container(picker).width(220)].spacing(4)
        };

        row![
            picker(
                "Tarih",
                DatePicker::date(gallery.picked_date, |date| {
                    Message::Gallery(Demo::DatePicked(date))
                })
                .now(now)
                .into()
            ),
            picker(
                "Tarih ve saat",
                DatePicker::date_time(gallery.picked_moment, |moment| {
                    Message::Gallery(Demo::MomentPicked(moment))
                })
                .now(now)
                .into()
            ),
            picker(
                "Saat",
                TimePicker::new(gallery.picked_time, |time| {
                    Message::Gallery(Demo::TimePicked(time))
                })
                .now(now)
                .into()
            ),
        ]
        .spacing(16)
        .into()
    }

    /// Bölgeler: renk örnekli ve şehir sayılı seçenekler.
    fn demo_select(&self) -> Element<'_, Message> {
        let regions: Vec<Choice> = self
            .layers
            .get(1)
            .map(|cities| {
                cities
                    .sublayers
                    .iter()
                    .enumerate()
                    .map(|(index, sublayer)| {
                        Choice::new(sublayer.name.clone())
                            .color(sublayer.color)
                            .detail(format!("{} şehir", cities.sublayer_features(index).count()))
                    })
                    .collect()
            })
            .unwrap_or_default();

        container(
            Select::new(regions, self.gallery.picked_region, |index| {
                Message::Gallery(Demo::RegionPicked(Some(index)))
            })
            .searchable(true)
            .clear(Message::Gallery(Demo::RegionPicked(None)))
            .action(
                Icon::Target,
                "Bölgeye yakınlaştır",
                pressed("Bölgeye yakınlaştır"),
            ),
        )
        .width(260)
        .into()
    }

    fn demo_table(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;
        let rows = gallery.rows();
        let matched = rows
            .iter()
            .filter(|&&record| gallery.matches(record))
            .count();

        let columns = gallery.schema.iter().enumerate().map(|(index, field)| {
            let title = match &field.unit {
                Some(unit) => format!("{} ({unit})", field.name),
                None => field.name.clone(),
            };
            let order = gallery
                .sort
                .and_then(|(column, order)| (column == index).then_some(order));

            let column = table::Column::new(title)
                .width(demo_width(field))
                .sortable(order, Message::Gallery(Demo::Sorted(index)));

            if field.is_numeric() {
                column.align_right()
            } else {
                column
            }
        });

        let body = rows.iter().map(|&record| {
            let cells = gallery
                .schema
                .iter()
                .enumerate()
                .map(|(index, field)| self.demo_cell(field, gallery.value(record, index)));

            table::Row::new(cells)
                .selected(gallery.matches(record))
                .current(record == gallery.record)
                .on_press(Message::Gallery(Demo::RecordSelected(record)))
        });

        let toolbar = Toolbar::new()
            .search(&gallery.search, "Tabloda ara", |search| {
                Message::Gallery(Demo::SearchChanged(search))
            })
            .spacer()
            .push(label::caption(format!(
                "{} kayıt, {matched} tanesi sorguya uyuyor",
                rows.len()
            )));

        container(column![
            toolbar,
            Table::new(columns)
                .extend(body)
                .horizontal()
                .height(196)
                .empty("Aramaya uyan kayıt yok."),
        ])
        .padding(1)
        .width(Fill)
        .style(style::container::bordered)
        .into()
    }

    fn demo_cell<'a>(&'a self, field: &'a Field, value: &'a Value) -> Element<'a, Message> {
        let content = match (&field.kind, value) {
            (FieldKind::Object { target }, Value::Object(id)) => city_name(self, target, *id),
            _ => field.format(value),
        };

        // Hücreler tek satırdır: çok satırlı metnin (ör. Not) ilk satırı
        // gösterilir, sütuna sığmayan metin kırpılır.
        let content = text::first_line(&content).into_owned();

        if content.is_empty() {
            return label::caption("—").into();
        }

        let cell = match field.kind {
            FieldKind::Object { .. } => {
                return row![
                    icon(Icon::Link).size(12.0).tone(Tone::Muted),
                    label::body(content).wrapping(Wrapping::None)
                ]
                .spacing(4)
                .align_y(Center)
                .into();
            }
            FieldKind::Date | FieldKind::Time | FieldKind::DateTime => label::mono(content),
            _ if field.is_numeric() => label::mono(content),
            _ => label::body(content),
        };

        cell.wrapping(Wrapping::None).into()
    }

    fn demo_query(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;
        let problem = crate::view::query::problem(&gallery.query, &gallery.schema);
        let matched = (0..gallery.records.len())
            .filter(|&record| gallery.matches(record))
            .count();

        let expression = if gallery.query.is_empty() {
            "Koşul yok".to_owned()
        } else {
            gallery.query.describe(&gallery.schema)
        };

        let outcome = match problem {
            Some(problem) => problem.to_owned(),
            None => format!("{matched} / {} kayıt eşleşiyor", gallery.records.len()),
        };

        column![
            container(QueryBuilder::new(&gallery.schema, &gallery.query, |edit| {
                Message::Gallery(Demo::QueryEdited(edit))
            }))
            .width(640),
            container(
                column![
                    label::mono_caption(expression).style(style::text::default),
                    label::caption(outcome),
                ]
                .spacing(4),
            )
            .padding([8, 10])
            .width(640)
            .style(style::container::field),
        ]
        .spacing(12)
        .into()
    }

    fn demo_segmented(&self) -> Element<'_, Message> {
        let gallery = &self.gallery;
        let mode = gallery.mode;
        let chosen = |slot: usize| gallery.segments[slot];
        let pick = |slot: usize| move |name: Named| Message::Gallery(Demo::Segment(slot, name.1));

        let explanation = match mode {
            SelectionMode::New => "Seçimi bulunan öğelerle değiştirir.",
            SelectionMode::Add => "Bulunan öğeleri mevcut seçime ekler.",
            SelectionMode::Remove => "Bulunan öğeleri mevcut seçimden çıkarır.",
            SelectionMode::Intersect => "Seçimde yalnızca bulunan öğeleri bırakır.",
        };

        let views = named(&["Plan", "3B", "Kesit"]);
        let windows = named(&["Pencere", "Kesişen", "Çokgen", "Çit"]);
        let aligns = named(&["Sola", "Ortaya", "Sağa"]);
        let hazards = named(&["Deprem", "Sel", "Yangın"]);

        let example =
            |title: &'static str, note: &'static str, control: Element<'static, Message>| {
                row![
                    column![label::body(title), label::caption(note)]
                        .spacing(2)
                        .width(220),
                    container(control).width(Fill),
                ]
                .spacing(16)
                .align_y(Center)
            };

        column![
            example(
                "İkonlu, tam genişlik",
                "Parçalar genişliği eşit paylaşır.",
                column![
                    Segmented::new(SelectionMode::ALL, mode, |mode| {
                        Message::Gallery(Demo::ModeSelected(mode))
                    })
                    .icons([Icon::Select, Icon::Plus, Icon::Minus, Icon::Svg(INTERSECT)])
                    .width(Fill),
                    label::caption(explanation),
                ]
                .spacing(6)
                .into(),
            ),
            example(
                "Vurgulu",
                "Görünümü ya da kipi değiştiren seçim.",
                Segmented::new(views.clone(), views[chosen(0).unwrap_or(0)], pick(0))
                    .icons([Icon::Svg(PLAN), Icon::Cube, Icon::Svg(SECTION)])
                    .accent()
                    .into(),
            ),
            example(
                "Yalnız ikon",
                "Adlar ipucunda; araç çubukları ve dar paneller.",
                Segmented::new(windows.clone(), windows[chosen(1).unwrap_or(0)], pick(1))
                    .icons([
                        Icon::Svg(WINDOW),
                        Icon::Svg(CROSSING),
                        Icon::Svg(LASSO),
                        Icon::Svg(FENCE),
                    ])
                    .icon_only()
                    .into(),
            ),
            example(
                "Sıkışık",
                "Özellik satırına ve tablo hücresine sığar.",
                row![
                    Segmented::new(aligns.clone(), aligns[chosen(2).unwrap_or(0)], pick(2))
                        .icons([
                            Icon::Svg(ALIGN_LEFT),
                            Icon::Svg(ALIGN_CENTER),
                            Icon::Svg(ALIGN_RIGHT),
                        ])
                        .icon_only()
                        .compact(),
                    Segmented::new(
                        ["—", "Evet", "Hayır"].map(Named::text),
                        Named::text("Evet"),
                        |_| { Message::Gallery(Demo::Pressed("Evet")) }
                    )
                    .compact(),
                ]
                .spacing(12)
                .align_y(Center)
                .into(),
            ),
            example(
                "Seçimsiz ve devre dışı",
                "Çoklu seçimde değerler farklıysa hiçbiri seçili değildir.",
                row![
                    Segmented::optional(hazards.clone(), chosen(3).map(|i| hazards[i]), pick(3))
                        .icons([Icon::Svg(QUAKE), Icon::Svg(FLOOD), Icon::Svg(FIRE)]),
                    Segmented::new_with(
                        SelectionMode::ALL,
                        SelectionMode::New,
                        |mode| Message::Gallery(Demo::ModeSelected(mode)),
                        |mode| matches!(mode, SelectionMode::New | SelectionMode::Add),
                    )
                    .compact(),
                ]
                .spacing(12)
                .align_y(Center)
                .into(),
            ),
        ]
        .spacing(14)
        .into()
    }
}

/// Çoklu seçim örneğinin yanındaki notlar: inceleyicinin yeni davranışları.
fn scenario_notes<'a>() -> Element<'a, Message> {
    let line = |title: &'static str, text: &'static str| {
        column![label::body(title), label::caption(text)].spacing(2)
    };

    column![
        line(
            "Çeşitli",
            "Değerleri farklı alan; yazılan değer bütün seçime uygulanır.",
        ),
        line(
            "Adı sürükle",
            "Sayı ve aralık alanlarında; Shift ince, Ctrl kaba ayar.",
        ),
        line(
            "Sütun kenarı",
            "Ad sütununu genişletir; çift tık varsayılana döndürür.",
        ),
        line("Sağ tık", "İlk değerine döndür, Boş bırak, Değeri kopyala.",),
        line(
            "Yalnız değişenler",
            "Araç çubuğunda; incelemenin başından beri değişen alanlar.",
        ),
        line(
            "Özel satır",
            "Konum, dönüklük ve renk gibi uygulamanın düzenleyicileri.",
        ),
    ]
    .spacing(10)
    .into()
}

/// Dönüklüğün ikonu: döndürme oku.
const TURN: &str = r#"<path d="M15.5 10a5.5 5.5 0 1 1-1.6-3.9"/><path d="M14.5 3.2v3.2h-3.2"/>"#;

/// Parçalı seçim örneklerinin seçeneği: adı ve sırası.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Named(&'static str, usize);

impl Named {
    fn text(name: &'static str) -> Self {
        Self(name, 0)
    }
}

impl std::fmt::Display for Named {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

fn named(names: &[&'static str]) -> Vec<Named> {
    names
        .iter()
        .enumerate()
        .map(|(index, name)| Named(name, index))
        .collect()
}

// Örneklerin ikonları, web setinin 20×20 ızgarasında.
const INTERSECT: &str = r#"<rect x="3" y="3.5" width="9" height="9" rx="1"/><rect x="8" y="7.5" width="9" height="9" rx="1"/><rect x="8" y="7.5" width="4" height="5" fill="currentColor" fill-opacity="0.35" stroke="none"/>"#;
const PLAN: &str = r#"<path d="M3.5 4.5h13v11h-13z"/><path d="M3.5 10.5h6v5M12 4.5v6h4.5"/>"#;
const SECTION: &str = r#"<path d="M3 16.5h14"/><path d="M5 16.5V8.5l5-4 5 4v8"/><path d="M2.5 11.5h15" stroke-dasharray="2 2"/>"#;
const WINDOW: &str = r#"<rect x="3.5" y="5" width="13" height="10" rx="1"/>"#;
const CROSSING: &str =
    r#"<rect x="3.5" y="5" width="13" height="10" rx="1" stroke-dasharray="2.5 2"/>"#;
const LASSO: &str = r#"<path d="M4 14.5l2-9.5 9 2.5 1.5 7z"/>"#;
const FENCE: &str = r#"<path d="M3 14l4-7 4 5 6-7"/>"#;
const ALIGN_LEFT: &str = r#"<path d="M4 5h12M4 9h8M4 13h12M4 17h7"/>"#;
const ALIGN_CENTER: &str = r#"<path d="M4 5h12M6 9h8M4 13h12M6.5 17h7"/>"#;
const ALIGN_RIGHT: &str = r#"<path d="M4 5h12M8 9h8M4 13h12M9 17h7"/>"#;
const QUAKE: &str = r#"<path d="M2 10.5h3l2-5.5 3 11 2-8 2 4.5h4"/>"#;
const FLOOD: &str = r#"<path d="M2.5 8c2-1.5 3-1.5 5 0s3 1.5 5 0 3-1.5 5 0M2.5 13c2-1.5 3-1.5 5 0s3 1.5 5 0 3-1.5 5 0"/>"#;
const FIRE: &str = r#"<path d="M10 17.5c-3 0-5-2-5-5 0-3 3-4 3-7.5 2 1 3.5 3 3.5 5.5 1-1 1.5-2 1.5-3 2 1.5 2 3.5 2 5 0 3-2 5-5 5z"/>"#;

/// Başvurulan şehrin adı; bulunamazsa "#numara".
fn city_name(showcase: &Showcase, target: &str, id: ObjectId) -> String {
    showcase
        .layers
        .iter()
        .find(|layer| layer.name == target)
        .and_then(|layer| layer.feature(id).map(|feature| layer.label(feature)))
        .unwrap_or_else(|| format!("#{id}"))
}

/// Örnek tablonun sütun genişlikleri.
fn demo_width(field: &Field) -> f32 {
    match field.kind {
        FieldKind::Text if field.editable => 150.0,
        FieldKind::Text => 84.0,
        FieldKind::Integer { .. } => 56.0,
        FieldKind::Real { .. } => 100.0,
        FieldKind::Bool => 72.0,
        FieldKind::Range { .. } => 90.0,
        FieldKind::Choice(_) => 84.0,
        FieldKind::Date => 92.0,
        FieldKind::Time => 64.0,
        FieldKind::DateTime => 134.0,
        FieldKind::Object { .. } => 104.0,
    }
}

/// Alan türlerinin düzenleyicileri.
fn editor_table<'a>() -> Element<'a, Message> {
    Table::new([
        table::Column::new("Alan türü").width(150),
        table::Column::new("Düzenleyici").width(Fill),
    ])
    .extend(
        [
            ("Metin", "Metin girişi; yazarken denetlenir"),
            (
                "Uzun metin",
                "İlk satır önizlemesi ve çok satırlı düzenleyici",
            ),
            (
                "Tam sayı, ondalık",
                "Metin girişi, birim ve artırma/azaltma okları",
            ),
            ("Evet/hayır", "Parçalı seçim; boş bırakılabilirse üç parça"),
            (
                "Kodlu değer",
                "Aranabilir açılır liste; \"Boş bırak\" komutu",
            ),
            ("Aralık", "Kaydırıcı ve sayı girişi"),
            ("Tarih", "Takvim: ay ve yıl görünümü, hafta numarası, bugün"),
            ("Tarih ve saat", "Takvim ve saat ızgarası yan yana"),
            ("Saat", "Saat ve dakika ızgarası, \"Şimdi\""),
            (
                "Nesne başvurusu",
                "Aranabilir liste, haritadan seçme, başvuruya git",
            ),
            ("Salt okunur", "Kilitli ve sönük"),
        ]
        .map(|(kind, editor)| {
            table::Row::new([label::body(kind).into(), label::muted(editor).into()])
        }),
    )
    .into()
}

/// Alan ve metin fonksiyonlarının canlı sonuçları.
fn field_examples<'a>() -> Element<'a, Message> {
    let population = Field::integer("Nüfus").unit("kişi");
    let height = Field::real("Yükseklik", 1).unit("m");
    let plate = Field::integer("Plaka").between(1, 81);
    let usage = Field::choice("Kullanım", ["Konut", "Ticaret"]);
    let permit = Field::date("Ruhsat");

    let shown = |result: Result<Value, String>, field: &Field| match result {
        Ok(value) => format!("Ok: {}", field.format(&value)),
        Err(error) => format!("Hata: {error}"),
    };

    let examples = [
        (
            "population.format_with_unit(&Value::Integer(15_840_900))",
            population.format_with_unit(&Value::Integer(15_840_900)),
        ),
        (
            "height.format_with_unit(&Value::Real(1234.5))",
            height.format_with_unit(&Value::Real(1234.5)),
        ),
        (
            "Field::boolean(\"Asansör\").format(&Value::Bool(true))",
            Field::boolean("Asansör").format(&Value::Bool(true)),
        ),
        (
            "permit.parse(\"12.3.2019\")",
            shown(permit.parse("12.3.2019"), &permit),
        ),
        (
            "usage.parse(\"konut\")",
            shown(usage.parse("konut"), &usage),
        ),
        ("plate.parse(\"82\")", shown(plate.parse("82"), &plate)),
        (
            "text::compare(\"Çınar\", \"Cadde\")",
            format!("{:?}", text::compare("Çınar", "Cadde")),
        ),
        (
            "text::contains(\"İSTANBUL\", \"istanbul\")",
            text::contains("İSTANBUL", "istanbul").to_string(),
        ),
    ];

    Table::new([
        table::Column::new("Çağrı").width(Fill),
        table::Column::new("Sonuç").width(300),
    ])
    .extend(examples.map(|(call, result)| {
        table::Row::new([label::mono(call).into(), label::mono(result).into()])
    }))
    .into()
}
