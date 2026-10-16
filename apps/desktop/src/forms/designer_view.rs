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
    ADD_GROUP, ADD_NOTE, ADD_TAB, CANCEL, CREATE, DOWN, ENFORCE, Event, NAME, NEW_FORM, PREVIEW,
    PROPERTIES, PUBLISH, REDO, REMOVE_FORM, REMOVE_ITEM, Rule, SAVE, TITLE, Text, UNDO, UP, Window,
    bound, find, issues_of, msg,
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
            .map(|b| issues_of(b, &fields))
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
        let inspector = if w.previewing {
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
        // Özellikler | Önizleme over the third column (docs/adr/0403 §3).
        let side = Segmented::new(
            [PROPERTIES, PREVIEW],
            if w.previewing { PREVIEW } else { PROPERTIES },
            |t| msg(Event::Preview(t == PREVIEW)),
        );
        let inspector: Element<'_, Message> = column![side, inspector].spacing(8).into();
        let main = row![
            container(left).width(220).height(Fill),
            container(center).width(Fill).height(Fill),
            container(inspector)
                .width(if w.previewing { 400 } else { 340 })
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
            FormItem::Tab { label: text, .. } => {
                col = col.push(label::strong("Sekme")).push(words::field(
                    "Ad",
                    input("Sekmenin adı", text, Text::Label),
                    None,
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
        scrollable(col.padding(iced::Padding {
            top: 0.0,
            right: 16.0,
            bottom: 0.0,
            left: 4.0,
        }))
        .height(Fill)
        .into()
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
        let shown = self.entry_preview(e);
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
