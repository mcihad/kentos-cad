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
            frame = frame.footer(self.forms_footer(w, form, &issues, locked.is_none()));
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
