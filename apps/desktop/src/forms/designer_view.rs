//! Formlar's view (docs/adr/0401): the forms' list and the layer's free fields, the item tree with
//! its toolbar and the preflight's issues, the chosen item's inspector and the footer.

use iced::widget::{Column, Row, button, column, container, row, scrollable, space, text_input};
use iced::{Center, Element, Fill};
use kentos_contracts::{
    FormBody, FormIssue, FormItem, FormSeverity, FormWidget, LayerField, LayerNodeType, ProjectForm,
};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, Elided, Segmented, focus_ring, overlay};
use kentos_ui::{label, style};

use kentos_ui::widget::tree_view::{self, Node, TreeView};

use super::designer::{
    ADD_ATTACHMENTS, ADD_GROUP, ADD_LOCATION, ADD_NOTE, ADD_RELATED, ADD_TAB, AUDIT, CANCEL,
    CHANGES, CREATE, DOWN, ENFORCE, Event, FLOW, NAME, NEW_FORM, PREVIEW, PROPERTIES, PUBLISH,
    RECOMPUTE, REDO, REMOVE_FORM, REMOVE_ITEM, Rule, SAVE, TITLE, Text, UNDO, UP, Window, bound,
    find, msg,
};
use crate::app::{App, Message};
use crate::exchange::words::{self, Kind};

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
            if f.required || layer.is_some_and(|l| l.required) {
                words.push_str(" *");
            }
            (Icon::Type, words, ruled)
        }
    }
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
        let main = row![
            container(left).width(220).height(Fill),
            container(center).width(Fill).height(Fill),
            container(inspector)
                .width(if w.previewing || w.changes || w.flow {
                    400
                } else {
                    340
                })
                .height(Fill),
        ]
        .spacing(12)
        .height(typography::scaled(390.0));

        let mut body = Column::new().spacing(12);
        if let Some(why) = locked {
            body = body.push(words::summary(vec![words::text_line(Kind::Warn, why)]));
        }
        body = body.push(main);
        if form.is_some() {
            body = body.push(self.forms_footer(w, form, &issues, locked.is_none()));
        }
        let title = match form {
            Some(f) if w.dirty => format!("{TITLE} · {} ●", f.name),
            Some(f) => format!("{TITLE} · {}", f.name),
            None => TITLE.to_owned(),
        };
        overlay::modal(
            Frame::new(title).push(body).width(1040.0),
            msg(Event::Cancel),
        )
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
                let name = text_input("Formun adı (boşsa katmandan)", &n.name)
                    .size(typography::body())
                    .on_input(|t| msg(Event::NewName(t)))
                    .on_submit(msg(Event::Create))
                    .padding([5, 8])
                    .style(style::field::input);
                col = col.push(focus_ring(name)).push(pick).push(
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
            col = col
                .push(label::strong("Katmanın alanları"))
                .push(scrollable(palette).height(Fill));
        }
        col.into()
    }

    /// The layers a form can be made for, by their path.
    fn form_layers(&self) -> Vec<(String, String)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let mut out = Vec::new();
        fn walk(nodes: &[kentos_contracts::LayerNode], at: &str, out: &mut Vec<(String, String)>) {
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
                    .padding([4, 7])
                    .style(style::button::tool(false)),
                words.into(),
                iced::widget::tooltip::Position::Bottom,
            )
        };
        let has_item = chosen.is_some();
        let toolbar = row![
            tool(ADD_TAB, Icon::Tabs, Some(Event::AddTab)),
            tool(ADD_GROUP, Icon::Layout, Some(Event::AddGroup)),
            tool(ADD_NOTE, Icon::Info, Some(Event::AddNote)),
            tool(ADD_RELATED, Icon::Link, Some(Event::AddRelated)),
            tool(
                ADD_ATTACHMENTS,
                crate::icons::from_web(Some("photo")),
                Some(Event::AddAttachments)
            ),
            tool(
                ADD_LOCATION,
                crate::icons::from_web(Some("pin")),
                Some(Event::AddLocation)
            ),
            space::horizontal().width(8),
            icon_tool(UP, Icon::ChevronUp, has_item.then_some(Event::Move(-1))),
            icon_tool(DOWN, Icon::ChevronDown, has_item.then_some(Event::Move(1))),
            icon_tool(
                REMOVE_ITEM,
                Icon::Eraser,
                has_item.then_some(Event::RemoveItem)
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
            focus_ring(
                text_input(placeholder, value)
                    .size(typography::body())
                    .on_input(move |t| msg(Event::Text(which.clone(), t)))
                    .padding([5, 8])
                    .style(style::field::input),
            )
        };
        let rule_in = |words: &'static str,
                       rule: Rule,
                       value: &str,
                       placeholder: &'static str|
         -> Element<'_, Message> {
            let r = rule.clone();
            let field = text_input(placeholder, value)
                .size(typography::body())
                .on_input(move |t| msg(Event::RuleText(r.clone(), t)))
                .padding([5, 8])
                .font(typography::mono())
                .style(style::field::input);
            let builder = button(label::body("ε"))
                .on_press(msg(Event::Builder(rule.clone())))
                .padding([4, 8])
                .style(style::button::tool(false));
            words::field(
                words,
                row![focus_ring(field), builder].spacing(4).align_y(Center),
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
            }
            FormItem::Related {
                label: text,
                layer,
                field,
                visible_when,
                view,
                columns,
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
                    .push(words::field("Katman", pick_layer, error_of("related")))
                    .push(words::field("Alan", pick_field, None))
                    .push(
                        label::caption(
                            "Seçilen alanında bu kaydın kalıcı kimliğini tutan nesneler listelenir; her biri kendi formuyla açılır.",
                        )
                        .style(muted),
                    );
                // Liste | Tablo, and the table's columns (docs/adr/0425).
                use kentos_contracts::RelatedView as RV;
                let views = [RV::List, RV::Table];
                col = col.push(words::field(
                    "Görünüm",
                    Select::new(
                        ["Liste", "Tablo"].map(Choice::new),
                        views.iter().position(|v| v == view),
                        move |i| msg(Event::RelatedView(views[i])),
                    ),
                    None,
                ));
                if *view == RV::Table {
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
            FormItem::Attachments {
                label: text,
                accept,
                min,
                max,
                visible_when,
                ..
            } => {
                use kentos_contracts::AttachmentAccept as A;
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
                let mut choices = vec![Choice::new("Alanın türüne göre")];
                choices.extend(fits.iter().map(|w| Choice::new(w.label())));
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
                    .push(words::field("Öğe türü", widget, None))
                    .push(self.lookup_rows(&id, f, error_of("lookup")))
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
                                    l.kind == kentos_contracts::LayerFieldKind::Text
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
                    col = col
                        .push(rule_row("Kural", Rule::Check(id.clone(), i), &c.rule))
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
                                    .padding([5, 6])
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
        scrollable(col.padding(iced::Padding {
            top: 0.0,
            right: 16.0,
            bottom: 0.0,
            left: 4.0,
        }))
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
            | FormItem::Location { visible_when, .. } => visible_when.clone(),
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
        f: &kentos_contracts::FormField,
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
            .filter(|u| u.how != kentos_contracts::FieldUseHow::Key && !u.kept)
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
        if w.dirty {
            status.push_str(" · kaydedilmedi");
        }
        let name = text_input("Formun adı", &w.name)
            .size(typography::body())
            .on_input(|t| msg(Event::Name(t)))
            .on_submit(msg(Event::Save))
            .padding([5, 8])
            .width(240)
            .style(style::field::input);
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
        let top = row![
            label::body(NAME),
            focus_ring(name),
            label::caption(status).style(muted),
            space::horizontal().width(Fill),
            clear_hidden,
            enforce,
        ]
        .spacing(10)
        .align_y(Center);
        let actions = row![
            words::ghost(REMOVE_FORM, editable.then(|| msg(Event::RemoveForm))),
            words::ghost(AUDIT, Some(msg(Event::Audit))),
            words::ghost(RECOMPUTE, editable.then(|| msg(Event::Recompute))),
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
        column![top, actions].spacing(10).into()
    }
}

impl App {
    /// A field's list from another layer (docs/adr/0404 §1): Kendi değerleri or a layer, its key
    /// (Kimlik: the object's persistent id), its label, the chain's matches and the filter.
    fn lookup_rows<'a>(
        &'a self,
        id: &str,
        f: &kentos_contracts::FormField,
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
            kentos_contracts::LOOKUP_UID.to_owned(),
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
            col = col.push(
                row![
                    container(theirs).width(Fill),
                    label::caption("=").style(muted),
                    container(ours).width(Fill),
                    button(icon(Icon::Close).size(12.0))
                        .on_press(msg(Event::RemoveMatch(i)))
                        .padding([5, 6])
                        .style(style::button::tool(false)),
                ]
                .spacing(4)
                .align_y(Center),
            );
        }
        let rule = Rule::LookupFilter(id.to_owned());
        let r = rule.clone();
        let filter = text_input(
            "Süzgeç yok: bütün nesneler",
            l.filter.as_deref().unwrap_or(""),
        )
        .size(typography::body())
        .on_input(move |t| msg(Event::RuleText(r.clone(), t)))
        .padding([5, 8])
        .font(typography::mono())
        .style(style::field::input);
        col.push(words::secondary("Zincir ekle", Some(msg(Event::AddMatch))))
            .push(words::field(
                "Listenin süzgeci",
                row![
                    focus_ring(filter),
                    button(label::body("ε"))
                        .on_press(msg(Event::Builder(rule)))
                        .padding([4, 8])
                        .style(style::button::tool(false)),
                ]
                .spacing(4)
                .align_y(Center),
                None,
            ))
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
                        .padding([3, 6])
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
        let name = text_input("Senaryonun adı", &w.scenario_name)
            .size(typography::body())
            .on_input(|t| msg(Event::ScenarioName(t)))
            .on_submit(msg(Event::SaveScenario))
            .padding([4, 8])
            .style(style::field::input);
        col.push(
            row![
                container(focus_ring(name)).width(Fill),
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
            .filter(|f| f.kind == kentos_contracts::LayerFieldKind::Text && f.values.is_none())
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
            let label_input = text_input("Adımın adı", &step.label)
                .size(typography::body())
                .on_input(move |t| msg(Event::StepLabel(i, t)))
                .padding([4, 8])
                .style(style::field::input);
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
                    container(focus_ring(label_input)).width(Fill),
                    button(icon(Icon::Close).size(12.0))
                        .on_press(msg(Event::StepRemove(i)))
                        .padding([3, 6])
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
        if changes.is_empty() {
            col = col.push(label::caption("Taslak son yayınla aynı.").style(muted));
        }
        for c in changes {
            use kentos_contracts::FormChangeKind as K;
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
                label::body(c.words.clone()).width(Fill)
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
