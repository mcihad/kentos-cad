//! Alanlar's look (the owner's word of 27 November 2026, docs/adr/9349): the layer's fields listed on the left, each
//! its name, its alias, its kind and what marks it (required, listed, new, renamed); the chosen field's settings on the
//! right in one form of sections: Genel (name, alias, kind, length or decimals, unit), Değerler (required, unique,
//! default, range, value list), Nesne işlemleri (split, merge, kept in the drawing), Kaynak (what its table keeps) and
//! Kullanıldığı yerler. The list's own buttons stand under it; the rarer ones (Verilerden al, Değerleri biçime çevir,
//! a data layer's Kaynak şemasını yenile and Kuralları kaynağa yaz) under Diğer ▾ over the list. What the window says
//! stands left of Vazgeç and Kaydet. The value list opens in its own window over it.
//!
//! The settings only show what the chosen field's kind takes: a number's range and unit, a text's length, a list for
//! the kinds that hold codes. Each box keeps its id (`cell_id`), so a click or Tab into it chooses the field as before
//! (GIS-003).

use iced::widget::{Column, button, column, container, row, space, text};
use iced::{Center, Element, Fill, Length};
use kentos_commands::{LayerField, LayerFieldKind};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::typography;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Form, ListDetails, Menu, MenuButton};
use kentos_ui::{label, style};

use super::{
    ADD, CANCEL, Cell, DOWN, Event, FIELDS_TITLE, FROM_DATA, FieldRow, MIGRATE, REFRESH, REMOVE, RULES, SAVE, UP,
    Window, cell_id, column_said, constraints_said, msg, problem_of, renames_of, unique_in_table,
};
use crate::app::{App, Message};
use crate::exchange::words::{self, Kind};
use crate::icons::from_web;

/// The list's and the settings' sizes, at the 12 px body text.
const LIST_WIDTH: f32 = 260.0;
const HEIGHT: f32 = 430.0;
const WIDTH: f32 = 900.0;
const LABELS: f32 = 130.0;

/// Whether a kind takes a length (a text) or decimals (a decimal number).
fn sized(kind: LayerFieldKind) -> Option<&'static str> {
    match kind {
        LayerFieldKind::Text => Some("Uzunluk"),
        LayerFieldKind::Decimal => Some("Ondalık basamak"),
        _ => None,
    }
}

/// Whether a kind holds codes of a value list.
fn listed(kind: LayerFieldKind) -> bool {
    matches!(
        kind,
        LayerFieldKind::Text | LayerFieldKind::Integer | LayerFieldKind::BigInt | LayerFieldKind::Decimal
    )
}

impl App {
    pub(crate) fn layer_fields_view(&self) -> Element<'_, Message> {
        let (Some(w), Some(doc)) = (&self.gis.layer_fields, &self.document) else {
            return text("").into();
        };
        let node = doc.model.layers().get(&w.layer);
        let data_layer = node.is_some_and(|n| n.data.is_some());
        // Over the list: the layer and where its fields are, and Diğer ▾.
        let place = match node.and_then(|n| n.data.as_ref()) {
            Some(d) if d.driver == "gpkg" => format!(
                "GeoPackage tablosu “{}”: alanlar tablonun sütunlarıdır.",
                d.table
            ),
            Some(d) => format!("Veritabanı tablosu “{}”: alanlar tablonun sütunlarıdır.", d.table),
            None => "Alanlar katmanın nesnelerinin öznitelikleridir.".to_owned(),
        };
        let more = {
            let data = data_layer;
            MenuButton::new(
                container(
                    row![label::body("Diğer"), icon(Icon::ChevronDown).size(10.0).tone(Tone::Muted)]
                        .spacing(4)
                        .align_y(Center),
                )
                .padding(kentos_ui::theme::metrics::action()),
                move || {
                    let menu = Menu::new()
                        .item(FROM_DATA, Some(msg(Event::FromData)))
                        .icon(from_web(Some("fieldsFromData")))
                        .detail("Nesnelerin taşıdığı ama listede olmayan her öznitelik için bir alan ekler.")
                        .item(MIGRATE, Some(msg(Event::Migrate)))
                        .icon(from_web(Some("conversion")))
                        .detail("Kayıtlı değerleri alanların biçimine çevirir (tarih, sayı, evet/hayır).");
                    if data {
                        menu.separator()
                            .item(REFRESH, Some(msg(Event::Refresh)))
                            .icon(from_web(Some("blockSync")))
                            .detail("Alanları tablonun sütunlarına göre yeniden okur.")
                            .item(RULES, Some(msg(Event::Rules)))
                            .icon(from_web(Some("domains")))
                            .detail("Alanların değer listelerini ve aralıklarını tablonun kuralı olarak yazar.")
                    } else {
                        menu
                    }
                },
            )
        };
        let head = row![
            column![
                label::strong(doc.model.layers().path(&w.layer)),
                label::caption(place).style(style::text::muted),
            ]
            .spacing(2)
            .width(Fill),
            more,
        ]
        .spacing(12)
        .align_y(Center);
        let body = ListDetails::new(self.fields_list(w), self.layer_field_settings(w))
            .list_foot(fields_list_foot(w))
            .list_width(LIST_WIDTH)
            .height(HEIGHT);
        let main = Frame::new(FIELDS_TITLE)
            .push(column![head, body].spacing(12))
            .status(fields_status(w))
            .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
            .action(words::primary(SAVE, (problem_of(&w.rows).is_none() && w.changed()).then(|| msg(Event::Save))))
            .width(WIDTH)
            .modal(msg(Event::Cancel));
        match &w.choices {
            Some(c) => iced::widget::stack![main, self.value_list_window(w, c)].into(),
            None => main,
        }
    }

    /// The fields, each a row: its name and alias, its kind and its marks; the chosen one lit.
    fn fields_list<'a>(&self, w: &'a Window) -> Element<'a, Message> {
        if w.rows.is_empty() {
            return container(label::muted("Alan yok: katmanın öznitelikleri serbest. Alan ekle ya da Diğer › Verilerden al ile ekleyin."))
                .padding(10)
                .into();
        }
        let mut list = Column::new().spacing(1).padding([4, 4]);
        for (i, r) in w.rows.iter().enumerate() {
            let name = if r.name.trim().is_empty() { "(adsız)".to_owned() } else { r.name.clone() };
            let mut names = column![label::body(name)].spacing(1);
            if !r.alias.trim().is_empty() {
                names = names.push(label::caption(r.alias.clone()).style(style::text::muted));
            }
            let mut marks = row![].spacing(6).align_y(Center);
            if r.from.is_none() {
                marks = marks.push(container(label::caption("yeni")).padding([0, 6]).style(style::container::badge));
            } else if r.from.as_deref() != Some(r.name.as_str()) {
                marks = marks.push(label::caption("ad değişiyor").style(style::text::warning));
            }
            if r.values.as_ref().is_some_and(|v| !v.is_empty()) || r.domain.is_some() {
                marks = marks.push(icon(from_web(Some("domains"))).size(13.0).tone(Tone::Muted));
            }
            if r.required {
                marks = marks.push(label::caption("zorunlu").style(style::text::accent));
            }
            let line = row![
                names.width(Fill),
                column![label::caption(r.kind.label()).style(style::text::muted), marks]
                    .spacing(2)
                    .align_x(iced::Alignment::End),
            ]
            .spacing(8)
            .align_y(Center);
            list = list.push(
                button(line)
                    .width(Fill)
                    .padding([6, 8])
                    .style(style::button::list_item(w.chosen == Some(i)))
                    .on_press(msg(Event::Choose(i))),
            );
        }
        list.into()
    }

    /// The chosen field's settings, sections of one form; what to do without one.
    fn layer_field_settings<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        let Some((i, r)) = w.chosen.and_then(|i| w.rows.get(i).map(|r| (i, r))) else {
            let words = if w.rows.is_empty() {
                "Katmana alan ekleyin: Alan ekle yeni bir alan, Diğer › Verilerden al nesnelerin taşıdığı öznitelikleri ekler."
            } else {
                "Ayarlarını görmek için soldan bir alan seçin."
            };
            return container(label::muted(words)).padding(16).into();
        };
        let Some(doc) = self.document.as_ref() else {
            return space().into();
        };
        let domains = &doc.model.settings().domains;
        let linked = r.domain.as_ref().and_then(|id| domains.iter().find(|d| d.id == *id));
        let input = |cell: Cell, value: &str, placeholder: &'static str| -> Element<'a, Message> {
            kentos_ui::widget::focus_watch::focus_watch(
                kentos_ui::widget::text_field(placeholder, value)
                    .id(cell_id(i, cell))
                    .on_input(move |t| msg(Event::Type(i, cell, t))),
            )
            .on_focus(move |on| msg(Event::Focused(i, on)))
            .into()
        };
        let check = |on: bool, words: &'static str, then: Option<Message>| words::check(on, words, then);
        let short = |e: Element<'a, Message>, width: f32| container(e).width(Length::Fixed(typography::scaled(width)));
        let mut form = Form::new().label_width(LABELS).section("Genel");
        form = form.field("Ad", input(Cell::Name, &r.name, "Ad")).required();
        if let Some(from) = r.from.as_deref().filter(|f| *f != r.name) {
            form = form.warning(Some(format!("Önceki adı “{from}”: değerleri kaydedince yeni adla taşınır.")));
        }
        form = form
            .field("Takma ad", input(Cell::Alias, &r.alias, "Tablo ve formda görünen ad"))
            .help("Tablonun başlığında, formda ve Öznitelikler'de görünür.")
            .field(
                "Tür",
                short(
                    Select::new(
                        LayerFieldKind::ALL.iter().map(|k| Choice::new(k.label())),
                        LayerFieldKind::ALL.iter().position(|k| *k == r.kind),
                        move |k| msg(Event::Kind(i, k)),
                    )
                    .into(),
                    220.0,
                ),
            );
        if let Some(size) = sized(r.kind) {
            form = form.field(size, short(input(Cell::Size, &r.size, ""), 120.0));
        }
        if r.kind.is_number() {
            form = form.field(
                "Birim",
                short(kentos_ui::widget::text_field("m², kg …", &r.unit).on_input(|t| msg(Event::Unit(t))).into(), 120.0),
            );
        }
        // Değerler: what a value must be.
        form = form.section("Değerler").row(
            row![
                kentos_ui::widget::tip(
                    row![
                        check_box(
                            if r.required { Check::Checked } else { Check::Unchecked },
                            Some(msg(Event::Required(i, !r.required))),
                        ),
                        label::body("Zorunlu"),
                    ]
                    .spacing(6)
                    .align_y(Center),
                    kentos_ui::widget::Tip::new("Zorunlu").body("Değeri boş bir kayıt yazılmaz."),
                    iced::widget::tooltip::Position::Top,
                ),
                check(r.unique, "Tekil", (r.kind != LayerFieldKind::Boolean).then(|| msg(Event::Unique(!r.unique)))),
            ]
            .spacing(20)
            .align_y(Center),
        );
        form = form.field("Varsayılan", short(input(Cell::Default, &r.default, "Yeni kayda yazılan değer"), 260.0));
        let ranged = (r.kind.is_number() || r.kind.is_temporal()) && r.domain.is_none();
        if ranged {
            let end = |cell: Cell, value: &str, out: bool, then: fn(bool) -> Event| {
                row![
                    short(input(cell, value, ""), 120.0),
                    check(out, "hariç", (!value.trim().is_empty()).then(|| msg(then(!out)))),
                ]
                .spacing(10)
                .align_y(Center)
            };
            form = form
                .field("En az", end(Cell::Min, &r.min, r.min_exclusive, Event::MinExclusive))
                .field("En çok", end(Cell::Max, &r.max, r.max_exclusive, Event::MaxExclusive));
        }
        if listed(r.kind) {
            let (words_now, summary) = match (linked, &r.values) {
                (Some(d), _) => (format!("Projenin listesi: {}", d.name), "Kodları Değer listeleri penceresindedir.".to_owned()),
                (None, Some(v)) if !v.is_empty() => {
                    let shown: Vec<String> = v.iter().take(4).map(|c| format!("{} ({})", c.label, c.code)).collect();
                    let more = if v.len() > 4 { format!(" ve {} değer daha", v.len() - 4) } else { String::new() };
                    (format!("{} değer", v.len()), format!("{}{more}.", shown.join(", ")))
                }
                _ => ("Liste yok".to_owned(), "Bir liste verilirse kayda kod yazılır, tabloda ve formda etiketi görünür.".to_owned()),
            };
            form = form
                .field(
                    "Değer listesi",
                    row![
                        label::body(words_now),
                        button(label::body(if r.values.as_ref().is_some_and(|v| !v.is_empty()) || linked.is_some() {
                            "Düzenle…"
                        } else {
                            "Liste ekle…"
                        }))
                            .style(style::button::secondary)
                            .padding(kentos_ui::theme::metrics::action_small())
                            .on_press(msg(Event::OpenList(i))),
                    ]
                    .spacing(12)
                    .align_y(Center),
                )
                .help(summary);
        }
        // Nesne işlemleri: what the value becomes when an object is split or objects are merged, and whether it leaves.
        let (split, merge) = policy_selects(i, r);
        form = form
            .section("Nesne işlemleri")
            .field("Nesne bölününce", short(split, 220.0))
            .field("Nesneler birleşince", short(merge, 220.0))
            .row(kentos_ui::widget::tip(
                check(r.withheld, "Dışarı verilmez", Some(msg(Event::Withheld(!r.withheld)))),
                kentos_ui::widget::Tip::new("Dışarı verilmez").body(
                    "Değerleri çizimde kalır: atlas, tablonun kopyası ve dosyası, istatistik, grafik ve komut kaydı onu \
                     dışarıda bırakır.",
                ),
                iced::widget::tooltip::Position::Top,
            ));
        // Kaynak: what the table keeps of it.
        let said = self.field_source_words(w, r);
        if !said.is_empty() {
            form = form.section("Kaynak");
            for line in said {
                form = form.row(label::caption(line).style(style::text::muted));
            }
        }
        let mut out = Column::new().spacing(14).padding([8, 14]).push(form);
        out = out.push(self.field_usage_box(w));
        if let Some(held) = self.table_constraints(w) {
            out = out.push(held);
        }
        out.into()
    }

    /// What the field's table keeps (docs/adr/0753, 0755 §5, 9324): its column, its unique constraint and where its
    /// rules are.
    fn field_source_words(&self, w: &Window, r: &FieldRow) -> Vec<String> {
        let Some(doc) = self.document.as_ref() else {
            return Vec::new();
        };
        let unique = doc
            .model
            .settings()
            .sources
            .iter()
            .find(|s| s.layer == w.layer)
            .is_some_and(|link| unique_in_table(link, &r.name));
        let ruled = r.values.as_ref().is_some_and(|v| !v.is_empty())
            || (r.kind.is_number() && (!r.min.trim().is_empty() || !r.max.trim().is_empty()));
        let database = doc
            .model
            .layers()
            .get(&w.layer)
            .and_then(|n| n.data.as_ref())
            .is_some_and(|d| d.driver == "postgis");
        let rule = w.source_rules.as_ref().filter(|_| ruled).map(|kept| {
            if kept.contains(&r.name) && database {
                "Kural proje ve kaynakta: KentOS yazarken denetler, veritabanı başka programların yazdığını da denetler."
                    .to_owned()
            } else if kept.contains(&r.name) {
                "Kural proje ve kaynakta: KentOS yazarken denetler, GeoPackage kuralı şema uzantısında tutar.".to_owned()
            } else {
                format!("Kural yalnız projede: tabloda kural yok, başka programlar denetlenmeden yazar. Diğer › {RULES}.")
            }
        });
        [
            column_said(r.column.as_ref()),
            unique.then(|| "Tabloda tekil: değer başka bir kayıtta olamaz.".to_owned()),
            rule,
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// The bound table's constraints (docs/adr/0755 §6); none without them.
    fn table_constraints<'a>(&self, w: &Window) -> Option<Element<'a, Message>> {
        let doc = self.document.as_ref()?;
        let link = doc
            .model
            .settings()
            .sources
            .iter()
            .find(|s| s.layer == w.layer)
            .filter(|s| !s.checks.is_empty() || s.uniques.iter().any(|u| u.columns.len() > 1))?;
        let fields = doc.model.layers().get(&w.layer).map(|n| n.fields.clone()).unwrap_or_default();
        let mut list = Column::new().spacing(4).push(label::strong("Tablonun kısıtları"));
        for (said, judged) in constraints_said(link, &fields) {
            list = list.push(row![container(label::body(said)).width(Fill), label::muted(judged)].spacing(12).align_y(Center));
        }
        Some(container(list).padding([8, 10]).style(style::container::field_box).width(Fill).into())
    }

    /// Değer listesi · <alan>: the codes, their labels, descriptions and days, or the project's list it links.
    fn value_list_window<'a>(&'a self, w: &'a Window, c: &'a super::Choices) -> Element<'a, Message> {
        let domains: &[kentos_commands::FieldDomain] =
            self.document.as_ref().map_or(&[], |d| d.model.settings().domains.as_slice());
        let name = w.rows.get(c.row).map(|r| r.name.clone()).unwrap_or_default();
        let row_kind = w.rows.get(c.row).map(|r| r.kind);
        let fitting: Vec<&kentos_commands::FieldDomain> = domains.iter().filter(|d| Some(d.kind) == row_kind).collect();
        let linked = c.domain.as_ref().and_then(|id| fitting.iter().find(|d| d.id == *id).copied());
        let own = linked.is_none();
        let heads = row![
            container(label::caption("Kod")).width(Length::Fixed(100.0)),
            container(label::caption("Etiket")).width(Length::Fixed(160.0)),
            container(label::caption("Açıklama")).width(Length::Fixed(180.0)),
            container(label::caption("Geçerlilik başı")).width(Length::Fixed(108.0)),
            container(label::caption("sonu")).width(Length::Fixed(108.0)),
            label::caption("Kalktı"),
        ]
        .spacing(6);
        let mut list = Column::new().spacing(4).push(heads);
        let shown_day = |d: &Option<String>| {
            d.as_deref().map_or_else(String::new, |d| {
                kentos_commands::display_value(&LayerField::new("gün", LayerFieldKind::Date), d)
            })
        };
        for (k, choice) in c.list.iter().enumerate() {
            let typed = |placeholder: &'static str, value: &str, on: fn(usize, String) -> Event| {
                let field = kentos_ui::widget::text_field(placeholder, value);
                if own { field.on_input(move |t| msg(on(k, t))) } else { field }
            };
            list = list.push(
                row![
                    container(typed("", &choice.code, Event::ListCode)).width(Length::Fixed(100.0)),
                    container(typed("", &choice.label, Event::ListLabel)).width(Length::Fixed(160.0)),
                    container(typed("", choice.description.as_deref().unwrap_or(""), Event::ListDescription))
                        .width(Length::Fixed(180.0)),
                    container(typed("GG.AA.YYYY", &shown_day(&choice.from), Event::ListFrom)).width(Length::Fixed(108.0)),
                    container(typed("GG.AA.YYYY", &shown_day(&choice.until), Event::ListUntil)).width(Length::Fixed(108.0)),
                    kentos_ui::widget::tip(
                        words::check(choice.retired, "", own.then(|| msg(Event::ListRetired(k, !choice.retired)))),
                        kentos_ui::widget::Tip::new("Kullanımdan kalktı")
                            .body("Tutan kayıtta kalır ve görünür; yeni seçimde sunulmaz, formla yeniden yazılmaz."),
                        iced::widget::tooltip::Position::Top,
                    ),
                    button(icon(Icon::Close).size(12.0))
                        .style(style::button::subtle)
                        .padding(kentos_ui::theme::metrics::icon_small(12.0))
                        .on_press_maybe(own.then(|| msg(Event::ListRemove(k)))),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        let ids: Vec<String> = fitting.iter().map(|d| d.id.clone()).collect();
        let mut offered = vec![Choice::new("Alanın kendi listesi")];
        offered.extend(fitting.iter().map(|d| {
            Choice::new(if d.coded() {
                format!("{} ({} kod)", d.name, d.values.as_ref().map_or(0, Vec::len))
            } else {
                format!("{} ({} – {})", d.name, d.min.as_deref().unwrap_or("…"), d.max.as_deref().unwrap_or("…"))
            })
        }));
        let now = match &c.domain {
            None => Some(0),
            Some(id) => ids.iter().position(|x| x == id).map(|p| p + 1),
        };
        let pick = Select::new(offered, now, move |k| msg(Event::ListDomain(k.checked_sub(1).map(|k| ids[k].clone()))));
        let mut body = Column::new().spacing(10).push(words::field(
            "Projenin listesi",
            container(pick).width(320),
            Some(match linked {
                Some(d) => format!(
                    "Kodlar “{}” listesinindir; Değer listeleri penceresinde düzenlenir, bağlı bütün alanlar izler.",
                    d.name
                ),
                None if fitting.is_empty() => {
                    "Projede bu türde değer listesi yok; Değer listeleri penceresinden ekleyin.".to_owned()
                }
                None => "Bir liste seçilirse alan ona bağlanır; liste değişince alan da değişir.".to_owned(),
            }),
        ));
        match linked.filter(|d| !d.coded()) {
            Some(d) => {
                body = body.push(label::caption(format!(
                    "Aralık: {} – {}",
                    d.min.as_deref().unwrap_or("…"),
                    d.max.as_deref().unwrap_or("…")
                )));
            }
            None => {
                body = body
                    .push(label::caption(
                        "Kayda kod yazılır, tabloda ve formda etiketi görünür; etiket yazılırsa kodu yazılır.",
                    ))
                    .push(list);
                if own {
                    body = body.push(
                        button(row![icon(from_web(Some("plus"))).size(14.0), label::body("Değer ekle")].spacing(6).align_y(Center))
                            .style(style::button::secondary)
                            .padding(kentos_ui::theme::metrics::action())
                            .on_press(msg(Event::ListAdd)),
                    );
                }
            }
        }
        if w.rows.get(c.row).is_some_and(|r| r.kind == LayerFieldKind::Text) {
            body = body.push(words::check(
                c.multiple,
                "Birden çok değer seçilebilir",
                Some(msg(Event::ListMultiple(!c.multiple))),
            ));
        }
        Frame::new(format!("Değer listesi · {name}"))
            .push(body)
            .action(words::secondary(CANCEL, Some(msg(Event::ListCancel))))
            .action(words::primary("Tamam", Some(msg(Event::ListOk))))
            .width(820.0)
            .modal(msg(Event::ListCancel))
    }
}

/// Under the list: Alan ekle, and Sil, Yukarı and Aşağı on the chosen field (their names in their tips).
fn fields_list_foot<'a>(w: &Window) -> Element<'a, Message> {
    let n = w.rows.len();
    let chosen = w.chosen;
    let small = |glyph: &str, words: &'static str, on: Option<Message>| {
        kentos_ui::widget::tip(
            button(icon(from_web(Some(glyph))).size(14.0))
                .style(style::button::subtle)
                .padding(kentos_ui::theme::metrics::icon_small(14.0))
                .on_press_maybe(on),
            kentos_ui::widget::Tip::from(words),
            iced::widget::tooltip::Position::Top,
        )
    };
    container(
        row![
            button(row![icon(from_web(Some("plus"))).size(14.0), label::body(ADD)].spacing(6).align_y(Center))
                .style(style::button::secondary)
                .padding(kentos_ui::theme::metrics::action())
                .on_press(msg(Event::Add)),
            space::horizontal(),
            small("chevronUp", UP, chosen.filter(|&i| i > 0).map(|_| msg(Event::Up))),
            small("chevronDown", DOWN, chosen.filter(|&i| i + 1 < n).map(|_| msg(Event::Down))),
            small("erase", REMOVE, chosen.map(|_| msg(Event::Remove))),
        ]
        .spacing(4)
        .align_y(Center),
    )
    .padding([6, 8])
    .into()
}

/// What the window says, left of its buttons: what it last did, else the fields' first problem, else how many fields
/// and how many are renamed.
fn fields_status<'a>(w: &Window) -> Element<'a, Message> {
    let problem = problem_of(&w.rows);
    let renamed = renames_of(&w.rows).len();
    let n = w.rows.len();
    let line = match (&w.said, &problem) {
        (Some((kind, text)), _) => words::text_line(*kind, text.clone()),
        (None, Some(p)) => words::text_line(Kind::Error, p.clone()),
        (None, None) if w.rows.is_empty() => words::text_line(Kind::Ok, "Alan yok: katmanın öznitelikleri serbest.".to_owned()),
        (None, None) if renamed > 0 => words::text_line(
            Kind::Ok,
            format!("{n} alan; {renamed} alanın adı değişiyor, değerleri yeni adla taşınacak."),
        ),
        (None, None) if w.changed() => words::text_line(Kind::Ok, format!("{n} alan; değişiklikler Kaydet ile yazılır.")),
        (None, None) => words::text_line(Kind::Ok, format!("{n} alan.")),
    };
    line
}

/// Bölünce and Birleşince of the row (docs/adr/0419): only what its kind takes.
fn policy_selects<'a>(i: usize, r: &FieldRow) -> (Element<'a, Message>, Element<'a, Message>) {
    use kentos_commands::{MergePolicy, SplitPolicy};
    let units = matches!(r.kind, LayerFieldKind::Integer | LayerFieldKind::BigInt)
        || (r.kind == LayerFieldKind::Decimal && !r.size.trim().is_empty());
    let number = r.kind.is_number();
    let splits: Vec<SplitPolicy> = SplitPolicy::ALL.into_iter().filter(|p| *p != SplitPolicy::Ratio || units).collect();
    let merges: Vec<MergePolicy> = MergePolicy::ALL
        .into_iter()
        .filter(|p| match p {
            MergePolicy::Sum => units,
            MergePolicy::Min | MergePolicy::Max => number,
            _ => true,
        })
        .collect();
    let split_at = splits.iter().position(|p| Some(*p) == r.split.or(Some(SplitPolicy::Copy)));
    let merge_at = merges.iter().position(|p| Some(*p) == r.merge.or(Some(MergePolicy::First)));
    let split = {
        let picks = splits.clone();
        Select::new(splits.iter().map(|p| Choice::new(p.label())), split_at, move |k| {
            let p = picks[k];
            msg(Event::Split(i, (p != SplitPolicy::Copy).then_some(p)))
        })
    };
    let merge = {
        let picks = merges.clone();
        Select::new(merges.iter().map(|p| Choice::new(p.label())), merge_at, move |k| {
            let p = picks[k];
            msg(Event::Merge(i, (p != MergePolicy::First).then_some(p)))
        })
    };
    (split.into(), merge.into())
}
