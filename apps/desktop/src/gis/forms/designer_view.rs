//! Formlar's view (docs/adr/0401): the forms' list and the layer's free fields, the item tree with
//! its toolbar and the preflight's issues, the chosen item's inspector and the footer.

use iced::widget::{Column, Row, button, column, container, row, scrollable, space};
use iced::{Center, Element, Fill};
use kentos_commands::{
    FormBody, FormIssue, FormItem, FormSeverity, FormWidget, LayerField, LayerNodeType, ProjectForm,
};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::panes::Side as PaneSide;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{
    Dialog as Frame, Elided, Menu, MenuButton, Panes, Segmented, fallback, segmented,
};
use kentos_ui::{label, style};

use kentos_ui::widget::tree_view::{self, Node, TreeView};

use super::designer::{
    ADD_ACTION, ADD_ATTACHMENTS, ADD_COMMENT, ADD_DIVIDER, ADD_GROUP, ADD_LOCATION, ADD_NOTE,
    ADD_REFERENCE, ADD_TAB, AUDIT, CANCEL, CHANGES, COPY_ITEM, CREATE, DOMAINS_WINDOW, DOWN,
    DROP_KEPT, ENFORCE, EXPORT, Event, FLOW, FORM_SETTINGS, IMPORT, LookupEdit, NAME, NEW_FORM,
    PALETTE_DOMAINS, PALETTE_FIELDS, PALETTE_RELATIONS, PALETTE_SECTIONS, PASTE_ITEM, PREVIEW,
    PROPERTIES, PUBLISH, PUBLISH_ANYWAY, PUT_BACK, Palette, RECOMPUTE, REDO, RELATED_RECORDS,
    REMOVE_FORM, REMOVE_ITEM, RESTORE_DRAFT, Rule, SAVE, SAVE_SECTION, SECTION_CHECKS,
    SECTION_GENERAL, SECTION_MORE, SECTION_RULES, SECTION_VALUE, Said, TITLE, Text, UNDO, UP,
    WITHDRAW, Window, bound, find, msg,
};
use crate::app::{App, Message};

/// The item's properties' scroll, which a picture moves to the list's extras.
pub(crate) const PROPERTIES_SCROLL: &str = "forms-properties";
use crate::exchange::words::{self, Kind};

/// An item's words as the tree shows them (a section's name by default).
pub(crate) fn item_words(item: &FormItem) -> String {
    let words = item_face(item, &[]).1;
    if words.trim().is_empty() {
        item.id().to_owned()
    } else {
        words
    }
}

/// A row of the item tree: its depth, its icon and words.
fn item_face(item: &FormItem, fields: &[LayerField]) -> (Icon, String, bool) {
    match item {
        FormItem::Tab { label, .. } => (Icon::Tabs, label.clone(), false),
        FormItem::Group { label, .. } => (
            Icon::Layout,
            label.clone().unwrap_or_else(|| "Grup".into()),
            false,
        ),
        FormItem::Note { text, .. } => (
            Icon::Info,
            text.lines().next().unwrap_or("").to_owned(),
            false,
        ),
        FormItem::Related {
            label,
            visible_when,
            ..
        } => (
            Icon::Link,
            label.clone().unwrap_or_else(|| "İlişkili kayıtlar".into()),
            visible_when.is_some(),
        ),
        FormItem::Attachments {
            label,
            visible_when,
            ..
        } => (
            crate::icons::from_web(Some("photo")),
            label.clone().unwrap_or_else(|| "Ekler".into()),
            visible_when.is_some(),
        ),
        FormItem::Location {
            label,
            visible_when,
            ..
        } => (
            crate::icons::from_web(Some("pin")),
            label.clone().unwrap_or_else(|| "Konum".into()),
            visible_when.is_some(),
        ),
        FormItem::Divider {
            label,
            visible_when,
            ..
        } => (
            Icon::Minus,
            label.clone().unwrap_or_else(|| "Bölücü".into()),
            visible_when.is_some(),
        ),
        FormItem::Action {
            label,
            visible_when,
            ..
        } => (Icon::Play, label.clone(), visible_when.is_some()),
        FormItem::Field(f) => {
            let layer = fields.iter().find(|l| l.key() == f.field);
            let name = f
                .label
                .clone()
                .or_else(|| layer.map(|l| l.label().to_owned()))
                .unwrap_or_else(|| f.field.clone());
            let ruled = f.visible_when.is_some()
                || f.required_when.is_some()
                || f.read_only_when.is_some()
                || f.default_expr.is_some()
                || f.calculate.is_some()
                || f.choice_filter.is_some()
                || f.lookup.is_some()
                || !f.checks.is_empty();
            let mut words = name;
            if f.mirror {
                // A mirror shows the field again, read-only (docs/adr/0437).
                words.push_str(" · yansıma");
                return (Icon::Link, words, ruled);
            }
            if f.required || layer.is_some_and(|l| l.required) {
                words.push_str(" *");
            }
            let glyph = layer.map_or(Icon::Type, |l| {
                widget_glyph(kentos_forms::widget_of(f, l), l.kind)
            });
            (glyph, words, ruled)
        }
    }
}

/// A field's widget as an icon, in the tree and in the widget's choices (docs/adr/0751): what the record shows,
/// the kind telling a date from a time, a big number from a text, JSON and bytes from text.
fn widget_glyph(widget: FormWidget, kind: kentos_commands::LayerFieldKind) -> Icon {
    use kentos_commands::LayerFieldKind as K;
    match (widget, kind) {
        (_, K::Json) => Icon::Svg(glyphs::JSON),
        (_, K::Binary) => Icon::Document,
        (FormWidget::Date, K::Time) => Icon::Clock,
        (FormWidget::Text, K::BigInt) | (FormWidget::Number, _) => Icon::Hash,
        (FormWidget::Text, K::Instant) | (FormWidget::Date, _) => Icon::Calendar,
        (FormWidget::Text, _) => Icon::Type,
        (FormWidget::TextArea, _) => crate::icons::from_web(Some("mtext")),
        (FormWidget::Slider, _) => Icon::Slider,
        (FormWidget::Check, _) => Icon::Svg(glyphs::CHECK),
        (FormWidget::Switch, _) => Icon::Svg(glyphs::SWITCH),
        (FormWidget::List, _) => Icon::Svg(glyphs::LIST),
        (FormWidget::Radio, _) => Icon::Svg(glyphs::RADIO),
        (FormWidget::Segmented, _) => Icon::Svg(glyphs::SEGMENTED),
        (FormWidget::Color, _) => crate::icons::from_web(Some("color")),
        (FormWidget::CheckList, _) => Icon::Svg(glyphs::CHECK_LIST),
        (FormWidget::Search, _) => Icon::Search,
        (FormWidget::Chips, _) => Icon::Svg(glyphs::CHIPS),
        (FormWidget::Tree, _) => Icon::Svg(glyphs::TREE),
    }
}

/// The widgets' own icons, on the web set's 20 × 20 grid (docs/adr/0751).
mod glyphs {
    /// A box ticked: one yes or no.
    pub(super) const CHECK: &str = r#"<rect x="3.5" y="3.5" width="13" height="13" rx="2"/><path d="m6.6 10.2 2.3 2.3 4.5-4.9"/>"#;
    /// A switch on.
    pub(super) const SWITCH: &str = r#"<rect x="2" y="6" width="16" height="8" rx="4"/><circle cx="14" cy="10" r="2.3" fill="currentColor" stroke="none"/>"#;
    /// A field with its drop-down's arrow.
    pub(super) const LIST: &str = r#"<rect x="2.5" y="5.5" width="15" height="9" rx="1.5"/><path d="M5.2 10h4.3M12 9l1.6 1.6L15.2 9"/>"#;
    /// A radio button chosen.
    pub(super) const RADIO: &str = r#"<circle cx="10" cy="10" r="6.5"/><circle cx="10" cy="10" r="2.7" fill="currentColor" stroke="none"/>"#;
    /// Three parts, the first chosen.
    pub(super) const SEGMENTED: &str = r#"<rect x="2.5" y="6" width="15" height="8" rx="1.5"/><path d="M7.5 6v8M12.5 6v8"/><rect x="2.5" y="6" width="5" height="8" rx="1.5" fill="currentColor" fill-opacity=".35" stroke="none"/>"#;
    /// Two values ticked in a list.
    pub(super) const CHECK_LIST: &str = r#"<rect x="2.5" y="3.5" width="5" height="5" rx="1"/><path d="m3.8 6 .9.9 1.6-1.8M10 6h7.5"/><rect x="2.5" y="11.5" width="5" height="5" rx="1"/><path d="m3.8 14 .9.9 1.6-1.8M10 14h7.5"/>"#;
    /// Two tags: the codes held as chips.
    pub(super) const CHIPS: &str = r#"<path d="M7 7.6V4.5h7.3l3.2 3.2-3.2 3.2h-1.2"/><path d="M2.5 8.5h8.3l3.2 3.5-3.2 3.5H2.5z"/><circle cx="5" cy="12" r=".85" fill="currentColor" stroke="none"/>"#;
    /// A branch and its two leaves.
    pub(super) const TREE: &str = r#"<rect x="2.5" y="2.5" width="5.5" height="4" rx="1"/><path d="M5.2 6.5V15h4.3M5.2 10h4.3"/><rect x="9.5" y="8" width="8" height="4" rx="1"/><rect x="9.5" y="13" width="8" height="4" rx="1"/>"#;
    /// Braces: a JSON value.
    pub(super) const JSON: &str = r#"<path d="M7.5 3C5.6 3 5.6 4.4 5.6 6.4S4.6 10 3 10c1.6 0 2.6 1 2.6 3.6S5.6 17 7.5 17M12.5 3c1.9 0 1.9 1.4 1.9 3.4S15.4 10 17 10c-1.6 0-2.6 1-2.6 3.6s0 3.4-1.9 3.4"/>"#;
}

impl App {
    pub(crate) fn forms_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.gis.forms_window else {
            return iced::widget::text("").into();
        };
        let locked = self.forms_locked();
        let form = self.chosen_form();
        let fields = form
            .map(|f| self.layer_fields_of(&f.layer))
            .unwrap_or_default();
        let issues = w
            .body
            .as_ref()
            .map(|b| {
                let mut issues = self.issues_of(b, &fields);
                // The data layer's table checked too (docs/adr/9318).
                if let Some(f) = form {
                    issues.extend(self.source_issues(&f.layer, b));
                }
                issues
            })
            .unwrap_or_default();

        let left = self.forms_list(w, locked.is_none(), &fields);
        let center = match (&w.body, form) {
            (Some(body), Some(_)) => self.forms_tree(w, body, &fields, &issues),
            _ => container(label::caption(
                "Projede form yok. Yeni form ile bir katmanın alanlarından form oluşturun.",
            ))
            .padding(16)
            .into(),
        };
        let inspector = if w.flow {
            self.forms_flow(w)
        } else if w.changes {
            self.forms_changes(w)
        } else if w.previewing {
            self.forms_preview(w)
        } else {
            match (&w.body, &w.item) {
                (Some(body), Some(id)) => match (find(&body.items, id), form) {
                    (Some(item), _) => self.forms_inspector(item, &fields, &issues),
                    (None, Some(f)) => self.form_overview(w, f, body, locked.is_none()),
                    (None, None) => empty_inspector(),
                },
                (Some(body), None) => match form {
                    Some(f) => self.form_overview(w, f, body, locked.is_none()),
                    None => empty_inspector(),
                },
                _ => empty_inspector(),
            }
        };
        // Özellikler | Önizleme | Değişiklikler over the third column (docs/adr/0403 §3, 0427).
        let side = Segmented::new(
            [PROPERTIES, PREVIEW, CHANGES, FLOW],
            if w.flow {
                FLOW
            } else if w.changes {
                CHANGES
            } else if w.previewing {
                PREVIEW
            } else {
                PROPERTIES
            },
            |t| {
                if t == FLOW {
                    msg(Event::Flow)
                } else if t == CHANGES {
                    msg(Event::Changes)
                } else {
                    msg(Event::Preview(t == PREVIEW))
                }
            },
        );
        let inspector: Element<'_, Message> = column![side, inspector].spacing(8).into();
        // The panes from the window's width and the text (docs/adr/0749): the list and the inspector their
        // shares, at least as wide as their tabs, the tree the rest; the inspector keeps its width whichever
        // tab shows, so nothing moves sideways when one is pressed.
        let main = Panes::new(
            container(left).height(Fill),
            container(center).height(Fill),
            container(inspector).height(Fill),
        )
        .left(0.2, 200.0, 260.0)
        .right(0.36, 330.0, 480.0)
        .middle(300.0)
        .least(
            PaneSide::Left,
            segmented::natural_width(
                &[
                    PALETTE_FIELDS,
                    PALETTE_RELATIONS,
                    PALETTE_DOMAINS,
                    PALETTE_SECTIONS,
                ],
                true,
            ),
        )
        .least(
            PaneSide::Right,
            segmented::natural_width(&[PROPERTIES, PREVIEW, CHANGES, FLOW], false),
        );
        // A window given a size of its own (docs/adr/0746): the three panes take what it has; else they are
        // 460 high (at 12 px text), less in a low window, the buttons under them in sight (docs/adr/0749). 390 while
        // the form's name, scope and switches were two lines under them; they are Form ayarları's now.
        let sized = kentos_ui::widget::sizable::sizes::get(TITLE).is_some();
        let main = container(main).height(Fill).max_height(if sized {
            f32::INFINITY
        } else {
            typography::scaled(460.0)
        });

        let mut above: Vec<Element<'_, Message>> = Vec::new();
        if let Some(why) = locked {
            above.push(words::summary(vec![words::text_line(Kind::Warn, why)]));
        }
        // The layer's fields changed since the latest release (docs/adr/0444 §2): published again over them.
        if let Some(r) = form.and_then(|f| f.releases.last())
            && r.fields
                .as_ref()
                .is_some_and(|d| *d != kentos_commands::bound_fields_digest(&r.body, &fields))
        {
            above.push(words::summary(vec![words::text_line(
                Kind::Warn,
                format!(
                    "Katmanın alanları {}. yayından sonra değişti; formu gözden geçirip yeniden yayınlayın (taslak aynıysa da yayınlanır).",
                    r.version
                ),
            )]));
        }
        // The records the release would break (docs/adr/0442 §2): published only when asked again.
        if let Some(b) = &w.breaking {
            let fields: Vec<String> = b
                .fields
                .iter()
                .map(|f| {
                    let name = fields
                        .iter()
                        .find(|l| l.key() == f.field)
                        .map_or(f.field.clone(), |l| l.label().to_owned());
                    format!("{name} ({})", f.records)
                })
                .collect();
            above.push(
                container(
                    row![
                        icon(Icon::Info).size(14.0),
                        column![
                            label::strong(format!(
                                "Yeni yayına {} kayıt uymuyor; önceki yayına uyuyorlardı.",
                                b.records
                            )),
                            label::caption(format!(
                                "Alanlar: {}. Kayıtları denetle ile görebilir, yayından önce düzeltebilirsiniz.",
                                fields.join(", ")
                            ))
                            .style(muted),
                        ]
                        .spacing(2)
                        .width(Fill),
                        words::secondary(CANCEL, Some(msg(Event::DropPublish))),
                        words::primary(PUBLISH_ANYWAY, Some(msg(Event::PublishAnyway))),
                    ]
                    .spacing(8)
                    .align_y(Center),
                )
                .padding([6, 10])
                .style(style::container::field_box)
                .width(Fill)
                .into(),
            );
        }
        // A draft kept on this device that is not the form's (docs/adr/0441 §5).
        if let Some(k) = &w.kept {
            above.push(
                container(
                    row![
                        icon(Icon::Info).size(14.0),
                        label::body(format!(
                            "Bu cihazda “{}” formunun kaydedilmemiş bir taslağı kaldı.",
                            k.name
                        )),
                        space::horizontal().width(Fill),
                        words::secondary(DROP_KEPT, Some(msg(Event::DropKept))),
                        words::primary(RESTORE_DRAFT, Some(msg(Event::RestoreDraft))),
                    ]
                    .spacing(8)
                    .align_y(Center),
                )
                .padding([6, 10])
                .style(style::container::field_box)
                .width(Fill)
                .into(),
            );
        }
        // The window's name, a dot while the copy is unsaved: the form is the chosen row of the list.
        let title = if form.is_some() && w.dirty {
            format!("{TITLE} ●")
        } else {
            TITLE.to_owned()
        };
        // The warnings, the panes taking what is left, the footer (docs/adr/0749); a project with no form its
        // invitation instead of the panes.
        let mut frame = Frame::new(title);
        for part in above {
            frame = frame.push(part);
        }
        frame = if self.project_forms().is_empty() {
            frame.fill(
                container(self.forms_empty(w, locked.is_none()))
                    .height(typography::scaled(360.0)),
            )
        } else {
            frame.fill(main)
        };
        if form.is_some() {
            frame = frame.footer(self.forms_footer(form, &issues, locked.is_none()));
        }
        frame.key(TITLE).width(1040.0).modal(msg(Event::Cancel))
    }

    fn forms_list<'a>(
        &'a self,
        w: &'a Window,
        editable: bool,
        fields: &[LayerField],
    ) -> Element<'a, Message> {
        let mut list = Column::new().spacing(1).padding(2);
        for f in self.project_forms() {
            let chosen = w.chosen.as_deref() == Some(f.id.as_str());
            // Its layer and where it stands, as a sentence (DESIGN.md §13: no “A · B · C” lines).
            let mut caption = format!("{} katmanı", self.form_layer_name(&f.layer));
            match f.releases.last() {
                Some(r) => caption.push_str(&format!(", {}. yayın", r.version)),
                None => caption.push_str(", taslak"),
            }
            if f.enforce {
                caption.push_str(", kuralları uygulanıyor");
            }
            let face = column![
                Elided::new(f.name.clone())
                    .size(typography::body())
                    .font(typography::ui())
                    .width(Fill),
                label::caption(caption).style(muted),
            ]
            .spacing(1);
            list = list.push(
                button(face)
                    .on_press(msg(Event::Choose(f.id.clone())))
                    .padding([4, 8])
                    .width(Fill)
                    .style(style::button::row(chosen)),
            );
        }
        // As tall as its forms, up to four of them; more scroll.
        let shown = self.project_forms().len().clamp(1, 4) as f32;
        let mut col = Column::new()
            .spacing(8)
            .push(label::strong("Formlar"))
            .push(
                container(scrollable(list).height(typography::scaled(6.0 + 42.0 * shown)))
                    .style(style::container::field_box)
                    .width(Fill),
            );
        match &w.new_form {
            Some(n) => col = col.push(self.new_form_fields(n, editable)),
            None => {
                col = col.push(words::secondary(
                    NEW_FORM,
                    editable.then(|| msg(Event::NewForm)),
                ));
            }
        }
        if w.body.is_some() {
            // What goes on the form: the layer's fields, its relations, the value lists, the saved sections.
            col = col.push(label::strong("Forma ekle"));
            let tabs = Segmented::new(
                [
                    PALETTE_FIELDS,
                    PALETTE_RELATIONS,
                    PALETTE_DOMAINS,
                    PALETTE_SECTIONS,
                ],
                match w.palette {
                    Palette::Fields => PALETTE_FIELDS,
                    Palette::Relations => PALETTE_RELATIONS,
                    Palette::Domains => PALETTE_DOMAINS,
                    Palette::Sections => PALETTE_SECTIONS,
                },
                |t| {
                    msg(Event::Palette(match t {
                        PALETTE_RELATIONS => Palette::Relations,
                        PALETTE_DOMAINS => Palette::Domains,
                        PALETTE_SECTIONS => Palette::Sections,
                        _ => Palette::Fields,
                    }))
                },
            )
            .compact();
            col = col.push(tabs);
            match w.palette {
                Palette::Fields => {}
                Palette::Relations => return col.push(self.relations_palette(editable)).into(),
                Palette::Domains => return col.push(self.domains_palette(fields)).into(),
                Palette::Sections => {
                    return col.push(self.sections_palette(editable, fields)).into();
                }
            }
        }
        // The layer's fields the form does not show yet: a press puts one where the chosen item is.
        if let Some(body) = &w.body {
            let on_form = bound(&body.items);
            let free: Vec<&LayerField> = fields
                .iter()
                .filter(|f| !on_form.iter().any(|k| k == f.key()))
                .collect();
            let mut palette = Column::new().spacing(2);
            // A layer without a field says so (not that all of them are on the form); Yeni alan makes one (docs/adr/9141).
            if fields.is_empty() {
                palette = palette.push(
                    label::caption(
                        "Katmanda alan yok: aşağıdan bir alan oluşturun, forma da eklenir.",
                    )
                    .style(muted),
                );
            } else if free.is_empty() {
                palette =
                    palette.push(label::caption("Katmanın bütün alanları formda.").style(muted));
            }
            for f in free {
                let face = row![
                    icon(Icon::Plus).size(12.0).tone(Tone::Muted),
                    label::body(f.label().to_owned()),
                    space::horizontal().width(Fill),
                    label::caption(f.kind.label()).style(muted),
                ]
                .spacing(6)
                .align_y(Center);
                palette = palette.push(
                    button(face)
                        .on_press_maybe(editable.then(|| msg(Event::AddField(f.key().to_owned()))))
                        .padding([3, 8])
                        .width(Fill)
                        .style(style::button::row(false)),
                );
            }
            col = col.push(scrollable(palette).height(Fill));
            if editable {
                col = col.push(self.new_field_row(w));
            }
        }
        col.into()
    }

    /// Yeni form's fields: the form's name, its layer, Oluştur and Vazgeç.
    fn new_form_fields<'a>(
        &'a self,
        n: &'a crate::forms::designer::NewForm,
        editable: bool,
    ) -> Element<'a, Message> {
        let layers = self.form_layers();
        let at = n
            .layer
            .as_ref()
            .and_then(|l| layers.iter().position(|(id, _)| id == l));
        let picks = layers.clone();
        let pick = Select::new(
            layers.iter().map(|(_, name)| Choice::new(name.clone())),
            at,
            move |i| msg(Event::NewLayer(picks[i].0.clone())),
        )
        .placeholder("Katman");
        let name = kentos_ui::widget::text_field("Boşsa katmanın adı", &n.name)
            .on_input(|t| msg(Event::NewName(t)))
            .on_submit(msg(Event::Create));
        column![
            words::field("Formun adı", name, None),
            words::field("Katman", pick, None),
            row![
                words::primary(CREATE, editable.then(|| msg(Event::Create))),
                words::secondary(CANCEL, Some(msg(Event::CancelNew))),
            ]
            .spacing(8),
        ]
        .spacing(8)
        .into()
    }

    /// A project with no form: one invitation in the middle (what a form is, Yeni form), not three empty columns with
    /// tabs that have nothing to show.
    fn forms_empty<'a>(&'a self, w: &'a Window, editable: bool) -> Element<'a, Message> {
        let mut col = Column::new()
            .spacing(12)
            .max_width(typography::scaled(420.0))
            .push(icon(Icon::Table).size(32.0).tone(Tone::Muted))
            .push(label::strong("Henüz form yok"))
            .push(
                label::body(
                    "Form, bir katmanın kayıtlarının nasıl girileceğini tarif eder: hangi alanların, hangi sırayla \
                     ve hangi kurallarla. Bir katmanın alanlarından başlanır; sekme, grup ve kurallar sonra eklenir.",
                )
                .style(muted),
            );
        col = match &w.new_form {
            Some(n) => col.push(self.new_form_fields(n, editable)),
            None => col.push(words::primary(NEW_FORM, editable.then(|| msg(Event::NewForm)))),
        };
        container(col).center(Fill).into()
    }

    /// Yeni alan (docs/adr/9141): a name and a kind, Oluştur makes the layer's field and puts it on the form.
    fn new_field_row(&self, w: &crate::forms::designer::Window) -> Element<'_, Message> {
        // On a data layer (docs/adr/9318): the column its table gets, said before it is added.
        if let Some(ask) = &w.column_ask {
            let rows = match ask.rows {
                Some(0) => "Tabloda henüz satır yok.".to_owned(),
                Some(n) => format!("Var olan {n} satırda bu alan boş kalır."),
                None => "Var olan satırlarda bu alan boş kalır.".to_owned(),
            };
            return Column::new()
                .spacing(6)
                .push(label::caption("Veri kaynağına sütun ekle").style(muted))
                .push(label::body(format!(
                    "“{}” tablosuna “{}” sütunu ({}) eklenecek. {rows}",
                    ask.table,
                    ask.field.name,
                    ask.field.kind.label()
                )))
                .push(
                    label::caption("Tablodaki sütun geri alınmaz; katmanın alanı Ctrl+Z ile geri alınır.")
                        .style(muted),
                )
                .push(
                    row![
                        button(label::body("Sütunu ekle"))
                            .on_press(msg(Event::ConfirmColumn))
                            .style(style::button::primary),
                        button(label::body("Vazgeç"))
                            .on_press(msg(Event::CancelColumn))
                            .style(style::button::secondary),
                    ]
                    .spacing(6),
                )
                .into();
        }
        let kinds = kentos_commands::LayerFieldKind::ALL;
        let chosen = kinds.iter().position(|k| {
            *k == w
                .new_field_kind
                .unwrap_or(kentos_commands::LayerFieldKind::Text)
        });
        let name = kentos_ui::widget::text_field("Yeni alanın adı", &w.new_field)
            .on_input(|t| msg(Event::NewFieldName(t)))
            .on_submit(msg(Event::CreateField));
        let kind = kentos_ui::widget::select::Select::new(
            kinds
                .iter()
                .map(|k| kentos_ui::widget::select::Choice::new(k.label())),
            chosen,
            move |i| msg(Event::NewFieldKind(kinds[i])),
        );
        Column::new()
            .spacing(4)
            .push(label::caption("Yeni alan").style(muted))
            .push(name)
            .push(
                row![
                    container(kind).width(Fill),
                    button(label::body("Oluştur"))
                        .on_press_maybe(
                            (!w.new_field.trim().is_empty()).then(|| msg(Event::CreateField))
                        )
                        .style(style::button::secondary),
                ]
                .spacing(6)
                .align_y(Center),
            )
            .into()
    }

    fn forms_tree(
        &self,
        w: &Window,
        body: &FormBody,
        fields: &[LayerField],
        issues: &[FormIssue],
    ) -> Element<'_, Message> {
        let chosen = w.item.as_deref();
        let icon_tool = |words: &'static str, glyph: Icon, event: Option<Event>| {
            kentos_ui::widget::tip(
                button(icon(glyph).size(14.0))
                    .on_press_maybe(event.map(msg))
                    .padding(kentos_ui::theme::metrics::icon_small(14.0))
                    .style(style::button::tool(false)),
                words.into(),
                iced::widget::tooltip::Position::Bottom,
            )
        };
        let has_item = chosen.is_some();
        // Ekle ▾ (docs/adr/0441 §1): every kind of item by its name and what it is for, where the chosen item is. Nine
        // tools side by side, six of them by their icons alone, said nothing of what they put on the form.
        let line = |named: bool| {
            let add_face = container(
                row![
                    icon(Icon::Plus).size(13.0),
                    label::body("Ekle"),
                    icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted),
                ]
                .spacing(5)
                .align_y(Center),
            )
            .padding([4, 8])
            .style(style::container::field_box);
            let add = MenuButton::new(add_face, || {
                Menu::new()
                    .header("Düzen")
                    .item(ADD_TAB, Some(msg(Event::AddTab)))
                    .icon(Icon::Tabs)
                    .detail("Formu sekmelere böler; öğeler sekmelerin içine girer.")
                    .item(ADD_GROUP, Some(msg(Event::AddGroup)))
                    .icon(Icon::Layout)
                    .detail("Alanları başlıklı bir kutuda toplar.")
                    .item(ADD_DIVIDER, Some(msg(Event::AddDivider)))
                    .icon(Icon::Minus)
                    .detail("Bölümleri ayıran bir çizgi, istenirse adıyla.")
                    .item(ADD_NOTE, Some(msg(Event::AddNote)))
                    .icon(Icon::Info)
                    .detail("Formu dolduran kişinin okuyacağı bir açıklama.")
                    .header("Kayıt")
                    .item(RELATED_RECORDS, Some(msg(Event::AddRelated)))
                    .icon(Icon::Link)
                    .detail("Bağlı katmanın kayıtları: liste ya da tablo.")
                    .item(ADD_REFERENCE, Some(msg(Event::AddReference)))
                    .icon(Icon::Target)
                    .detail("Kaydın bağlı olduğu üst kayıt.")
                    .item(ADD_ATTACHMENTS, Some(msg(Event::AddAttachments)))
                    .icon(crate::icons::from_web(Some("photo")))
                    .detail("Fotoğraf ve belge ekleme yeri.")
                    .item(ADD_LOCATION, Some(msg(Event::AddLocation)))
                    .icon(crate::icons::from_web(Some("pin")))
                    .detail("Kaydın haritadaki yeri.")
                    .item(ADD_ACTION, Some(msg(Event::AddAction)))
                    .icon(Icon::Play)
                    .detail("Bir işlem aracını ya da iş akışı adımını çalıştıran düğme.")
            });
            let face: Element<'_, Message> = if named {
                row![
                    icon(Icon::Properties).size(13.0),
                    label::body(FORM_SETTINGS)
                ]
                .spacing(5)
                .align_y(Center)
                .into()
            } else {
                icon(Icon::Properties).size(14.0).into()
            };
            let settings = kentos_ui::widget::tip(
                button(face)
                    .on_press(msg(Event::FormSettings))
                    .padding(if named {
                        iced::Padding::from([4, 8])
                    } else {
                        kentos_ui::theme::metrics::icon_small(14.0)
                    })
                    .style(style::button::tool(
                        !has_item && !w.previewing && !w.changes && !w.flow,
                    )),
                "Formun adı, kapsamı ve davranışı".into(),
                iced::widget::tooltip::Position::Bottom,
            );
            // One line: what is added, what is done to the chosen item, the form's settings, undo and redo.
            row![
                add,
                space::horizontal().width(4),
                icon_tool(UP, Icon::ChevronUp, has_item.then_some(Event::Move(-1))),
                icon_tool(DOWN, Icon::ChevronDown, has_item.then_some(Event::Move(1))),
                icon_tool(COPY_ITEM, Icon::Copy, has_item.then_some(Event::CopyItem)),
                icon_tool(
                    PASTE_ITEM,
                    crate::icons::from_web(Some("paste")),
                    self.form_clipboard.is_some().then_some(Event::PasteItem)
                ),
                icon_tool(
                    REMOVE_ITEM,
                    Icon::Eraser,
                    has_item.then_some(Event::RemoveItem)
                ),
                space::horizontal().width(Fill),
                settings,
                icon_tool(
                    UNDO,
                    Icon::Undo,
                    (!w.undo.is_empty()).then_some(Event::Undo)
                ),
                icon_tool(
                    REDO,
                    Icon::Redo,
                    (!w.redo.is_empty()).then_some(Event::Redo)
                ),
            ]
            .spacing(4)
            .align_y(Center)
        };
        // Where the line does not fit the middle pane (a small window), Form ayarları by its icon, its name in its tip.
        let toolbar = fallback(line(true), line(false));

        // The items as a tree: dragged before, after or into a tab or group (docs/adr/0403 §2).
        fn node<'a>(
            item: &FormItem,
            next: &mut usize,
            w: &Window,
            fields: &[LayerField],
            issues: &[FormIssue],
        ) -> Node<'a, Message> {
            let (glyph, words, ruled) = item_face(item, fields);
            let errored = issues
                .iter()
                .any(|i| i.item.as_deref() == Some(item.id()) && i.severity == FormSeverity::Error);
            let mut marks = Row::new().spacing(4).align_y(Center);
            if ruled {
                marks = marks.push(label::caption("ƒ").style(muted));
            }
            if errored {
                marks = marks.push(icon(Icon::Warning).size(12.0).tone(Tone::Warning));
            }
            let mut n = Node::new(words)
                .icon(icon(glyph).size(13.0).tone(Tone::Muted))
                .id(*next)
                .cells([marks.into()])
                .selected(w.item.as_deref() == Some(item.id()))
                .on_press(msg(Event::SelectItem(item.id().to_owned())));
            *next += 1;
            if matches!(item, FormItem::Tab { .. } | FormItem::Group { .. }) {
                let open = !w.folded.contains(item.id());
                n = n
                    .folder()
                    .expanded(open, msg(Event::Fold(item.id().to_owned())));
                // A folded one's children keep their places in the order.
                for child in item.items() {
                    let made = node(child, next, w, fields, issues);
                    if open {
                        n = n.push(made);
                    }
                }
            }
            n
        }
        let mut next = 0;
        let nodes: Vec<Node<'_, Message>> = body
            .items
            .iter()
            .map(|i| node(i, &mut next, w, fields, issues))
            .collect();
        let tree: Element<'_, Message> = if body.items.is_empty() {
            container(label::caption(if fields.is_empty() {
                "Form boş: katmanda alan yok. Soldan bir alan oluşturun; forma da eklenir."
            } else {
                "Form boş. Soldaki alanlardan birine basın ya da Ekle ile sekme, grup ve not koyun."
            }))
            .padding(10)
            .into()
        } else {
            TreeView::new([
                tree_view::Column::new("Öğe").width(Fill),
                tree_view::Column::new("").width(36),
            ])
            .header(false)
            .extend(nodes)
            .on_move(|from, to, place| msg(Event::Drop(from, to, place)))
            .height(Fill)
            .into()
        };

        // The issues under their headings (docs/adr/9161): what keeps the form from a release kind by kind, then the
        // warnings; a press on one goes to its item.
        let line_of = |i: &FormIssue, kind: Kind| {
            let line = words::text_line(kind, i.message.clone());
            match &i.item {
                Some(id) => button(line)
                    .on_press(msg(Event::SelectItem(id.clone())))
                    .padding([1, 4])
                    .style(style::button::row(false))
                    .into(),
                None => line,
            }
        };
        let mut problems = Column::new().spacing(2);
        let mut shown = 0;
        for (kind, list) in kentos_forms::publish_blockers(issues) {
            problems = problems.push(label::strong(format!("{} ({})", kind.label(), list.len())));
            for i in list {
                if shown < 8 {
                    problems = problems.push(line_of(i, Kind::Warn));
                }
                shown += 1;
            }
        }
        let warnings: Vec<&FormIssue> = issues
            .iter()
            .filter(|i| !kentos_forms::blocks_publishing(i))
            .collect();
        if !warnings.is_empty() {
            problems = problems.push(label::strong(format!("Uyarılar ({})", warnings.len())));
            for i in warnings {
                if shown < 8 {
                    problems = problems.push(line_of(i, Kind::Info));
                }
                shown += 1;
            }
        }
        if shown > 8 {
            problems =
                problems.push(label::caption(format!("{} sorun daha.", shown - 8)).style(muted));
        }
        let mut col = column![
            toolbar,
            container(tree)
                .style(style::container::field_box)
                .width(Fill)
                .height(Fill),
        ]
        .spacing(8);
        if issues.is_empty() {
            col = col.push(words::text_line(Kind::Ok, "Ön denetim: sorun yok."));
        } else {
            col = col.push(container(scrollable(problems)).max_height(typography::scaled(110.0)));
        }
        col.into()
    }

    fn forms_footer(
        &self,
        form: Option<&ProjectForm>,
        issues: &[FormIssue],
        editable: bool,
    ) -> Element<'_, Message> {
        let Some(form) = form else {
            return space::horizontal().into();
        };
        // What keeps it from a release, kind by kind (docs/adr/9161): an empty form is one too.
        let blockers = kentos_forms::publish_blockers(issues);
        // A role that designs but does not publish (docs/adr/9174).
        let publish_locked = self
            .forms_publish_locked()
            .filter(|_| self.forms_locked().is_none());
        // Where the form stands, as sentences (DESIGN.md §13: no “A · B · C” line).
        let mut status = format!("Taslak {}. sürüm", form.draft.revision);
        match form.releases.last() {
            Some(r) => status.push_str(&format!(", son yayın {}.", r.version)),
            None => status.push_str(", yayınlanmadı."),
        }
        if form.withdrawn {
            status.push_str(" Yayından kaldırıldı.");
        }
        if !blockers.is_empty() {
            let kinds: Vec<String> = blockers
                .iter()
                .map(|(kind, list)| match (kind, list.len()) {
                    (kentos_forms::PublishKind::Empty, _) => "boş form".to_owned(),
                    (kind, n) => format!("{n} {}", kind.label().to_lowercase()),
                })
                .collect();
            status.push_str(&format!(" Yayınlanamaz: {}.", kinds.join(", ")));
        }
        if publish_locked.is_some() {
            status.push_str(" Rolünüz yayınlamaz.");
        }
        // Form ▾ (DESIGN.md §7.9: one secondary place on the left): the form's own work, each told, the delete last
        // and red. Six buttons side by side before, under a name, a scope and three switches.
        let withdrawn = form.withdrawn;
        let can_withdraw =
            editable && !form.releases.is_empty() && (form.withdrawn || !form.enforce);
        let face = container(
            row![
                label::body("Form"),
                icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted),
            ]
            .spacing(5)
            .align_y(Center),
        )
        .padding([4, 10])
        .style(style::container::field_box);
        let menu = MenuButton::new(face, move || {
            Menu::new()
                .item(AUDIT, Some(msg(Event::Audit)))
                .icon(Icon::Check)
                .detail("Katmanın kayıtlarını formun kurallarıyla denetler.")
                .item(RECOMPUTE, editable.then(|| msg(Event::Recompute)))
                .icon(Icon::Retry)
                .detail("Hesaplanan alanları bütün kayıtlarda yeniden hesaplar.")
                .separator()
                .item(IMPORT, Some(msg(Event::Import)))
                .icon(Icon::Import)
                .detail("Başka bir projeden dışa aktarılmış formu alır.")
                .item(EXPORT, editable.then(|| msg(Event::Export)))
                .icon(Icon::Export)
                .detail("Formu bir dosyaya yazar.")
                .separator()
                // Yayından kaldır / Yeniden yayına al (docs/adr/0445 §1).
                .item(
                    if withdrawn { PUT_BACK } else { WITHDRAW },
                    can_withdraw.then(|| msg(Event::Withdraw(!withdrawn))),
                )
                .detail(if withdrawn {
                    "Son yayın yeniden veri girişinde kullanılır."
                } else {
                    "Veri girişi bu formu kullanmaz; taslak ve yayınlar kalır."
                })
                .item(REMOVE_FORM, editable.then(|| msg(Event::RemoveForm)))
                .icon(Icon::Eraser)
                .danger()
        });
        row![
            menu,
            label::caption(status).style(muted),
            space::horizontal().width(Fill),
            words::secondary(CANCEL, Some(msg(Event::Cancel))),
            words::secondary(
                PUBLISH,
                (editable && blockers.is_empty() && publish_locked.is_none())
                    .then(|| msg(Event::Publish))
            ),
            words::primary(SAVE, editable.then(|| msg(Event::Save))),
        ]
        .spacing(10)
        .align_y(Center)
        .into()
    }
}

/// A requirement's words (docs/adr/0441 §4).
fn require_words(r: &str) -> String {
    match r {
        "moment" => "anın tarihi".into(),
        "user" => "kullanıcının adı".into(),
        "layers" => "başka katmanlar".into(),
        "geometry" => "kaydın şekli".into(),
        "attachments" => "ekler".into(),
        "workflow" => "iş akışı".into(),
        tool => format!("işlem aracı {}", tool.trim_start_matches("tool:")),
    }
}

fn empty_inspector<'a>() -> Element<'a, Message> {
    container(label::caption(
        "Bir öğe seçin: sekmenin, grubun, alanın ya da notun özellikleri burada düzenlenir.",
    ))
    .padding(12)
    .into()
}

fn muted(theme: &iced::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(Tokens::of(theme).muted),
    }
}

mod palettes;

mod inspector;

mod flow;
