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
use kentos_ui::widget::table::{self, Table};
use kentos_ui::widget::{
    Choice, DatePicker, Inspector, QueryBuilder, Segmented, Select, TimePicker, Toolbar,
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
                    container(self.demo_inspector()).width(400),
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
                 gibi. Seçenekler Display ile yazılır.",
                self.demo_segmented(),
                Some("Segmented::new(SelectionMode::ALL, mode, Message::ModeSelected)"),
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

        let mut inspector = Inspector::new(&gallery.inspector, |event| {
            Message::Gallery(Demo::Inspector(event))
        })
        .now(DateTime::now(TIME_ZONE));

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
        let mode = self.gallery.mode;

        let explanation = match mode {
            SelectionMode::New => "Seçimi bulunan öğelerle değiştirir.",
            SelectionMode::Add => "Bulunan öğeleri mevcut seçime ekler.",
            SelectionMode::Remove => "Bulunan öğeleri mevcut seçimden çıkarır.",
            SelectionMode::Intersect => "Seçimde yalnızca bulunan öğeleri bırakır.",
        };

        column![
            Segmented::new(SelectionMode::ALL, mode, |mode| {
                Message::Gallery(Demo::ModeSelected(mode))
            }),
            label::muted(explanation),
        ]
        .spacing(8)
        .into()
    }
}

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
