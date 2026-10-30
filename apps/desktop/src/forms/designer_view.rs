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
use kentos_ui::widget::{Dialog as Frame, Elided, Panes, Segmented, fallback, segmented};
use kentos_ui::{label, style};

use kentos_ui::widget::tree_view::{self, Node, TreeView};

use super::designer::{
    ADD_ACTION, ADD_ATTACHMENTS, ADD_COMMENT, ADD_DIVIDER, ADD_GROUP, ADD_LOCATION, ADD_NOTE,
    ADD_REFERENCE, ADD_RELATED, ADD_TAB, AUDIT, CANCEL, CHANGES, COPY_ITEM, CREATE, DOMAINS_WINDOW,
    DOWN, DROP_KEPT, ENFORCE, EXPORT, Event, FLOW, IMPORT, LookupEdit, NAME, NEW_FORM,
    PALETTE_DOMAINS, PALETTE_FIELDS, PALETTE_RELATIONS, PALETTE_SECTIONS, PASTE_ITEM, PREVIEW,
    PROPERTIES, PUBLISH, PUBLISH_ANYWAY, PUT_BACK, Palette, RECOMPUTE, REDO, REMOVE_FORM,
    REMOVE_ITEM, RESTORE_DRAFT, Rule, SAVE, SAVE_SECTION, Said, TITLE, Text, UNDO, UP, WITHDRAW,
    Window, bound, find, msg,
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
        let Some(w) = &self.forms_window else {
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
            .map(|b| self.issues_of(b, &fields))
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
                (Some(body), Some(id)) => match find(&body.items, id) {
                    Some(item) => self.forms_inspector(item, &fields, &issues),
                    None => self.form_overview(body),
                },
                (Some(body), None) => self.form_overview(body),
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
        // 390 high (at 12 px text), less in a low window, the buttons under them in sight (docs/adr/0749).
        let sized = kentos_ui::widget::sizable::sizes::get(TITLE).is_some();
        let main = container(main).height(Fill).max_height(if sized {
            f32::INFINITY
        } else {
            typography::scaled(390.0)
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
        let title = match form {
            Some(f) if w.dirty => format!("{TITLE} · {} ●", f.name),
            Some(f) => format!("{TITLE} · {}", f.name),
            None => TITLE.to_owned(),
        };
        // The warnings, the panes taking what is left, the footer (docs/adr/0749).
        let mut frame = Frame::new(title);
        for part in above {
            frame = frame.push(part);
        }
        frame = frame.fill(main);
        if form.is_some() {
            frame = frame.push(self.forms_footer(w, form, &issues, locked.is_none()));
        }
        frame.key(TITLE).width(1040.0).modal(msg(Event::Cancel))
    }

    fn forms_list(
        &self,
        w: &Window,
        editable: bool,
        fields: &[LayerField],
    ) -> Element<'_, Message> {
        let mut list = Column::new().spacing(1).padding(2);
        for f in self.project_forms() {
            let chosen = w.chosen.as_deref() == Some(f.id.as_str());
            let mut caption = self.form_layer_name(&f.layer);
            if let Some(r) = f.releases.last() {
                caption.push_str(&format!(" · {}. yayın", r.version));
            } else {
                caption.push_str(" · taslak");
            }
            if f.enforce {
                caption.push_str(" · uygulanıyor");
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
        let mut col = Column::new()
            .spacing(8)
            .push(label::strong("Formlar"))
            .push(
                container(scrollable(list).height(typography::scaled(170.0)))
                    .style(style::container::field_box)
                    .width(Fill),
            );
        match &w.new_form {
            Some(n) => {
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
                let name = kentos_ui::widget::text_field("Formun adı (boşsa katmandan)", &n.name)
                    .on_input(|t| msg(Event::NewName(t)))
                    .on_submit(msg(Event::Create));
                col = col.push(name).push(pick).push(
                    row![
                        words::primary(CREATE, editable.then(|| msg(Event::Create))),
                        words::secondary(CANCEL, Some(msg(Event::CancelNew))),
                    ]
                    .spacing(8),
                );
            }
            None => {
                col = col.push(words::secondary(
                    NEW_FORM,
                    editable.then(|| msg(Event::NewForm)),
                ));
            }
        }
        if w.body.is_some() {
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
            if free.is_empty() {
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
        }
        col.into()
    }

    /// The layers a form can be made for, by their path.
    fn form_layers(&self) -> Vec<(String, String)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let mut out = Vec::new();
        fn walk(nodes: &[kentos_commands::LayerNode], at: &str, out: &mut Vec<(String, String)>) {
            for n in nodes {
                let path = if at.is_empty() {
                    n.name.clone()
                } else {
                    format!("{at} / {}", n.name)
                };
                if n.kind == LayerNodeType::Layer && n.service.is_none() {
                    out.push((n.id.clone(), path.clone()));
                }
                walk(&n.children, &path, out);
            }
        }
        walk(doc.model.layers().nodes(), "", &mut out);
        out
    }

    /// A palette row: a press adds what it names.
    fn palette_row<'a>(
        glyph: Icon,
        words: String,
        detail: String,
        press: Option<Message>,
    ) -> Element<'a, Message> {
        button(
            row![
                icon(glyph).size(12.0).tone(Tone::Muted),
                column![
                    Elided::new(words)
                        .size(typography::body())
                        .font(typography::ui())
                        .width(Fill),
                    label::caption(detail).style(muted),
                ]
                .spacing(1),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .on_press_maybe(press)
        .padding([3, 8])
        .width(Fill)
        .style(style::button::row(false))
        .into()
    }

    /// İlişkiler (docs/adr/0441 §1): the project's relations of the form's layer; a press adds a related
    /// item (the layer is the parent) or a reference (it is the child).
    fn relations_palette(&self, editable: bool) -> Element<'_, Message> {
        let layer = self
            .chosen_form()
            .map(|f| f.layer.clone())
            .unwrap_or_default();
        let relations = self
            .document
            .as_ref()
            .map(|d| d.model.settings().relations.clone())
            .unwrap_or_default();
        let mut list = Column::new().spacing(2);
        for r in relations
            .iter()
            .filter(|r| r.parent == layer || r.child == layer)
        {
            let (glyph, detail) = if r.parent == layer {
                (
                    Icon::Link,
                    format!("Alt kayıtlar: {}", self.form_layer_name(&r.child)),
                )
            } else {
                (
                    Icon::Target,
                    format!("Üst kayda başvuru: {}", self.form_layer_name(&r.parent)),
                )
            };
            list = list.push(Self::palette_row(
                glyph,
                r.name.clone(),
                detail,
                editable.then(|| msg(Event::AddRelationItem(r.id.clone()))),
            ));
        }
        if relations
            .iter()
            .all(|r| r.parent != layer && r.child != layer)
        {
            list = list.push(
                label::caption("Bu katmanın ilişkisi yok. İlişkiler penceresinden kurun.")
                    .style(muted),
            );
        }
        scrollable(list).height(Fill).into()
    }

    /// Değer listeleri (docs/adr/0441 §1): the project's lists, the layer's fields using each.
    fn domains_palette(&self, fields: &[LayerField]) -> Element<'_, Message> {
        let domains = self
            .document
            .as_ref()
            .map(|d| d.model.settings().domains.clone())
            .unwrap_or_default();
        let mut list = Column::new().spacing(2);
        for d in &domains {
            let users: Vec<String> = fields
                .iter()
                .filter(|f| f.domain.as_deref() == Some(d.id.as_str()))
                .map(|f| f.label().to_owned())
                .collect();
            let size = match &d.values {
                Some(v) => format!("{} değer", v.len()),
                None => "aralık".to_owned(),
            };
            let detail = if users.is_empty() {
                format!("{size} · bu katmanda kullanılmıyor")
            } else {
                format!("{size} · {}", users.join(", "))
            };
            list = list.push(Self::palette_row(
                Icon::ViewRows,
                d.name.clone(),
                detail,
                Some(msg(Event::OpenDomains)),
            ));
        }
        if domains.is_empty() {
            list = list.push(label::caption("Projede değer listesi yok.").style(muted));
        }
        column![
            scrollable(list).height(Fill),
            words::secondary(DOMAINS_WINDOW, Some(msg(Event::OpenDomains))),
        ]
        .spacing(6)
        .into()
    }

    /// Bölümler (docs/adr/0441 §1): the section library on this device, each version with how its
    /// fields bind to the form's layer.
    fn sections_palette<'a>(
        &'a self,
        editable: bool,
        fields: &[LayerField],
    ) -> Element<'a, Message> {
        let sections = self.form_files.sections();
        let mut list = Column::new().spacing(2);
        for (i, s) in sections.iter().enumerate().rev() {
            let binds = crate::forms::library::bindings(s, fields);
            let missing = binds.iter().filter(|(_, to)| to.is_none()).count();
            let detail = if binds.is_empty() {
                format!("{}. sürüm · alansız", s.version)
            } else if missing == 0 {
                format!("{}. sürüm · {} alan bağlanır", s.version, binds.len())
            } else {
                format!(
                    "{}. sürüm · {} alan, bu katmanda olmayan: {}",
                    s.version,
                    binds.len(),
                    missing
                )
            };
            list = list.push(Self::palette_row(
                Icon::Layout,
                s.name.clone(),
                detail,
                editable.then(|| msg(Event::InsertSection(i))),
            ));
        }
        if sections.is_empty() {
            list = list.push(
                label::caption("Kitaplık boş. Bir öğe seçip özelliklerinin altında Bölüm kitaplığına koy’a basın.")
                    .style(muted),
            );
        }
        scrollable(list).height(Fill).into()
    }

    /// The chosen item into the section library under the name typed (its own by default).
    fn section_save_row(&self) -> Element<'_, Message> {
        let name = self
            .forms_window
            .as_ref()
            .map_or("", |w| w.section_name.as_str());
        let input = kentos_ui::widget::text_field("Bölümün adı (boşsa öğenin)", name)
            .on_input(|t| msg(Event::SectionName(t)))
            .on_submit(msg(Event::SaveSection));
        column![
            label::strong("Bölüm kitaplığı"),
            row![
                input,
                words::secondary(SAVE_SECTION, Some(msg(Event::SaveSection))),
            ]
            .spacing(6)
            .align_y(Center),
        ]
        .spacing(6)
        .into()
    }

    /// The review comments on an item (else on the form), with one to add (docs/adr/0441 §5).
    /// Çeviriler (docs/adr/0443 §5): the form's languages as chips, a new one's tag added; the chosen
    /// one's words beside the form's own: its name, each item's label, a field's help.
    fn translations_box<'a>(&'a self, body: &FormBody) -> Element<'a, Message> {
        let Some(w) = &self.forms_window else {
            return column![].into();
        };
        let editable = self.forms_locked().is_none();
        let fields = self
            .chosen_form()
            .map(|f| self.layer_fields_of(&f.layer))
            .unwrap_or_default();
        let mut col = Column::new().spacing(8).push(label::strong("Çeviriler"));
        let mut chips = Row::new().spacing(4);
        for tag in body.translations.keys() {
            let on = w.language.as_deref() == Some(tag.as_str());
            chips = chips.push(
                row![
                    button(label::caption(crate::forms::entry::language_name(Some(
                        tag
                    ))))
                    .padding(kentos_ui::theme::metrics::action_small())
                    .style(if on {
                        style::button::primary
                    } else {
                        style::button::ghost
                    })
                    .on_press(msg(Event::LanguagePick(tag.clone()))),
                    button(icon(Icon::Close).size(10.0))
                        .padding(kentos_ui::theme::metrics::icon_small(10.0))
                        .style(style::button::ghost)
                        .on_press_maybe(editable.then(|| msg(Event::LanguageRemove(tag.clone())))),
                ]
                .spacing(0)
                .align_y(Center),
            );
        }
        let tag_ok = kentos_commands::language_tag_ok(w.new_language.trim())
            && !body.translations.contains_key(w.new_language.trim());
        let add = row![
            kentos_ui::widget::text_field("Dil etiketi (en, de, ar)", &w.new_language)
                .on_input(|t| msg(Event::LanguageText(t)))
                .on_submit_maybe((editable && tag_ok).then(|| msg(Event::LanguageAdd)))
                .width(150),
            words::ghost(
                "Dil ekle",
                (editable && tag_ok).then(|| msg(Event::LanguageAdd))
            ),
        ]
        .spacing(6)
        .align_y(Center);
        col = col.push(chips.wrap()).push(add);
        let Some(tag) = w
            .language
            .as_ref()
            .filter(|t| body.translations.contains_key(*t))
        else {
            return col
                .push(
                    label::caption(
                        "Formun sözleri başka dillerde: yalnız gösterilen değişir; değerler, kodlar ve kurallar formundur.",
                    )
                    .style(muted),
                )
                .into();
        };
        let words = &body.translations[tag];
        let line = |own: String, value: String, item: Option<String>, said: Said| {
            let field = kentos_ui::widget::text_field(&own, &value);
            let field = if editable {
                field.on_input(move |t| msg(Event::Translate(item.clone(), said.clone(), t)))
            } else {
                field
            };
            row![
                container(Elided::new(own).size(typography::caption())).width(130),
                field,
            ]
            .spacing(6)
            .align_y(Center)
        };
        let form_name = self
            .chosen_form()
            .map(|f| f.name.clone())
            .unwrap_or_default();
        col = col.push(line(
            form_name,
            words.name.clone().unwrap_or_default(),
            None,
            Said::Label,
        ));
        for (_, item) in body.walk() {
            let id = item.id().to_owned();
            let own = item_face(item, &fields).1;
            let said = words.items.get(&id);
            col = col.push(line(
                own,
                said.and_then(|s| s.label.clone()).unwrap_or_default(),
                Some(id.clone()),
                Said::Label,
            ));
            if let FormItem::Field(f) = item {
                if let Some(help) = &f.help {
                    col = col.push(line(
                        format!("Yardım: {help}"),
                        said.and_then(|s| s.help.clone()).unwrap_or_default(),
                        Some(id.clone()),
                        Said::Help,
                    ));
                }
                if let Some(hint) = &f.placeholder {
                    col = col.push(line(
                        format!("Boşken: {hint}"),
                        said.and_then(|s| s.placeholder.clone()).unwrap_or_default(),
                        Some(id.clone()),
                        Said::Placeholder,
                    ));
                }
                // The listed values' labels; the codes are the list's own.
                let values = fields
                    .iter()
                    .find(|l| l.key() == f.field)
                    .and_then(|l| l.values.clone())
                    .unwrap_or_default();
                for c in values {
                    col = col.push(line(
                        format!("{} · {}", c.code, c.label),
                        said.and_then(|s| s.choices.get(&c.code).cloned())
                            .unwrap_or_default(),
                        Some(id.clone()),
                        Said::Choice(c.code.clone()),
                    ));
                }
            }
        }
        col.into()
    }

    fn comments_box(&self, item: Option<&str>) -> Element<'_, Message> {
        let comments: Vec<kentos_commands::FormComment> = self
            .chosen_form()
            .map(|f| {
                f.comments
                    .iter()
                    .filter(|c| c.item.as_deref() == item)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let open = comments.iter().filter(|c| !c.resolved).count();
        let title = if comments.is_empty() {
            "Yorumlar".to_owned()
        } else {
            format!("Yorumlar · {open} açık")
        };
        let mut col = Column::new().spacing(6).push(label::strong(title));
        for c in comments {
            let who = [
                c.author.clone(),
                c.at.as_ref().map(|a| a.replacen('T', " ", 1)),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
            let mut text = column![label::body(c.text.clone())].spacing(2);
            if !who.is_empty() {
                text = text.push(label::caption(who).style(muted));
            }
            let toggle = kentos_ui::widget::tip(
                button(icon(if c.resolved { Icon::Undo } else { Icon::Check }).size(13.0))
                    .on_press(msg(Event::ResolveComment(c.id, !c.resolved)))
                    .padding(kentos_ui::theme::metrics::icon_small(13.0))
                    .style(style::button::tool(false)),
                if c.resolved {
                    "Yeniden aç"
                } else {
                    "Çözüldü"
                }
                .into(),
                iced::widget::tooltip::Position::Left,
            );
            col = col.push(
                container(row![text.width(Fill), toggle].spacing(6).align_y(Center))
                    .padding([6, 8])
                    .style(if c.resolved {
                        style::container::surface
                    } else {
                        style::container::field_box
                    })
                    .width(Fill),
            );
        }
        let typed = self
            .forms_window
            .as_ref()
            .map_or("", |w| w.comment_text.as_str());
        let input = kentos_ui::widget::text_field(
            if item.is_some() {
                "Bu öğeye yorum yazın"
            } else {
                "Forma yorum yazın"
            },
            typed,
        )
        .on_input(|t| msg(Event::CommentText(t)))
        .on_submit(msg(Event::AddComment));
        col.push(
            row![
                input,
                words::secondary(
                    ADD_COMMENT,
                    (!typed.trim().is_empty()).then(|| msg(Event::AddComment))
                ),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .into()
    }

    /// The form with no item chosen (docs/adr/0441 §4, §5): what it needs of its host and the form's
    /// own comments.
    fn form_overview<'a>(&'a self, body: &FormBody) -> Element<'a, Message> {
        let requires = kentos_forms::requirements(body);
        let words_of = |r: &str| -> String {
            match r {
                "moment" => "Anın tarihi ve saati".into(),
                "user" => "Kullanıcının adı".into(),
                "layers" => "Başka katmanlar".into(),
                "geometry" => "Kaydın şekli".into(),
                "attachments" => "Ekler".into(),
                "workflow" => "İş akışı".into(),
                tool => {
                    let id = tool.trim_start_matches("tool:");
                    match self.processing.registry.get(id) {
                        Some(t) => format!("İşlem aracı: {}", t.label),
                        None => format!("İşlem aracı: {id} (kayıtlı değil)"),
                    }
                }
            }
        };
        let mut col = Column::new()
            .spacing(10)
            .push(label::strong("Formun gerektirdikleri"));
        if requires.is_empty() {
            col = col.push(
                label::caption(
                    "Saf form: yalnız kaydın değerleriyle çalışır; her ortamda aynı açılır.",
                )
                .style(muted),
            );
        } else {
            let mut chips = Row::new().spacing(4);
            for r in &requires {
                chips = chips.push(
                    container(label::caption(words_of(r)))
                        .padding([2, 8])
                        .style(style::container::token),
                );
            }
            col = col.push(chips.wrap()).push(
                label::caption(
                    "Bunları sağlamayan bir ortam formu salt okunur açar ve hangisinin eksik olduğunu söyler. Masaüstü hepsini sağlar.",
                )
                .style(muted),
            );
        }
        col = col.push(self.translations_box(body));
        col = col.push(self.comments_box(None));
        scrollable(col.padding(iced::Padding {
            top: 0.0,
            right: 14.0,
            bottom: 0.0,
            left: 4.0,
        }))
        .height(Fill)
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
        let tool = |words: &'static str, glyph: Icon, event: Option<Event>| {
            button(
                row![icon(glyph).size(13.0), label::body(words)]
                    .spacing(5)
                    .align_y(Center),
            )
            .on_press_maybe(event.map(msg))
            .padding([4, 8])
            .style(style::button::tool(false))
        };
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
        // The components (docs/adr/0441 §1): the common three named, the rest by their icons; where the
        // three named do not fit, they too by their icons, their names in their tips (docs/adr/0749).
        let adds = |named: bool| {
            let first = |words: &'static str, glyph: Icon, event: Event| -> Element<'_, Message> {
                if named {
                    tool(words, glyph, Some(event)).into()
                } else {
                    icon_tool(words, glyph, Some(event))
                }
            };
            row![
                first(ADD_TAB, Icon::Tabs, Event::AddTab),
                first(ADD_GROUP, Icon::Layout, Event::AddGroup),
                first(ADD_NOTE, Icon::Info, Event::AddNote),
                icon_tool(ADD_DIVIDER, Icon::Minus, Some(Event::AddDivider)),
                icon_tool(ADD_RELATED, Icon::Link, Some(Event::AddRelated)),
                icon_tool(ADD_REFERENCE, Icon::Target, Some(Event::AddReference)),
                icon_tool(
                    ADD_ATTACHMENTS,
                    crate::icons::from_web(Some("photo")),
                    Some(Event::AddAttachments)
                ),
                icon_tool(
                    ADD_LOCATION,
                    crate::icons::from_web(Some("pin")),
                    Some(Event::AddLocation)
                ),
                icon_tool(ADD_ACTION, Icon::Play, Some(Event::AddAction)),
            ]
            .spacing(4)
            .align_y(Center)
        };
        let toolbar = fallback(adds(true), adds(false));
        // The chosen item's tools on a line of their own, the form's undo and redo at its end: the add tools
        // fill the first (docs/adr/0437, 0749).
        let item_tools = row![
            icon_tool(UP, Icon::ChevronUp, has_item.then_some(Event::Move(-1))),
            icon_tool(DOWN, Icon::ChevronDown, has_item.then_some(Event::Move(1))),
            icon_tool(
                REMOVE_ITEM,
                Icon::Eraser,
                has_item.then_some(Event::RemoveItem)
            ),
            icon_tool(COPY_ITEM, Icon::Copy, has_item.then_some(Event::CopyItem)),
            icon_tool(
                PASTE_ITEM,
                crate::icons::from_web(Some("paste")),
                self.form_clipboard.is_some().then_some(Event::PasteItem)
            ),
            space::horizontal().width(Fill),
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
        .align_y(Center);
        let toolbar = column![toolbar, item_tools].spacing(4);

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
            container(label::caption(
                "Form boş. Soldaki alanlardan birine basın ya da sekme, grup ve not ekleyin.",
            ))
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

        let mut problems = Column::new().spacing(2);
        for i in issues.iter().take(8) {
            let kind = if i.severity == FormSeverity::Error {
                Kind::Warn
            } else {
                Kind::Info
            };
            let line = words::text_line(kind, i.message.clone());
            problems = problems.push(match &i.item {
                Some(id) => button(line)
                    .on_press(msg(Event::SelectItem(id.clone())))
                    .padding([1, 4])
                    .style(style::button::row(false))
                    .into(),
                None => line,
            });
        }
        if issues.len() > 8 {
            problems = problems
                .push(label::caption(format!("{} sorun daha.", issues.len() - 8)).style(muted));
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

    fn forms_inspector(
        &self,
        item: &FormItem,
        fields: &[LayerField],
        issues: &[FormIssue],
    ) -> Element<'_, Message> {
        let id = item.id().to_owned();
        let error_of = |rule: &str| {
            issues
                .iter()
                .find(|i| i.item.as_deref() == Some(id.as_str()) && i.rule.as_deref() == Some(rule))
                .map(|i| i.message.clone())
        };
        let input = |placeholder: &str, value: &str, which: Text| {
            let which = which.clone();
            kentos_ui::widget::text_field(placeholder, value)
                .on_input(move |t| msg(Event::Text(which.clone(), t)))
        };
        let rule_in = |words: &'static str,
                       rule: Rule,
                       value: &str,
                       placeholder: &'static str|
         -> Element<'_, Message> {
            let r = rule.clone();
            let field = kentos_ui::widget::text_field(placeholder, value)
                .on_input(move |t| msg(Event::RuleText(r.clone(), t)))
                .font(typography::mono());
            let builder = button(label::body("ε"))
                .on_press(msg(Event::Builder(rule.clone())))
                .padding([4, 8])
                .style(style::button::tool(false));
            words::field(
                words,
                row![field, builder].spacing(4).align_y(Center),
                error_of(&rule.path()),
            )
        };
        let rule_row = |words: &'static str, rule: Rule, value: &str| {
            rule_in(words, rule, value, "Kural yok: her zaman")
        };
        let mut col = Column::new().spacing(10);
        match item {
            FormItem::Tab {
                label: text,
                visible_when,
                ..
            } => {
                col = col
                    .push(label::strong("Sekme"))
                    .push(words::field(
                        "Ad",
                        input("Sekmenin adı", text, Text::Label),
                        None,
                    ))
                    .push(rule_row(
                        "Görünürlük",
                        Rule::Visible(id.clone()),
                        visible_when.as_deref().unwrap_or(""),
                    ));
            }
            FormItem::Group {
                label: text,
                columns,
                visible_when,
                collapsed,
                across,
                ..
            } => {
                let cols = columns.unwrap_or(1);
                col = col
                    .push(label::strong("Grup"))
                    .push(words::field(
                        "Başlık",
                        input("Başlıksız", text.as_deref().unwrap_or(""), Text::Label),
                        None,
                    ))
                    .push(words::field(
                        "Sütun",
                        Segmented::new([1u8, 2u8], cols, |n| msg(Event::Columns(n))),
                        None,
                    ))
                    .push(rule_row(
                        "Görünürlük",
                        Rule::Visible(id.clone()),
                        visible_when.as_deref().unwrap_or(""),
                    ))
                    .push(words::check(
                        *collapsed,
                        "Kapalı açılır",
                        Some(msg(Event::Collapsed(!*collapsed))),
                    ));
                if cols == 2 {
                    // Satır satır (docs/adr/0443 §6): Tab goes across, then down.
                    col = col.push(kentos_ui::widget::tip(
                        words::check(*across, "Satır satır", Some(msg(Event::Across(!*across)))),
                        kentos_ui::widget::Tip::new("Satır satır").body(
                            "Öğeler soldan sağa, satır satır dizilir; Tab önce yana, sonra aşağı gider",
                        ),
                        iced::widget::tooltip::Position::Top,
                    ));
                }
            }
            FormItem::Related {
                label: text,
                layer,
                field,
                visible_when,
                view,
                columns,
                min,
                max,
                ..
            } => {
                let layers = self.form_layers();
                let at = layers.iter().position(|(id, _)| id == layer);
                let picks = layers.clone();
                let pick_layer = Select::new(
                    layers.iter().map(|(_, name)| Choice::new(name.clone())),
                    at,
                    move |i| msg(Event::RelatedLayer(picks[i].0.clone())),
                )
                .placeholder("Katman");
                let their = self.layer_fields_of(layer);
                let at_field = their.iter().position(|f| f.key() == field);
                let keys: Vec<String> = their.iter().map(|f| f.key().to_owned()).collect();
                let pick_field = Select::new(
                    their.iter().map(|f| Choice::new(f.label().to_owned())),
                    at_field,
                    move |i| msg(Event::RelatedField(keys[i].clone())),
                )
                .placeholder("Bu kaydın kimliğini tutan alan");
                col = col
                    .push(label::strong("İlişkili kayıtlar"))
                    .push(words::field(
                        "Başlık",
                        input("İlişkili kayıtlar", text.as_deref().unwrap_or(""), Text::Label),
                        None,
                    ))
                    .push(self.relation_pick(layer, field))
                    .push(words::field("Katman", pick_layer, error_of("related")))
                    .push(words::field("Alan", pick_field, None))
                    .push(
                        label::caption(
                            "Seçilen alanında bu kaydın kalıcı kimliğini tutan nesneler listelenir; her biri kendi formuyla açılır.",
                        )
                        .style(muted),
                    );
                // Liste | Tablo, and the table's columns (docs/adr/0425).
                use kentos_commands::RelatedView as RV;
                let views = [RV::List, RV::Table, RV::Cards];
                col = col.push(words::field(
                    "Görünüm",
                    Select::new(
                        ["Liste", "Tablo", "Kartlar"].map(Choice::new),
                        views.iter().position(|v| v == view),
                        move |i| msg(Event::RelatedView(views[i])),
                    ),
                    None,
                ));
                if *view != RV::List {
                    let mut chips = Row::new().spacing(4);
                    for f in their.iter().filter(|f| f.key() != field) {
                        let on = columns.iter().any(|c| c == f.key());
                        chips = chips.push(
                            button(label::caption(f.label().to_owned()))
                                .on_press(msg(Event::RelatedColumn(f.key().to_owned())))
                                .padding([2, 8])
                                .style(style::button::chip(on)),
                        );
                    }
                    col = col.push(words::field(
                        "Sütunlar",
                        chips.wrap(),
                        Some(
                            "Seçilmezse katmanın ilk dört alanı; ondalık sayı sütunlarının toplamı tablonun altında."
                                .into(),
                        ),
                    ));
                }
                // How many related objects the record holds (docs/adr/0444 §1): 1 one to one, at least 1 a child it
                // must have.
                let count = |n: &Option<u32>| n.map(|n| n.to_string()).unwrap_or_default();
                col = col
                    .push(
                        row![
                            container(words::field(
                                "En az",
                                input("Sınırsız", &count(min), Text::AttachMin),
                                None,
                            ))
                            .width(Fill),
                            container(words::field(
                                "En çok",
                                input("Sınırsız", &count(max), Text::AttachMax),
                                None,
                            ))
                            .width(Fill),
                        ]
                        .spacing(12),
                    )
                    .push(
                        label::caption(
                            "En çok 1 bire bir, en az 1 zorunlu alt kayıttır; kayıt formla yazılırken, gönderimin alt kayıtlarıyla sayılır.",
                        )
                        .style(muted),
                    );
                col = col.push(rule_row(
                    "Görünürlük",
                    Rule::Visible(id.clone()),
                    visible_when.as_deref().unwrap_or(""),
                ));
            }
            FormItem::Location {
                label: text,
                visible_when,
                ..
            } => {
                col = col
                    .push(label::strong("Konum"))
                    .push(words::field(
                        "Başlık",
                        input("Konum", text.as_deref().unwrap_or(""), Text::Label),
                        None,
                    ))
                    .push(rule_row(
                        "Görünürlük",
                        Rule::Visible(id.clone()),
                        visible_when.as_deref().unwrap_or(""),
                    ))
                    .push(
                        label::caption(
                            "Kaydın şekli küçük bir haritada, kuzey yukarı; merkezinin koordinatları ve alanı ya da uzunluğuyla. Hiçbir şey yazmaz.",
                        )
                        .style(muted),
                    );
            }
            FormItem::Divider {
                label: text,
                visible_when,
                ..
            } => {
                col = col
                    .push(label::strong("Bölücü"))
                    .push(words::field(
                        "Başlık",
                        input(
                            "Başlıksız çizgi",
                            text.as_deref().unwrap_or(""),
                            Text::Label,
                        ),
                        None,
                    ))
                    .push(rule_row(
                        "Görünürlük",
                        Rule::Visible(id.clone()),
                        visible_when.as_deref().unwrap_or(""),
                    ))
                    .push(
                        label::caption(
                            "Formu bölen bir çizgi; başlığı üstünde. Hiçbir şey yazmaz.",
                        )
                        .style(muted),
                    );
            }
            FormItem::Action {
                label: text,
                kind,
                tool,
                step,
                runs,
                visible_when,
                ..
            } => {
                use kentos_commands::{ActionKind as K, ActionRuns as R};
                let kinds = [K::Tool, K::Recompute, K::Step, K::Show];
                let kind_pick = Select::new(
                    [
                        "İşlem aracı",
                        "Hesapları yenile",
                        "İş akışı adımı",
                        "Çizimde göster",
                    ]
                    .map(Choice::new),
                    kinds.iter().position(|k| k == kind),
                    move |i| msg(Event::ActionKind(kinds[i])),
                );
                col = col
                    .push(label::strong("Eylem"))
                    .push(words::field(
                        "Düğme",
                        input("Düğmenin adı", text, Text::Label),
                        None,
                    ))
                    .push(words::field("Tür", kind_pick, None));
                match kind {
                    // Only a registered tool is offered (docs/adr/0441 §6): no address, query or script.
                    K::Tool => {
                        let tools: Vec<(String, String)> = self
                            .processing
                            .registry
                            .tools()
                            .iter()
                            .map(|t| (t.id.clone(), t.label.clone()))
                            .collect();
                        let current = tool.clone().unwrap_or_default();
                        let at = tools.iter().position(|(i, _)| *i == current);
                        let ids: Vec<String> = tools.iter().map(|(i, _)| i.clone()).collect();
                        let pick = Select::new(
                            tools
                                .iter()
                                .map(|(i, l)| Choice::new(l.clone()).detail(i.clone())),
                            at,
                            move |i| msg(Event::ActionTool(ids[i].clone())),
                        )
                        .placeholder("İşlem aracı");
                        let missing = at.is_none().then(|| {
                            format!("“{current}” kayıtlı bir işlem aracı değil; listeden seçin.")
                        });
                        let places = [R::Auto, R::Here, R::Background];
                        let runs_pick = Select::new(
                            ["Otomatik", "Bu bilgisayarda", "Arka planda"].map(Choice::new),
                            places.iter().position(|r| r == runs),
                            move |i| msg(Event::ActionRuns(places[i])),
                        );
                        col = col
                            .push(words::field("Araç", pick, missing))
                            .push(words::field("Nerede çalışır", runs_pick, None));
                    }
                    K::Step => {
                        let steps: Vec<String> = self
                            .forms_window
                            .as_ref()
                            .and_then(|w| w.body.as_ref())
                            .and_then(|b| b.workflow.as_ref())
                            .map(|f| f.steps.iter().map(|s| s.label.clone()).collect())
                            .unwrap_or_default();
                        let at = step
                            .as_ref()
                            .and_then(|s| steps.iter().position(|x| x == s));
                        let names = steps.clone();
                        let pick = Select::new(
                            steps.iter().map(|s| Choice::new(s.clone())),
                            at,
                            move |i| msg(Event::ActionStep(names[i].clone())),
                        )
                        .placeholder("Adım");
                        let missing = (at.is_none()).then(|| {
                            if steps.is_empty() {
                                "Formun iş akışı yok; önce İş akışı'ndan adımlar ekleyin."
                                    .to_owned()
                            } else {
                                "İş akışının bir adımını seçin.".to_owned()
                            }
                        });
                        col = col.push(words::field("Adım", pick, missing));
                    }
                    K::Recompute | K::Show => {}
                }
                let says = match kind {
                    K::Tool => {
                        "Kayıt seçili olarak İşlemler'in aracını açar; araç kendi kurallarıyla yazar. Yalnız kayıtlı araçlar seçilir."
                    }
                    K::Recompute => {
                        "Formun hesaplarını kayıtta yeniden çalıştırır (Hesapları yenile); değiştirilebilir hesaplara dokunmaz."
                    }
                    K::Step => {
                        "İş akışının adımını kayıtta atar; adımın kuralları ve gerekçesi geçerlidir."
                    }
                    K::Show => "Kaydı çizimde gösterir ve seçer; hiçbir şey yazmaz.",
                };
                col = col
                    .push(rule_row(
                        "Görünürlük",
                        Rule::Visible(id.clone()),
                        visible_when.as_deref().unwrap_or(""),
                    ))
                    .push(label::caption(says).style(muted));
            }
            FormItem::Attachments {
                label: text,
                accept,
                min,
                max,
                visible_when,
                ..
            } => {
                use kentos_commands::AttachmentAccept as A;
                let kinds = [A::Any, A::Photo, A::Document];
                let accepts = Select::new(
                    ["Hepsi", "Fotoğraflar", "Belgeler ve adresler"].map(Choice::new),
                    kinds.iter().position(|k| k == accept),
                    move |i| msg(Event::Accept(kinds[i])),
                );
                let count = |n: &Option<u32>| n.map(|n| n.to_string()).unwrap_or_default();
                col = col
                    .push(label::strong("Ekler"))
                    .push(words::field(
                        "Başlık",
                        input("Ekler", text.as_deref().unwrap_or(""), Text::Label),
                        None,
                    ))
                    .push(words::field("Ekler", accepts, None))
                    .push(
                        row![
                            container(words::field(
                                "En az",
                                input("Sınırsız", &count(min), Text::AttachMin),
                                None,
                            ))
                            .width(Fill),
                            container(words::field(
                                "En çok",
                                input("Sınırsız", &count(max), Text::AttachMax),
                                None,
                            ))
                            .width(Fill),
                        ]
                        .spacing(12),
                    )
                    .push(
                        label::caption(
                            "Kaydın fotoğrafları ve belgeleri formda eklenir, açılır ve kaldırılır; sayı kayıt formla yazılırken ve Kayıtları denetle'de denetlenir.",
                        )
                        .style(muted),
                    )
                    .push(rule_row(
                        "Görünürlük",
                        Rule::Visible(id.clone()),
                        visible_when.as_deref().unwrap_or(""),
                    ));
            }
            FormItem::Note { text, .. } => {
                col = col
                    .push(label::strong("Not"))
                    .push(words::field(
                        "Metin",
                        input("Notun metni", text, Text::Note),
                        error_of("text"),
                    ))
                    .push(
                        label::caption("{Kat} ya da {Kat * 3} kaydın değeriyle yazılır.")
                            .style(muted),
                    );
            }
            FormItem::Field(f) => {
                let layer = fields.iter().find(|l| l.key() == f.field);
                let title = match layer {
                    Some(l) => format!("{} · {}", l.label(), l.kind.label()),
                    None => format!("{} · katmanda yok", f.field),
                };
                let fits: Vec<FormWidget> = layer.map_or_else(Vec::new, |l| {
                    FormWidget::ALL
                        .into_iter()
                        .filter(|w| kentos_forms::widget_fits(*w, l).is_ok())
                        .collect()
                });
                // Each with its widget's icon, the field's kind's first (docs/adr/0751).
                let own = layer.map(|l| {
                    let by_kind = kentos_commands::FormField {
                        widget: None,
                        ..f.clone()
                    };
                    widget_glyph(kentos_forms::widget_of(&by_kind, l), l.kind)
                });
                let mut choices = vec![match own {
                    Some(glyph) => Choice::new("Alanın türüne göre").icon(glyph),
                    None => Choice::new("Alanın türüne göre"),
                }];
                choices.extend(fits.iter().map(|w| {
                    let kind = layer.map_or(kentos_commands::LayerFieldKind::Text, |l| l.kind);
                    Choice::new(w.label()).icon(widget_glyph(*w, kind))
                }));
                let at = match f.widget {
                    None => Some(0),
                    Some(w) => fits.iter().position(|x| *x == w).map(|i| i + 1),
                };
                let picks = fits.clone();
                // A widget the field's kind does not take shows as such, until another is chosen.
                let unfit = match (at, f.widget) {
                    (None, Some(w)) => format!("{} (alana uymuyor)", w.label()),
                    _ => "Seçin".to_owned(),
                };
                let widget = Select::new(choices, at, move |i| {
                    msg(Event::Widget(if i == 0 {
                        None
                    } else {
                        picks.get(i - 1).copied()
                    }))
                })
                .placeholder(unfit);
                let layer_required = layer.is_some_and(|l| l.required);
                col = col
                    .push(label::strong(title))
                    .push(self.field_links(f, fields))
                    .push(words::field(
                        "Etiket",
                        input(
                            layer.map_or("", |l| l.label()),
                            f.label.as_deref().unwrap_or(""),
                            Text::Label,
                        ),
                        None,
                    ))
                    .push(words::field("Öğe türü", widget, None));
                // A mirror has only its label, help, widget and visibility (docs/adr/0437).
                if f.mirror {
                    return col
                        .push(label::caption(
                            "Yansıma: alanın değeri burada salt okunur görünür; alanın kendi öğesi yazar.",
                        ).style(muted))
                        .push(words::field(
                            "Yardım",
                            input("Alanın altında", f.help.as_deref().unwrap_or(""), Text::Help),
                            None,
                        ))
                        .push(rule_row(
                            "Görünürlük",
                            Rule::Visible(id.clone()),
                            f.visible_when.as_deref().unwrap_or(""),
                        ))
                        .padding(iced::Padding {
                            top: 0.0,
                            right: 16.0,
                            bottom: 0.0,
                            left: 4.0,
                        })
                        .into();
                }
                col = col
                    .push(words::secondary(
                        "Yansımasını ekle",
                        Some(msg(Event::AddMirror)),
                    ))
                    .push(self.lookup_rows(&id, f, error_of("lookup")));
                // A searched list starts after a few letters (docs/adr/0439 §6).
                if f.widget == Some(FormWidget::Search) {
                    col = col.push(words::field(
                        "Aramaya başlama",
                        row![
                            kentos_ui::widget::text_field(
                                "1",
                                &f.min_search.map(|n| n.to_string()).unwrap_or_default()
                            )
                            .on_input(|t| msg(Event::MinSearch(t)))
                            .width(70),
                            label::caption("harften sonra").style(muted),
                        ]
                        .spacing(6)
                        .align_y(Center),
                        None,
                    ));
                }
                col = col
                    .push(
                        row![
                            words::check(
                                f.required || layer_required,
                                "Zorunlu",
                                (!layer_required).then(|| msg(Event::Required(!f.required))),
                            ),
                            words::check(
                                f.read_only,
                                "Salt okunur",
                                Some(msg(Event::ReadOnly(!f.read_only)))
                            ),
                        ]
                        .spacing(16),
                    )
                    .push(
                        row![
                            words::check(f.unique, "Tekil", Some(msg(Event::Unique(!f.unique)))),
                            words::check(
                                f.suggest,
                                "Öneri",
                                (layer.is_some_and(|l| {
                                    l.kind == kentos_commands::LayerFieldKind::Text
                                        && l.values.as_ref().is_none_or(|v| v.is_empty())
                                }) && f.lookup.is_none()
                                    || f.suggest)
                                    .then(|| msg(Event::Suggest(!f.suggest))),
                            ),
                            words::check(
                                f.reuse_last,
                                "Son değeri kullan",
                                (f.calculate.is_none() || f.reuse_last)
                                    .then(|| msg(Event::ReuseLast(!f.reuse_last))),
                            ),
                        ]
                        .spacing(16),
                    )
                    // Kept out of the recorder's examples (docs/adr/0443 §2).
                    .push(kentos_ui::widget::tip(
                        words::check(f.redact, "Örneklerde gizle", Some(msg(Event::Redact(!f.redact)))),
                        kentos_ui::widget::Tip::new("Örneklerde gizle").body(
                            "Komut geçmişinin Python'unda ve tekrar oynatma girdisinde değeri ‹gizli› yazılır",
                        ),
                        iced::widget::tooltip::Position::Top,
                    ))
                    .push(words::field(
                        "Yardım",
                        input(
                            "Alanın altında",
                            f.help.as_deref().unwrap_or(""),
                            Text::Help,
                        ),
                        None,
                    ))
                    .push(words::field(
                        "Yer tutucu",
                        input(
                            "Boşken",
                            f.placeholder.as_deref().unwrap_or(""),
                            Text::Placeholder,
                        ),
                        None,
                    ))
                    .push(words::field(
                        "Varsayılan",
                        input(
                            "Yeni kayıtta",
                            f.default.as_deref().unwrap_or(""),
                            Text::Default,
                        ),
                        error_of("default"),
                    ))
                    .push(rule_in(
                        "Varsayılan ifade",
                        Rule::Default(id.clone()),
                        f.default_expr.as_deref().unwrap_or(""),
                        "Yok: @kullanici, @tarih, $alan …",
                    ))
                    .push(rule_in(
                        "Hesap",
                        Rule::Calculate(id.clone()),
                        f.calculate.as_deref().unwrap_or(""),
                        "Yok: değeri elle yazılır",
                    ))
                    .push(words::check(
                        f.overridable,
                        "Değiştirilebilir: hesap boş alanı doldurur, yazılan değer kalır",
                        (f.calculate.is_some() || f.overridable)
                            .then(|| msg(Event::Overridable(!f.overridable))),
                    ))
                    .push(rule_row(
                        "Görünürlük",
                        Rule::Visible(id.clone()),
                        f.visible_when.as_deref().unwrap_or(""),
                    ))
                    .push(rule_row(
                        "Zorunluluk",
                        Rule::Required(id.clone()),
                        f.required_when.as_deref().unwrap_or(""),
                    ))
                    .push(rule_row(
                        "Salt okunurluk",
                        Rule::ReadOnly(id.clone()),
                        f.read_only_when.as_deref().unwrap_or(""),
                    ));
                if layer.is_some_and(|l| l.values.as_ref().is_some_and(|v| !v.is_empty())) {
                    col = col.push(rule_in(
                        "Seçenek süzgeci",
                        Rule::ChoiceFilter(id.clone()),
                        f.choice_filter.as_deref().unwrap_or(""),
                        "Yok: bütün değerler (@secenek her biri)",
                    ));
                }
                col = col.push(label::strong("Denetimler"));
                for (i, c) in f.checks.iter().enumerate() {
                    // When it is asked (docs/adr/0438 §2, docs/adr/0760 §1).
                    use kentos_commands::CheckMoment;
                    let moments = [
                        None,
                        Some(CheckMoment::Leave),
                        Some(CheckMoment::Save),
                        Some(CheckMoment::Submit),
                    ];
                    let at = moments.iter().position(|m| *m == c.on);
                    let when = Select::new(
                        [
                            Choice::new("Değer değişince"),
                            Choice::new("Alandan çıkınca"),
                            Choice::new("Kaydederken"),
                            Choice::new("İş akışında ilerlerken"),
                        ],
                        at,
                        move |k| msg(Event::CheckMoment(i, moments[k])),
                    );
                    col = col
                        .push(rule_row("Kural", Rule::Check(id.clone(), i), &c.rule))
                        .push(words::field("Ne zaman", when, None))
                        .push(
                            row![
                                container(input(
                                    "Tutmazsa söylenen",
                                    &c.message,
                                    Text::CheckMessage(i)
                                ))
                                .width(Fill),
                                words::check(
                                    c.warning,
                                    "Uyarı",
                                    Some(msg(Event::CheckWarning(i, !c.warning)))
                                ),
                                button(icon(Icon::Close).size(12.0))
                                    .on_press(msg(Event::RemoveCheck(i)))
                                    .padding(kentos_ui::theme::metrics::icon(12.0))
                                    .style(style::button::tool(false)),
                            ]
                            .spacing(4)
                            .align_y(Center),
                        );
                }
                col = col.push(words::secondary("Denetim ekle", Some(msg(Event::AddCheck))));
            }
        }
        // Alt türlerde göster (docs/adr/0426): the visibility rule by the layer's subtypes.
        if !matches!(item, FormItem::Note { .. })
            && let Some(row) = self.kinds_row(item)
        {
            col = col.push(row);
        }
        // The item into the section library and its review comments (docs/adr/0441 §1, §5).
        col = col
            .push(self.section_save_row())
            .push(self.comments_box(Some(&id)));
        scrollable(col.padding(iced::Padding {
            top: 0.0,
            right: 16.0,
            bottom: 0.0,
            left: 4.0,
        }))
        .id(PROPERTIES_SCROLL)
        .height(Fill)
        .into()
    }

    /// Alt türlerde göster (docs/adr/0426): a chip for each of the layer's subtypes; the chosen ones are
    /// the item's visibility rule. A rule written otherwise leaves the chips off.
    fn kinds_row<'a>(&self, item: &FormItem) -> Option<Element<'a, Message>> {
        let (sorter, codes) = self.forms_sorter()?;
        let form = self.chosen_form()?;
        let node = self.document.as_ref()?.model.layers().get(&form.layer)?;
        let subtypes = node.subtypes.as_ref()?;
        let rule = match item {
            FormItem::Tab { visible_when, .. }
            | FormItem::Group { visible_when, .. }
            | FormItem::Related { visible_when, .. }
            | FormItem::Attachments { visible_when, .. }
            | FormItem::Location { visible_when, .. }
            | FormItem::Divider { visible_when, .. }
            | FormItem::Action { visible_when, .. } => visible_when.clone(),
            FormItem::Field(f) => f.visible_when.clone(),
            FormItem::Note { .. } => None,
        };
        let now = crate::forms::designer::kinds_of_rule(rule.as_deref(), &sorter);
        let mut chips = Row::new().spacing(4);
        for code in &codes {
            let words = subtypes
                .get(code)
                .map_or(code.as_str(), |t| subtypes.label_of(&node.fields, t))
                .to_owned();
            let on = now.as_ref().is_some_and(|k| k.contains(code));
            chips = chips.push(
                button(label::caption(words))
                    .on_press_maybe(now.is_some().then(|| msg(Event::ShowInKind(code.clone()))))
                    .padding([2, 8])
                    .style(style::button::chip(on)),
            );
        }
        let note = if now.is_some() {
            "Seçilmezse her türde görünür; seçilenler görünürlük kuralını yazar."
        } else {
            "Görünürlük kendi kuralıyla yazılmış; alt türleri seçmek için kuralı silin."
        };
        Some(words::field(
            "Alt türlerde göster",
            chips.wrap(),
            Some(note.into()),
        ))
    }

    /// Bağlar (docs/adr/0414 §4, plan FORM-DESIGN-09): the fields a field item's rules read, and the
    /// rules, notes and layer settings that read the field, with this form as the designer has it.
    fn field_links<'a>(
        &self,
        f: &kentos_commands::FormField,
        fields: &[LayerField],
    ) -> Element<'a, Message> {
        use kentos_expression::editor::fields_of;
        let Some(field) = fields.iter().find(|x| x.key() == f.field) else {
            return space().into();
        };
        let rules = [
            &f.visible_when,
            &f.required_when,
            &f.read_only_when,
            &f.default_expr,
            &f.calculate,
            &f.choice_filter,
        ];
        let mut reads: Vec<String> = Vec::new();
        let sources = rules
            .iter()
            .filter_map(|r| r.as_deref())
            .chain(f.checks.iter().map(|c| c.rule.as_str()));
        for src in sources {
            for name in fields_of(src) {
                if name != field.name
                    && fields.iter().any(|x| x.name == name)
                    && !reads.contains(&name)
                {
                    reads.push(name);
                }
            }
        }
        let mut readers = Vec::new();
        if let (Some(doc), Some(w), Some(form)) =
            (&self.document, &self.forms_window, self.chosen_form())
        {
            let mut settings = doc.model.settings().clone();
            if let (Some(body), Some(at)) =
                (&w.body, settings.forms.iter_mut().find(|x| x.id == form.id))
            {
                at.draft = body.clone();
            }
            readers = kentos_forms::field_refs::uses(
                &settings,
                doc.model.layers().nodes(),
                &form.layer,
                field,
            )
            .into_iter()
            // What the drafts and the layer read; a published release keeps its own.
            .filter(|u| u.how != kentos_commands::FieldUseHow::Key && !u.kept)
            .collect();
        }
        let mut col = Column::new()
            .spacing(4)
            .push(label::caption("Bağlar").style(muted))
            .push(label::caption(if reads.is_empty() {
                "Kuralları başka alan okumuyor.".to_owned()
            } else {
                format!("Kuralları okuyor: {}", reads.join(", "))
            }));
        if readers.is_empty() {
            col = col.push(label::caption(format!(
                "“{}” alanını okuyan kural yok.",
                field.label()
            )));
        } else {
            col = col.push(label::caption(format!(
                "“{}” alanını okuyanlar ({}):",
                field.label(),
                readers.len()
            )));
            for u in readers {
                col = col.push(label::caption(u.place).style(muted));
            }
        }
        col.into()
    }

    fn forms_footer(
        &self,
        w: &Window,
        form: Option<&ProjectForm>,
        issues: &[FormIssue],
        editable: bool,
    ) -> Element<'_, Message> {
        let Some(form) = form else {
            return space::horizontal().into();
        };
        let errors = issues
            .iter()
            .filter(|i| i.severity == FormSeverity::Error)
            .count();
        let mut status = format!("Taslak {}. sürüm", form.draft.revision);
        match form.releases.last() {
            Some(r) => status.push_str(&format!(" · son yayın {}", r.version)),
            None => status.push_str(" · yayınlanmadı"),
        }
        if form.withdrawn {
            status.push_str(" · yayından kaldırıldı");
        }
        if w.dirty {
            status.push_str(" · kaydedilmedi");
        }
        let name = kentos_ui::widget::text_field("Formun adı", &w.name)
            .on_input(|t| msg(Event::Name(t)))
            .on_submit(msg(Event::Save))
            .width(240);
        let enforce = words::check(
            form.enforce,
            ENFORCE,
            (editable && (!form.releases.is_empty() || form.enforce))
                .then(|| msg(Event::Enforce(!form.enforce))),
        );
        let clear = w.body.as_ref().is_some_and(|b| b.clear_hidden);
        let clear_hidden = words::check(
            clear,
            "Gizlenenleri sil",
            editable.then(|| msg(Event::ClearHidden(!clear))),
        );
        // Adım adım (docs/adr/0437): a form of two or more tabs.
        let stepped = w.body.as_ref().is_some_and(|b| b.steps);
        let tabbed = w.body.as_ref().is_some_and(|b| {
            b.items
                .iter()
                .filter(|i| matches!(i, FormItem::Tab { .. }))
                .count()
                >= 2
        });
        let steps = kentos_ui::widget::tip(
            words::check(
                stepped,
                "Adım adım",
                (editable && (tabbed || stepped)).then(|| msg(Event::Steps(!stepped))),
            ),
            kentos_ui::widget::Tip::new("Adım adım").body(
                "Sekmeler sırayla doldurulan adımlar olur; sorunlu adımdan ileri geçilmez, Kaydet son adımdadır.",
            ),
            iced::widget::tooltip::Position::Top,
        );
        let top = row![
            label::body(NAME),
            name,
            label::caption(status).style(muted),
            space::horizontal().width(Fill),
            steps,
            clear_hidden,
            enforce,
        ]
        .spacing(10)
        .align_y(Center);
        let actions = row![
            words::ghost(REMOVE_FORM, editable.then(|| msg(Event::RemoveForm))),
            words::ghost(AUDIT, Some(msg(Event::Audit))),
            words::ghost(RECOMPUTE, editable.then(|| msg(Event::Recompute))),
            words::ghost(EXPORT, editable.then(|| msg(Event::Export))),
            // Yayından kaldır / Yeniden yayına al (docs/adr/0445 §1).
            words::ghost(
                if form.withdrawn { PUT_BACK } else { WITHDRAW },
                (editable && !form.releases.is_empty() && (form.withdrawn || !form.enforce))
                    .then(|| msg(Event::Withdraw(!form.withdrawn))),
            ),
            words::ghost(IMPORT, Some(msg(Event::Import))),
            space::horizontal().width(Fill),
            words::secondary(CANCEL, Some(msg(Event::Cancel))),
            words::secondary(
                PUBLISH,
                (editable && errors == 0).then(|| msg(Event::Publish))
            ),
            words::primary(SAVE, editable.then(|| msg(Event::Save))),
        ]
        .spacing(8)
        .align_y(Center);
        // Kapsam (docs/adr/0539): which records a Veri girişi reaches through the form.
        let scope_text = w
            .body
            .as_ref()
            .and_then(|b| b.scope.clone())
            .unwrap_or_default();
        let mut scope_input = kentos_ui::widget::text_field(
            "Bütün kayıtlar (ör. Sorumlu = @kullanici_eposta)",
            &scope_text,
        )
        .width(Fill);
        if editable {
            scope_input = scope_input.on_input(|t| msg(Event::Scope(t)));
        }
        let scope = row![
            kentos_ui::widget::tip(
                label::body("Kapsam"),
                kentos_ui::widget::Tip::new("Kapsam").body(
                    "Veri girişi bu formla yalnız ifadenin doğru olduğu kayıtları görür, sayar ve yazar; @kullanici_eposta oturumdaki kişidir. Boş: katmanın bütün kayıtları.",
                ),
                iced::widget::tooltip::Position::Top,
            ),
            scope_input,
        ]
        .spacing(10)
        .align_y(Center);
        column![top, scope, actions].spacing(10).into()
    }
}

impl App {
    /// İlişki (docs/adr/0440 §6): the project relations whose parent is the form's layer; one picked fills
    /// the related item's layer and field.
    fn relation_pick<'a>(&'a self, layer: &str, field: &str) -> Element<'a, Message> {
        let form_layer = self.chosen_form().map(|f| f.layer.clone());
        let relations: Vec<kentos_commands::ProjectRelation> = self
            .document
            .as_ref()
            .map(|d| {
                d.model
                    .settings()
                    .relations
                    .iter()
                    .filter(|r| Some(&r.parent) == form_layer.as_ref())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let at = relations
            .iter()
            .position(|r| r.child == layer && r.field == field);
        let ids: Vec<String> = relations.iter().map(|r| r.id.clone()).collect();
        words::field(
            "İlişki",
            Select::new(
                relations.iter().map(|r| Choice::new(r.name.clone())),
                at,
                move |i| msg(Event::RelatedFrom(ids[i].clone())),
            )
            .placeholder("Projenin ilişkilerinden"),
            None,
        )
    }

    /// A field's list from another layer (docs/adr/0404 §1): Kendi değerleri or a layer, its key
    /// (Kimlik: the object's persistent id), its label, the chain's matches and the filter.
    fn lookup_rows<'a>(
        &'a self,
        id: &str,
        f: &kentos_commands::FormField,
        error: Option<String>,
    ) -> Element<'a, Message> {
        let layers = self.form_layers();
        let at = f
            .lookup
            .as_ref()
            .and_then(|l| layers.iter().position(|(lid, _)| *lid == l.layer))
            .map_or(0, |i| i + 1);
        let mut names = vec![Choice::new("Alanın kendi değerleri")];
        names.extend(layers.iter().map(|(_, n)| Choice::new(n.clone())));
        let picks = layers.clone();
        let pick_layer = Select::new(names, Some(at), move |i| {
            msg(Event::LookupLayer((i > 0).then(|| picks[i - 1].0.clone())))
        });
        let mut col =
            Column::new()
                .spacing(8)
                .push(words::field("Değer listesi", pick_layer, error));
        let Some(l) = &f.lookup else {
            return col.into();
        };
        let their = self.layer_fields_of(&l.layer);
        let mut keys: Vec<(String, String)> = vec![(
            kentos_commands::LOOKUP_UID.to_owned(),
            "Kimlik (nesneye bağ)".to_owned(),
        )];
        keys.extend(
            their
                .iter()
                .map(|x| (x.key().to_owned(), x.label().to_owned())),
        );
        let at_key = keys.iter().position(|(k, _)| *k == l.key);
        let pk = keys.clone();
        let pick_key = Select::new(
            keys.iter().map(|(_, n)| Choice::new(n.clone())),
            at_key,
            move |i| msg(Event::LookupKey(pk[i].0.clone())),
        );
        let at_label = l
            .label
            .as_ref()
            .and_then(|lab| keys.iter().position(|(k, _)| k == lab));
        let pl = keys.clone();
        let pick_label = Select::new(
            keys.iter().map(|(_, n)| Choice::new(n.clone())),
            at_label,
            move |i| msg(Event::LookupLabel(Some(pl[i].0.clone()))),
        )
        .placeholder("Saklanan değer");
        col = col
            .push(words::field("Saklanan", pick_key, None))
            .push(words::field("Gösterilen", pick_label, None));
        let own = self
            .chosen_form()
            .map(|form| self.layer_fields_of(&form.layer))
            .unwrap_or_default();
        for (i, m) in l.matches.iter().enumerate() {
            let tk: Vec<String> = their.iter().map(|x| x.key().to_owned()).collect();
            let ok: Vec<String> = own.iter().map(|x| x.key().to_owned()).collect();
            let theirs = Select::new(
                their.iter().map(|x| Choice::new(x.label().to_owned())),
                tk.iter().position(|k| *k == m.field),
                move |j| msg(Event::MatchField(i, tk[j].clone())),
            )
            .placeholder("Listenin alanı");
            let ours = Select::new(
                own.iter().map(|x| Choice::new(x.label().to_owned())),
                ok.iter().position(|k| *k == m.record),
                move |j| msg(Event::MatchRecord(i, ok[j].clone())),
            )
            .placeholder("Kaydın alanı");
            // Aa: compared without regard to case (docs/adr/0439 §4).
            let fold = button(label::caption("Aa"))
                .on_press(msg(Event::Lookup(LookupEdit::Fold(i, !m.fold))))
                .padding([4, 6])
                .style(style::button::tool(m.fold));
            col = col.push(
                row![
                    container(theirs).width(Fill),
                    label::caption("=").style(muted),
                    container(ours).width(Fill),
                    kentos_ui::widget::tip(
                        fold,
                        kentos_ui::widget::Tip::new("Büyük küçük harfe bakmadan karşılaştır"),
                        iced::widget::tooltip::Position::Top,
                    ),
                    button(icon(Icon::Close).size(12.0))
                        .on_press(msg(Event::RemoveMatch(i)))
                        .padding(kentos_ui::theme::metrics::icon(12.0))
                        .style(style::button::tool(false)),
                ]
                .spacing(4)
                .align_y(Center),
            );
        }
        let rule = Rule::LookupFilter(id.to_owned());
        let r = rule.clone();
        let filter = kentos_ui::widget::text_field(
            "Süzgeç yok: bütün nesneler",
            l.filter.as_deref().unwrap_or(""),
        )
        .on_input(move |t| msg(Event::RuleText(r.clone(), t)))
        .font(typography::mono());
        col = col
            .push(words::secondary("Zincir ekle", Some(msg(Event::AddMatch))))
            .push(words::field(
                "Listenin süzgeci",
                row![
                    filter,
                    button(label::body("ε"))
                        .on_press(msg(Event::Builder(rule)))
                        .padding([4, 8])
                        .style(style::button::tool(false)),
                ]
                .spacing(4)
                .align_y(Center),
                None,
            ));
        col.push(self.lookup_extras(l, &their, &own)).into()
    }

    /// A list's order, columns, page, duplicates, geometry, copies and stamp (docs/adr/0439).
    fn lookup_extras<'a>(
        &'a self,
        l: &kentos_commands::FormLookup,
        their: &[LayerField],
        own: &[LayerField],
    ) -> Element<'a, Message> {
        let edit = |e: LookupEdit| msg(Event::Lookup(e));
        let their_keys: Vec<String> = their.iter().map(|x| x.key().to_owned()).collect();
        let their_names: Vec<String> = their.iter().map(|x| x.label().to_owned()).collect();
        let name_of = |key: &str| {
            their_keys
                .iter()
                .position(|k| k == key)
                .map_or_else(|| "Kimlik".to_owned(), |i| their_names[i].clone())
        };
        // Sıralama: the labels, or a field, greatest first when asked.
        let mut sorts = vec![Choice::new("Etiketin doğal sırası")];
        sorts.extend(their_names.iter().map(|n| Choice::new(n.clone())));
        let at_sort = l.sort.as_ref().map_or(Some(0), |s| {
            their_keys.iter().position(|k| *k == s.field).map(|i| i + 1)
        });
        let sk = their_keys.clone();
        let sort = Select::new(sorts, at_sort, move |i| {
            msg(Event::Lookup(LookupEdit::Sort(
                (i > 0).then(|| sk[i - 1].clone()),
            )))
        });
        let descending = l.sort.as_ref().is_some_and(|s| s.descending);
        let sort_row = row![
            container(sort).width(Fill),
            button(label::caption("Azalan"))
                .on_press_maybe(
                    l.sort
                        .is_some()
                        .then(|| edit(LookupEdit::Descending(!descending)))
                )
                .padding([4, 8])
                .style(style::button::tool(descending)),
        ]
        .spacing(4)
        .align_y(Center);
        // Sütunlar: chips, one more added from the layer's fields.
        let mut chips = Row::new().spacing(4);
        for (i, c) in l.columns.iter().enumerate() {
            chips = chips.push(
                button(
                    row![label::caption(name_of(c)), icon(Icon::Close).size(10.0)]
                        .spacing(4)
                        .align_y(Center),
                )
                .padding([2, 8])
                .style(style::button::chip(true))
                .on_press(edit(LookupEdit::ColumnRemove(i))),
            );
        }
        let free: Vec<(String, String)> =
            std::iter::once((kentos_commands::LOOKUP_UID.to_owned(), "Kimlik".to_owned()))
                .chain(their_keys.iter().cloned().zip(their_names.iter().cloned()))
                .filter(|(k, _)| !l.columns.contains(k))
                .collect();
        let mut columns = Column::new().spacing(4);
        if !l.columns.is_empty() {
            columns = columns.push(chips.wrap());
        }
        if l.columns.len() < kentos_commands::MOST_LOOKUP_COLUMNS {
            let fk: Vec<String> = free.iter().map(|(k, _)| k.clone()).collect();
            columns = columns.push(
                container(
                    Select::new(
                        free.iter().map(|(_, n)| Choice::new(n.clone())),
                        None,
                        move |i| msg(Event::Lookup(LookupEdit::ColumnAdd(fk[i].clone()))),
                    )
                    .placeholder("Sütun ekle…"),
                )
                .width(200),
            );
        }
        let limit = kentos_ui::widget::text_field(
            "Hepsi",
            &l.limit.map(|n| n.to_string()).unwrap_or_default(),
        )
        .on_input(move |t| msg(Event::Lookup(LookupEdit::Limit(t))))
        .width(90);
        // Yinelenen anahtar.
        use kentos_commands::LookupDuplicates as D;
        let dups = [D::Warn, D::First, D::Refuse];
        let duplicates = Select::new(
            ["Uyar, ilkini al", "İlkini al", "Reddet"].map(Choice::new),
            dups.iter().position(|d| *d == l.duplicates),
            move |i| msg(Event::Lookup(LookupEdit::Duplicates(dups[i]))),
        );
        // Konum: the relation to the record's geometry and its tolerance.
        use kentos_commands::LookupRelation as R;
        let mut places = vec![Choice::new("Konuma bakma")];
        places.extend(R::ALL.iter().map(|r| Choice::new(r.label())));
        let at_place = l.spatial.as_ref().map_or(Some(0), |s| {
            R::ALL.iter().position(|r| *r == s.relation).map(|i| i + 1)
        });
        let place = Select::new(places, at_place, move |i| {
            msg(Event::Lookup(LookupEdit::Spatial(
                (i > 0).then(|| R::ALL[i - 1]),
            )))
        });
        let mut place_row = Row::new()
            .spacing(4)
            .align_y(Center)
            .push(container(place).width(Fill));
        if let Some(s) = &l.spatial {
            place_row = place_row
                .push(label::caption("Tolerans").style(muted))
                .push(
                    kentos_ui::widget::text_field(
                        "0",
                        &s.tolerance.map(|t| t.to_string()).unwrap_or_default(),
                    )
                    .on_input(move |t| msg(Event::Lookup(LookupEdit::Tolerance(t))))
                    .width(80),
                );
        }
        // Kopyala: the chosen object's field into the record's, each row; the stamp.
        let own_keys: Vec<String> = own.iter().map(|x| x.key().to_owned()).collect();
        let own_names: Vec<String> = own.iter().map(|x| x.label().to_owned()).collect();
        let mut copies = Column::new().spacing(4);
        for (i, c) in l.copy.iter().enumerate() {
            let tk = their_keys.clone();
            let ok = own_keys.clone();
            copies = copies.push(
                row![
                    container(
                        Select::new(
                            their_names.iter().map(|n| Choice::new(n.clone())),
                            tk.iter().position(|k| *k == c.from),
                            move |j| msg(Event::Lookup(LookupEdit::CopyFrom(i, tk[j].clone()))),
                        )
                        .placeholder("Listenin alanı")
                    )
                    .width(Fill),
                    label::caption("→").style(muted),
                    container(
                        Select::new(
                            own_names.iter().map(|n| Choice::new(n.clone())),
                            ok.iter().position(|k| *k == c.to),
                            move |j| msg(Event::Lookup(LookupEdit::CopyTo(i, ok[j].clone()))),
                        )
                        .placeholder("Kaydın alanı")
                    )
                    .width(Fill),
                    button(icon(Icon::Close).size(12.0))
                        .on_press(edit(LookupEdit::CopyRemove(i)))
                        .padding(kentos_ui::theme::metrics::icon(12.0))
                        .style(style::button::tool(false)),
                ]
                .spacing(4)
                .align_y(Center),
            );
        }
        let texts: Vec<&LayerField> = own
            .iter()
            .filter(|x| x.kind == kentos_commands::LayerFieldKind::Text)
            .collect();
        let tkeys: Vec<String> = texts.iter().map(|x| x.key().to_owned()).collect();
        let mut stamps = vec![Choice::new("Damga yok")];
        stamps.extend(texts.iter().map(|x| Choice::new(x.label().to_owned())));
        let at_stamp = l.stamp.as_ref().map_or(Some(0), |s| {
            tkeys.iter().position(|k| k == s).map(|i| i + 1)
        });
        let stamp = Select::new(stamps, at_stamp, move |i| {
            msg(Event::Lookup(LookupEdit::Stamp(
                (i > 0).then(|| tkeys[i - 1].clone()),
            )))
        });
        Column::new()
            .spacing(8)
            .push(words::field("Sıralama", sort_row, None))
            .push(words::field("Sütunlar", columns, None))
            .push(words::field("Sayfa", limit, None))
            .push(words::field("Yinelenen anahtar", duplicates, None))
            .push(words::field("Konum", place_row, None))
            .push(words::field("Kopyala", copies, None))
            .push(words::secondary(
                "Kopya ekle",
                Some(edit(LookupEdit::CopyAdd)),
            ))
            .push(words::field("Kaynak damgası", stamp, None))
            .into()
    }

    /// The preview of the copy: the form as Formla düzenle shows it, on the chosen object of its
    /// layer or a new record; typed values are not written.
    fn forms_preview<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        if let Some(why) = &w.preview_error {
            return container(words::summary(vec![words::text_line(
                Kind::Warn,
                why.clone(),
            )]))
            .padding(8)
            .into();
        }
        let Some(e) = &self.form_preview else {
            return empty_inspector();
        };
        let shown = column![self.entry_preview(e), self.scenarios_box(w)].spacing(16);
        scrollable(container(shown).padding(iced::Padding {
            top: 0.0,
            right: 14.0,
            bottom: 0.0,
            left: 2.0,
        }))
        .height(Fill)
        .into()
    }
}

impl App {
    /// Senaryolar (docs/adr/0420): the copy's scenarios run on it as the command runs them, the
    /// preview's values kept as a new one.
    fn scenarios_box<'a>(&self, w: &'a Window) -> Element<'a, Message> {
        let mut col = Column::new().spacing(6).push(label::strong("Senaryolar"));
        let (Some(body), Some(form), Some(doc)) = (&w.body, self.chosen_form(), &self.document)
        else {
            return col.into();
        };
        let prepared = kentos_native_application::forms_test::prepared_for(&doc.model, form, body);
        if body.tests.is_empty() {
            col = col.push(label::caption(
                "Senaryo yok: önizlemeye değer yazın ve senaryo olarak kaydedin; form değişince yeniden çalışır.",
            ).style(muted));
        }
        let mut passed = 0;
        for (i, t) in body.tests.iter().enumerate() {
            let differences = match &prepared {
                Ok(p) => kentos_forms::scenario::run(p, t),
                Err(why) => vec![why.clone()],
            };
            let ok = differences.is_empty();
            passed += usize::from(ok);
            let mark = if ok {
                icon(Icon::Check).size(14.0).tone(Tone::Success)
            } else {
                icon(Icon::Warning).size(14.0).tone(Tone::Danger)
            };
            let mut line = Column::new().spacing(2).push(
                row![
                    mark,
                    container(label::body(t.name.clone())).width(Fill),
                    button(icon(Icon::Close).size(12.0))
                        .on_press(msg(Event::RemoveScenario(i)))
                        .padding(kentos_ui::theme::metrics::icon_small(12.0))
                        .style(style::button::tool(false)),
                ]
                .spacing(6)
                .align_y(Center),
            );
            for d in differences {
                line = line.push(container(label::caption(d).style(muted)).padding([0, 20]));
            }
            col = col.push(line);
        }
        if !body.tests.is_empty() {
            col = col.push(label::caption(format!(
                "{passed} / {} senaryo geçiyor.",
                body.tests.len()
            )));
        }
        let name = kentos_ui::widget::text_field("Senaryonun adı", &w.scenario_name)
            .on_input(|t| msg(Event::ScenarioName(t)))
            .on_submit(msg(Event::SaveScenario));
        col.push(
            row![
                container(name).width(Fill),
                words::secondary("Senaryo olarak kaydet", Some(msg(Event::SaveScenario))),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .into()
    }
}

impl App {
    /// İş akışı (docs/adr/0428): the state field, the first state, the locking states and the steps.
    fn forms_flow<'a>(&self, w: &'a Window) -> Element<'a, Message> {
        let Some(body) = &w.body else {
            return empty_inspector();
        };
        let layer = self
            .chosen_form()
            .map(|f| f.layer.clone())
            .unwrap_or_default();
        let fields = self.layer_fields_of(&layer);
        let Some(flow) = &body.workflow else {
            return column![
                label::caption(
                    "İş akışı yok. Kayıtlar bir durum alanıyla (Taslak, Gönderildi, Onaylandı gibi) adım adım ilerlesin: \
                     her adım bir düğme olur, geçerli kayıt ya da gerekçe isteyebilir, onaylı kayıt kilitlenebilir.",
                )
                .style(muted),
                words::secondary("İş akışı ekle", Some(msg(Event::WorkflowAdd))),
            ]
            .spacing(10)
            .into();
        };
        let listed: Vec<&LayerField> = fields
            .iter()
            .filter(|f| f.values.as_ref().is_some_and(|v| !v.is_empty()) && !f.holds_many())
            .collect();
        let keys: Vec<String> = listed.iter().map(|f| f.key().to_owned()).collect();
        let state_field = Select::new(
            listed.iter().map(|f| Choice::new(f.label().to_owned())),
            keys.iter().position(|k| *k == flow.field),
            move |i| msg(Event::WorkflowField(keys[i].clone())),
        );
        let states: Vec<(String, String)> = fields
            .iter()
            .find(|f| f.key() == flow.field)
            .and_then(|f| f.values.as_ref())
            .map(|v| {
                v.iter()
                    .map(|c| (c.code.clone(), c.label.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let codes: Vec<String> = states.iter().map(|(c, _)| c.clone()).collect();
        let initial_codes = codes.clone();
        let initial = Select::new(
            states.iter().map(|(_, l)| Choice::new(l.clone())),
            codes.iter().position(|c| *c == flow.initial),
            move |i| msg(Event::WorkflowInitial(initial_codes[i].clone())),
        );
        let chips = |on: &[String], each: &dyn Fn(String) -> Event| {
            let mut row = Row::new().spacing(4);
            for (code, words) in &states {
                row = row.push(
                    button(label::caption(words.clone()))
                        .on_press(msg(each(code.clone())))
                        .padding([2, 8])
                        .style(style::button::chip(on.contains(code))),
                );
            }
            row.wrap()
        };
        let reasons: Vec<&LayerField> = fields
            .iter()
            .filter(|f| f.kind == kentos_commands::LayerFieldKind::Text && f.values.is_none())
            .collect();
        let mut col = Column::new()
            .spacing(10)
            .push(words::field("Durum alanı", state_field, None))
            .push(words::field("İlk durum", initial, None))
            .push(words::field(
                "Kilitli durumlar",
                chips(&flow.locked, &Event::WorkflowLocked),
                Some("Bu durumdaki kayıt değiştirilemez; yalnız bir adım onu açar.".into()),
            ))
            .push(words::field(
                "Taslak durumlar",
                chips(&flow.drafts, &Event::WorkflowDraft),
                Some(
                    "Bu durumdaki kayıt eksik kaydedilebilir: formun zorunlu alanları uyarır."
                        .into(),
                ),
            ));
        // Geçmiş alanı (docs/adr/0429): each step's line, read-only in the form.
        let log_keys: Vec<String> = reasons.iter().map(|f| f.key().to_owned()).collect();
        let mut log_choices = vec![Choice::new("—")];
        log_choices.extend(reasons.iter().map(|f| Choice::new(f.label().to_owned())));
        let log_at = match &flow.log {
            None => Some(0),
            Some(k) => log_keys.iter().position(|x| x == k).map(|p| p + 1),
        };
        col = col
            .push(words::field(
                "Geçmiş alanı",
                Select::new(log_choices, log_at, move |k| {
                    msg(Event::WorkflowLog(
                        k.checked_sub(1).map(|k| log_keys[k].clone()),
                    ))
                }),
                Some("Her adım tarih, kişi, adım ve gerekçeyle bir satır ekler.".into()),
            ))
            .push(label::strong("Adımlar"));
        for (i, step) in flow.steps.iter().enumerate() {
            let reason_keys: Vec<String> = reasons.iter().map(|f| f.key().to_owned()).collect();
            let mut reason_choices = vec![Choice::new("—")];
            reason_choices.extend(reasons.iter().map(|f| Choice::new(f.label().to_owned())));
            let reason_at = match &step.reason {
                None => Some(0),
                Some(k) => reason_keys.iter().position(|x| x == k).map(|p| p + 1),
            };
            let label_input = kentos_ui::widget::text_field("Adımın adı", &step.label)
                .on_input(move |t| msg(Event::StepLabel(i, t)));
            let to_codes = codes.clone();
            let to = Select::new(
                states.iter().map(|(_, l)| Choice::new(l.clone())),
                codes.iter().position(|c| *c == step.to),
                move |k| msg(Event::StepTo(i, to_codes[k].clone())),
            );
            let reason = Select::new(reason_choices, reason_at, move |k| {
                msg(Event::StepReason(
                    i,
                    k.checked_sub(1).map(|k| reason_keys[k].clone()),
                ))
            });
            let boxed = column![
                row![
                    container(label_input).width(Fill),
                    button(icon(Icon::Close).size(12.0))
                        .on_press(msg(Event::StepRemove(i)))
                        .padding(kentos_ui::theme::metrics::icon_small(12.0))
                        .style(style::button::tool(false)),
                ]
                .spacing(6)
                .align_y(Center),
                words::field(
                    "Şu durumlardan",
                    chips(&step.from, &move |c| Event::StepFrom(i, c)),
                    Some(if step.from.is_empty() {
                        "Seçilmezse her durumdan.".into()
                    } else {
                        "Yalnız seçilen durumlardaki kayıtta görünür.".into()
                    }),
                ),
                words::field("Şu duruma", to, None),
                words::check(
                    step.valid,
                    "Geçerli kayıt ister",
                    Some(msg(Event::StepValid(i, !step.valid)))
                ),
                words::field(
                    "Gerekçe alanı",
                    reason,
                    Some("Seçilirse adım gerekçe yazılmadan atılmaz.".into())
                ),
            ]
            .spacing(8);
            col = col.push(
                container(boxed)
                    .padding(10)
                    .style(style::container::field_box)
                    .width(Fill),
            );
        }
        col = col.push(
            row![
                words::secondary("Adım ekle", Some(msg(Event::StepAdd))),
                space::horizontal().width(Fill),
                words::ghost("İş akışını kaldır", Some(msg(Event::WorkflowRemove))),
            ]
            .spacing(8)
            .align_y(Center),
        );
        scrollable(col.padding(iced::Padding {
            top: 0.0,
            right: 14.0,
            bottom: 0.0,
            left: 2.0,
        }))
        .height(Fill)
        .into()
    }

    /// Değişiklikler (docs/adr/0427): the window's copy against the form's last release, item by item;
    /// pressing a change chooses its item.
    fn forms_changes<'a>(&self, w: &'a Window) -> Element<'a, Message> {
        let Some(body) = &w.body else {
            return empty_inspector();
        };
        let Some(last) = self.chosen_form().and_then(|f| f.releases.last()) else {
            return container(label::caption(
                "Form hiç yayınlanmadı: karşılaştırılacak yayın yok. İlk yayından sonra taslağın farkı burada görünür.",
            ))
            .padding(12)
            .into();
        };
        let changes = kentos_forms::compare::compare(&last.body, body);
        let mut col = Column::new().spacing(4).push(label::caption(format!(
            "{}. yayından bu yana {} değişiklik.",
            last.version,
            changes.len()
        )));
        // What the draft needs of its host that the release did not, and the other way (docs/adr/0442 §3).
        let (was, now) = (
            kentos_forms::requirements(&last.body),
            kentos_forms::requirements(body),
        );
        let added: Vec<String> = now
            .iter()
            .filter(|r| !was.contains(r))
            .map(|r| require_words(r))
            .collect();
        let gone: Vec<String> = was
            .iter()
            .filter(|r| !now.contains(r))
            .map(|r| require_words(r))
            .collect();
        if !added.is_empty() {
            col = col.push(words::text_line(
                Kind::Warn,
                format!("Yeni gerektirdikleri: {}", added.join(", ")),
            ));
        }
        if !gone.is_empty() {
            col = col.push(
                label::caption(format!("Artık gerektirmedikleri: {}", gone.join(", ")))
                    .style(muted),
            );
        }
        if changes.is_empty() {
            col = col.push(label::caption("Taslak son yayınla aynı.").style(muted));
        }
        for c in changes {
            use kentos_commands::FormChangeKind as K;
            // A mark of the change's kind: added green, removed red, moved and changed in the accent.
            let (mark, tone): (&str, fn(&Tokens) -> iced::Color) = match c.kind {
                K::Added => ("+", |t| t.success),
                K::Removed => ("−", |t| t.danger),
                K::Moved => ("↕", |t| t.accent),
                K::Changed => ("•", |t| t.accent),
            };
            let line = row![
                container(label::strong(mark).style(move |theme: &iced::Theme| {
                    iced::widget::text::Style {
                        color: Some(tone(&Tokens::of(theme))),
                    }
                }))
                .width(14),
                label::body(c.words.clone()).width(Fill),
                label::caption(
                    c.areas
                        .iter()
                        .map(|a| match a.as_str() {
                            "layout" => "yerleşim",
                            "binding" => "bağ",
                            "rule" => "kural",
                            "action" => "eylem",
                            "workflow" => "iş akışı",
                            "tests" => "senaryo",
                            other => other,
                        })
                        .collect::<Vec<_>>()
                        .join(" · ")
                )
                .style(muted)
            ]
            .spacing(6)
            .align_y(Center);
            col = col.push(match c.item.filter(|_| c.kind != K::Removed) {
                Some(id) => button(line)
                    .on_press(msg(Event::SelectItem(id)))
                    .padding([3, 6])
                    .width(Fill)
                    .style(style::button::row(false))
                    .into(),
                None => Element::from(container(line).padding([3, 6])),
            });
        }
        scrollable(col.padding(iced::Padding {
            top: 0.0,
            right: 14.0,
            bottom: 0.0,
            left: 2.0,
        }))
        .height(Fill)
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
