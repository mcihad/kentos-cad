//! A form's items on screen: fields and notes, groups and two columns, attachments; which groups start
//! closed and what a field's chosen value and its notes say.

use super::*;

impl App {
    /// The items of a tab or of the top: fields and notes in a form, a group under its title, two
    /// columns side by side.
    pub(crate) fn entry_block<'a>(
        &'a self,
        e: &'a Entry,
        items: &'a [FormItem],
        locked: bool,
    ) -> Element<'a, Message> {
        let mut out = Column::new().spacing(14);
        let mut run: Vec<&FormItem> = Vec::new();
        let flush = |run: &mut Vec<&'a FormItem>, out: Column<'a, Message>| {
            if run.is_empty() {
                return out;
            }
            let items: Vec<&'a FormItem> = std::mem::take(run);
            // A wide window (a big form made larger, docs/adr/0746) takes the fields in two columns, down then across:
            // the form's order is Tab's; the designer's narrow preview and the docked editor keep one (docs/adr/0751).
            let form: Element<'a, Message> = if items.len() > 1 && !e.preview && !e.docked {
                let wide = typography::scaled(TWO_COLUMNS);
                kentos_ui::widget::Responsive::new(move |size| {
                    if size.width < wide {
                        return self.entry_form(e, &items, locked);
                    }
                    let half = items.len().div_ceil(2);
                    row![
                        container(self.entry_form(e, &items[..half], locked)).width(Fill),
                        container(self.entry_form(e, &items[half..], locked)).width(Fill),
                    ]
                    .spacing(24)
                    .into()
                })
                .height(iced::Length::Shrink)
                .into()
            } else {
                self.entry_form(e, &items, locked)
            };
            out.push(form)
        };
        for item in items {
            if !e.state(item.id()).is_some_and(|s| s.shown) {
                continue;
            }
            match item {
                FormItem::Group {
                    id,
                    label: title,
                    columns,
                    items: inner,
                    collapsed,
                    across,
                    ..
                } => {
                    out = flush(&mut run, out);
                    let closed = e.closed.contains(id);
                    if *collapsed {
                        // Its title opens and closes it.
                        let words = e
                            .words_of(id, title.as_ref())
                            .unwrap_or_else(|| "Grup".to_owned());
                        let head = iced::widget::button(
                            row![
                                icon(if closed {
                                    Icon::ChevronRight
                                } else {
                                    Icon::ChevronDown
                                })
                                .size(12.0),
                                label::strong(words),
                            ]
                            .spacing(6)
                            .align_y(Center),
                        )
                        .padding([2, 4])
                        .style(style::button::ghost)
                        .on_press(e.msg(Event::Group(id.clone())));
                        if closed {
                            out = out.push(
                                container(head)
                                    .padding(6)
                                    .style(style::container::field_box)
                                    .width(Fill),
                            );
                            continue;
                        }
                    }
                    let shown: Vec<&FormItem> = inner
                        .iter()
                        .filter(|i| e.state(i.id()).is_some_and(|s| s.shown))
                        .collect();
                    // The designer's narrow preview keeps one column.
                    let content: Element<'a, Message> =
                        if *columns == Some(2) && shown.len() > 1 && !e.preview && *across {
                            // A row at a time: Tab goes across, then down (docs/adr/0443 §6).
                            let mut rows = Column::new().spacing(8);
                            for pair in shown.chunks(2) {
                                let right: Element<'a, Message> = match pair.get(1) {
                                    Some(r) => {
                                        self.entry_form_in(e, std::slice::from_ref(r), locked, 80.0)
                                    }
                                    None => space::horizontal().width(Fill).into(),
                                };
                                rows = rows.push(
                                    row![
                                        container(self.entry_form_in(e, &pair[..1], locked, 80.0))
                                            .width(Fill),
                                        container(right).width(Fill),
                                    ]
                                    .spacing(16),
                                );
                            }
                            rows.into()
                        } else if *columns == Some(2) && shown.len() > 1 && !e.preview {
                            let half = shown.len().div_ceil(2);
                            row![
                                container(self.entry_form_in(e, &shown[..half], locked, 80.0))
                                    .width(Fill),
                                container(self.entry_form_in(e, &shown[half..], locked, 80.0))
                                    .width(Fill),
                            ]
                            .spacing(16)
                            .into()
                        } else {
                            self.entry_form(e, &shown, locked)
                        };
                    let mut group = Column::new().spacing(8);
                    if *collapsed {
                        group = group.push(
                            iced::widget::button(
                                row![
                                    icon(Icon::ChevronDown).size(12.0),
                                    label::strong(
                                        e.words_of(id, title.as_ref())
                                            .unwrap_or_else(|| "Grup".to_owned())
                                    ),
                                ]
                                .spacing(6)
                                .align_y(Center),
                            )
                            .padding([2, 4])
                            .style(style::button::ghost)
                            .on_press(e.msg(Event::Group(id.clone()))),
                        );
                    } else if let Some(t) = title {
                        group = group.push(label::strong(t.clone()));
                    }
                    out = out.push(
                        container(group.push(content))
                            .padding(10)
                            .style(style::container::field_box)
                            .width(Fill),
                    );
                }
                FormItem::Related {
                    id, label: title, ..
                } => {
                    out = flush(&mut run, out);
                    out = out.push(self.related_box(e, id, title.as_deref()));
                }
                FormItem::Attachments {
                    id,
                    label: title,
                    accept,
                    min,
                    max,
                    ..
                } => {
                    out = flush(&mut run, out);
                    out = out.push(self.attachments_box(
                        e,
                        id,
                        title.as_deref(),
                        *accept,
                        *min,
                        *max,
                    ));
                }
                FormItem::Location { label: title, .. } => {
                    out = flush(&mut run, out);
                    let uid = if e.preview { "" } else { e.uid.as_str() };
                    out = out.push(self.location_box(uid, title.as_deref()));
                }
                // A divider (docs/adr/0441 §2): a line across, its title on it.
                FormItem::Divider { label: title, .. } => {
                    out = flush(&mut run, out);
                    let line = || {
                        container(iced::widget::rule::horizontal(1).style(style::field::hairline))
                            .width(Fill)
                    };
                    out = out.push(match title {
                        Some(t) => Element::from(
                            row![label::caption(t.clone()).style(muted), line()]
                                .spacing(8)
                                .align_y(Center),
                        ),
                        None => line().into(),
                    });
                }
                // An action (docs/adr/0441 §6, 0442 §5): its kind's work on the record; a read-only
                // record turns off the kinds that write (docs/adr/0442 §5).
                FormItem::Action {
                    id,
                    label: title,
                    kind,
                    tool,
                    pin,
                    ..
                } => {
                    out = flush(&mut run, out);
                    // A tool not here, or an extension's whose code cannot run yet, says so under the button
                    // (docs/adr/8057 §12).
                    let state = tool
                        .as_deref()
                        .map(|t| self.pinned_tool_state(t, pin.as_ref()));
                    let known = state
                        .as_ref()
                        .is_none_or(|s| *s == crate::extensions::PinnedTool::Ready);
                    let read_only =
                        self.entry_locked(e) || e.screen.prepared.locked_by_state(&e.before);
                    let can = known
                        && !e.preview
                        && e.members.is_empty()
                        && !(kind.writes() && read_only);
                    let glyph = match kind {
                        kentos_commands::ActionKind::Tool => Icon::Play,
                        kentos_commands::ActionKind::Recompute => Icon::Redo,
                        kentos_commands::ActionKind::Step => Icon::ChevronRight,
                        kentos_commands::ActionKind::Show => Icon::Target,
                    };
                    let button = iced::widget::button(
                        row![
                            icon(glyph).size(12.0),
                            label::body(e.words_of(id, Some(title)).unwrap_or_default())
                        ]
                        .spacing(6)
                        .align_y(Center),
                    )
                    .padding(kentos_ui::theme::metrics::action())
                    .style(style::button::secondary)
                    .on_press_maybe(can.then(|| e.msg(Event::RunAction(id.clone()))));
                    let why = match state {
                        Some(crate::extensions::PinnedTool::Pending(why)) => {
                            Some(format!("Bekliyor: {why}"))
                        }
                        Some(crate::extensions::PinnedTool::Unavailable(why)) => {
                            Some(format!("Kullanılamaz: {why}"))
                        }
                        _ => None,
                    };
                    out = out.push(match why {
                        Some(why) => iced::widget::column![
                            button,
                            iced::widget::text(why)
                                .size(typography::caption())
                                .style(muted)
                                .width(Fill)
                        ]
                        .spacing(4)
                        .into(),
                        None => Element::from(button),
                    });
                }
                FormItem::Note { id, text } => {
                    out = flush(&mut run, out);
                    let text = e.note_in(id).unwrap_or_else(|| {
                        e.state(id)
                            .and_then(|s| s.text.clone())
                            .unwrap_or_else(|| text.clone())
                    });
                    out = out.push(
                        container(
                            iced::widget::text(text)
                                .size(typography::caption())
                                .style(muted)
                                .width(Fill),
                        )
                        .width(Fill),
                    );
                }
                _ => run.push(item),
            }
        }
        flush(&mut run, out).into()
    }

    /// Fields and notes in one form; a nested group is shown by [`Self::entry_block`].
    pub(super) fn entry_form<'a>(
        &'a self,
        e: &'a Entry,
        items: &[&'a FormItem],
        locked: bool,
    ) -> Element<'a, Message> {
        self.entry_form_in(e, items, locked, 130.0)
    }

    /// [`Self::entry_form`] with its label column `width` wide (narrower in two columns).
    pub(super) fn entry_form_in<'a>(
        &'a self,
        e: &'a Entry,
        items: &[&'a FormItem],
        locked: bool,
        width: f32,
    ) -> Element<'a, Message> {
        let mut form = Form::new().label_width(width);
        for item in items {
            match item {
                FormItem::Field(f) => {
                    let Some(field) = e.fields.iter().find(|l| l.key() == f.field) else {
                        continue;
                    };
                    let state = e.state(&f.id);
                    // A mirror is shown, never written (docs/adr/0437); nor is a value the bound table always makes
                    // itself (docs/adr/0754 §3).
                    let read_only = locked
                        || f.mirror
                        || field.made_always()
                        || state.is_some_and(|s| s.read_only);
                    let words = e
                        .words_of(&f.id, f.label.as_ref())
                        .unwrap_or_else(|| field.label().to_owned());
                    let control = entry_control(e, f, field, read_only);
                    // A list's chosen object, its columns live, and the values past the page
                    // (docs/adr/0439 §1, §2).
                    let control: Element<'a, Message> = match state.map(|s| (&s.preview, s.more)) {
                        Some((preview, more)) if preview.is_some() || more.is_some() => {
                            let mut col = Column::new().spacing(3).push(control);
                            if let Some(p) = preview {
                                let line: Vec<String> = p
                                    .iter()
                                    .filter(|v| !v.value.is_empty())
                                    .map(|v| format!("{}: {}", v.label, v.value))
                                    .collect();
                                if !line.is_empty() {
                                    col = col.push(label::caption(line.join("  ·  ")).style(muted));
                                }
                            }
                            if let Some(n) = more.filter(|_| !read_only) {
                                col = col.push(
                                    label::caption(format!(
                                        "Listede {n} değer daha var; aramak için yazın."
                                    ))
                                    .style(muted),
                                );
                            }
                            col.into()
                        }
                        _ => control,
                    };
                    // A number's unit after it (docs/adr/0438 §4).
                    let control: Element<'a, Message> = match &field.unit {
                        Some(unit) => Row::new()
                            .spacing(6)
                            .align_y(Center)
                            .push(container(control).width(Fill))
                            .push(label::caption(unit.clone()).style(muted))
                            .into(),
                        None => control,
                    };
                    // What the listed value held means, said under it (docs/adr/0438 §5).
                    let meaning = e
                        .after
                        .get(&field.name)
                        .and_then(|v| field.values.as_ref()?.iter().find(|c| c.code == *v))
                        .and_then(|c| c.description.clone());
                    // An overridable calculation typed over: ↺ gives it back to its calculation.
                    let control: Element<'a, Message> = if f.overridable
                        && f.calculate.is_some()
                        && !read_only
                        && !e.follows.contains(&f.field)
                    {
                        let back = kentos_ui::widget::tip(
                            iced::widget::button(icon(Icon::Undo).size(12.0))
                                .padding(kentos_ui::theme::metrics::icon_small(12.0))
                                .style(style::button::tool(false))
                                .on_press(e.msg(Event::Value(f.field.clone(), String::new()))),
                            kentos_ui::widget::Tip::new("Hesaba dön")
                                .body("Yazılan değer bırakılır, alan yeniden hesaplanır"),
                            iced::widget::tooltip::Position::Top,
                        );
                        Row::new()
                            .spacing(4)
                            .align_y(Center)
                            .push(container(control).width(Fill))
                            .push(back)
                            .into()
                    } else {
                        control
                    };
                    // Haritadan seç (docs/adr/0759 §1): a list from another layer takes its value from
                    // an object clicked on the map.
                    let control: Element<'a, Message> =
                        match (!read_only).then(|| e.map_pick_of(&f.id)).flatten() {
                            Some(_) => {
                                let pick = kentos_ui::widget::tip(
                                    iced::widget::button(icon(Icon::Target).size(12.0))
                                        .padding(kentos_ui::theme::metrics::icon_small(12.0))
                                        .style(style::button::tool(false))
                                        .on_press(e.msg(Event::PickOnMap(f.id.clone()))),
                                    kentos_ui::widget::Tip::new("Haritadan seç").body(
                                        "Değeri, listenin katmanından bir nesneye tıklayarak verin",
                                    ),
                                    iced::widget::tooltip::Position::Top,
                                );
                                Row::new()
                                    .spacing(4)
                                    .align_y(Center)
                                    .push(container(control).width(Fill))
                                    .push(pick)
                                    .into()
                            }
                            None => control,
                        };
                    // Alanlara not: the field's note for the next step (docs/adr/0442 §4).
                    let control: Element<'a, Message> = if e.reviewing && !f.mirror {
                        let key = f.field.clone();
                        let note = e.field_notes.get(&f.field).map_or("", String::as_str);
                        Column::new()
                            .spacing(3)
                            .push(control)
                            .push(
                                kentos_ui::widget::text_field("Bu alana not", note).on_input(
                                    move |text| e.msg(Event::FieldNote(key.clone(), text)),
                                ),
                            )
                            .into()
                    } else {
                        control
                    };
                    // Leaving the field asks its checks kept for leaving (docs/adr/0760 §2).
                    let control: Element<'a, Message> = if read_only || e.preview {
                        control
                    } else {
                        let id = f.id.clone();
                        kentos_ui::widget::focus_watch(control)
                            .on_focus(move |on| e.msg(Event::Focus(id.clone(), on)))
                            .into()
                    };
                    let said = said_note(e, &f.field);
                    form = form.field(words, control);
                    if state.is_some_and(|s| s.required) {
                        form = form.required();
                    }
                    match e.issue_of(field.key()).filter(|_| !f.mirror) {
                        Some(issue) if issue.severity == FormSeverity::Error => {
                            form = form.error(Some(issue.message.clone()));
                        }
                        Some(issue) => form = form.warning(Some(issue.message.clone())),
                        // The last step's note on it, as the reviewer wrote it.
                        None if said.is_some() => {
                            form = form.warning(said.map(|n| format!("İnceleyenin notu: {n}")));
                        }
                        None => {
                            if let Some(help) = e.help_in(&f.id).or(f.help.clone()) {
                                form = form.help(help);
                            } else if let Some(meaning) = meaning {
                                form = form.help(meaning);
                            } else if let Some(calc) = &f.calculate {
                                form = form.help(if f.overridable {
                                    format!("Boşken hesaplanır: {calc}; üzerine yazılabilir")
                                } else {
                                    format!("Hesaplanır: {calc}")
                                });
                            }
                        }
                    }
                }
                FormItem::Note { id, text } => {
                    let text = e.note_in(id).unwrap_or_else(|| {
                        e.state(id)
                            .and_then(|s| s.text.clone())
                            .unwrap_or_else(|| text.clone())
                    });
                    form = form.row(container(label::caption(text).style(muted)).width(Fill));
                }
                FormItem::Group { .. }
                | FormItem::Related { .. }
                | FormItem::Attachments { .. }
                | FormItem::Location { .. }
                | FormItem::Divider { .. }
                | FormItem::Action { .. } => {
                    form = form.row(self.entry_block(e, std::slice::from_ref(*item), locked));
                }
                FormItem::Tab { .. } => {}
            }
        }
        form.into()
    }

    /// An attachments item (docs/adr/0407 §1): the record's photos and documents, opened, removed and
    /// added in the form (written at once, as Öznitelikler's Ekler; not from the designer's preview).
    pub(super) fn attachments_box<'a>(
        &'a self,
        e: &'a Entry,
        id: &str,
        title: Option<&str>,
        accept: kentos_commands::AttachmentAccept,
        min: Option<u32>,
        max: Option<u32>,
    ) -> Element<'a, Message> {
        use kentos_commands::{AttachmentAccept, AttachmentKind};
        let photos = accept == AttachmentAccept::Photo;
        let documents = accept == AttachmentAccept::Document;
        let views = self
            .document
            .as_ref()
            .and_then(|d| record_slot(&d.model, &e.uid))
            .filter(|_| !e.preview)
            .map(|slot| self.attachment_views(slot))
            .unwrap_or_default();
        let shown: Vec<_> = views
            .into_iter()
            .filter(|v| match accept {
                AttachmentAccept::Any => true,
                AttachmentAccept::Photo => v.kind == AttachmentKind::Photo,
                AttachmentAccept::Document => v.kind != AttachmentKind::Photo,
            })
            .collect();
        let words = title.map_or_else(
            || {
                match accept {
                    AttachmentAccept::Any => "Ekler",
                    AttachmentAccept::Photo => "Fotoğraflar",
                    AttachmentAccept::Document => "Belgeler",
                }
                .to_owned()
            },
            str::to_owned,
        );
        let mut bounds = Vec::new();
        if let Some(n) = min {
            bounds.push(format!("en az {n}"));
        }
        if let Some(n) = max {
            bounds.push(format!("en çok {n}"));
        }
        let count = if bounds.is_empty() {
            shown.len().to_string()
        } else {
            format!("{} · {}", shown.len(), bounds.join(", "))
        };
        let can = !e.preview && e.members.is_empty();
        let mut col = Column::new().spacing(6).push(
            row![
                icon(crate::icons::from_web(Some(if photos {
                    "photo"
                } else {
                    "document"
                })))
                .size(13.0),
                label::strong(words),
                label::caption(count).style(muted),
                space::horizontal().width(Fill),
                words::ghost(
                    if e.facts.is_some() {
                        "Bilgiyi gizle"
                    } else {
                        "Bilgi"
                    },
                    (!e.preview && !shown.is_empty()).then(|| e.msg(Event::InspectAttachments))
                ),
                words::secondary(
                    if photos { "Fotoğraf ekle" } else { "Ekle" },
                    can.then(|| e.msg(Event::AttachAdd(photos)))
                ),
            ]
            .spacing(8)
            .align_y(Center),
        );
        if shown.is_empty() {
            col = col.push(
                label::caption(if e.preview {
                    "Önizlemede ekler gösterilmez."
                } else if photos {
                    "Fotoğraf yok."
                } else if documents {
                    "Belge yok."
                } else {
                    "Ek yok."
                })
                .style(muted),
            );
        }
        for v in shown {
            col = col.push(
                row![
                    icon(crate::icons::from_web(Some(crate::attachments::kind_icon(
                        v.kind
                    ))))
                    .size(14.0),
                    container(label::body(v.attachment.name.clone())).width(Fill),
                    label::caption(v.source.clone()).style(muted),
                    words::ghost("Aç", Some(e.msg(Event::AttachOpen(v.index)))),
                    words::ghost("Kaldır", can.then(|| e.msg(Event::AttachRemove(v.index)))),
                ]
                .spacing(8)
                .align_y(Center),
            );
            // What it is, as its bytes say (docs/adr/0442 §7).
            if let Some(f) = e.facts.as_ref().and_then(|all| all.get(v.index)) {
                col = col.push(
                    container(label::caption(facts_line(f)).style(muted)).padding(iced::Padding {
                        top: 0.0,
                        right: 0.0,
                        bottom: 0.0,
                        left: 22.0,
                    }),
                );
            }
        }
        if let Some(issue) = e.issues.iter().find(|i| i.item.as_deref() == Some(id)) {
            col = col.push(words::text_line(Kind::Error, issue.message.clone()));
        }
        container(col)
            .padding(10)
            .style(style::container::field_box)
            .width(Fill)
            .into()
    }
}

/// The ids of the groups that start closed.
pub(super) fn collapsed_groups(items: &[FormItem]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for item in items {
        if let FormItem::Group {
            id,
            collapsed: true,
            ..
        } = item
        {
            out.insert(id.clone());
        }
        out.extend(collapsed_groups(item.items()));
    }
    out
}

/// The item a value chosen (not typed) for the field `key` was given on (docs/adr/0760 §2): a list,
/// a check box, a slider or a colour finishes the field as it is chosen, as a browser's change event
/// fires on a selection at once; a typed one is left when the focus leaves it.
pub(super) fn chosen_item(items: &[FormItem], fields: &[LayerField], key: &str) -> Option<String> {
    for item in items {
        if let FormItem::Field(f) = item
            && f.field == key
            && !f.mirror
        {
            let field = fields.iter().find(|l| l.key() == key)?;
            let typed = matches!(
                kentos_forms::widget_of(f, field),
                FormWidget::Text
                    | FormWidget::TextArea
                    | FormWidget::Number
                    | FormWidget::Search
                    | FormWidget::Date
            );
            return (!typed).then(|| f.id.clone());
        }
        if let Some(id) = chosen_item(item.items(), fields, key) {
            return Some(id);
        }
    }
    None
}

/// A field's control by its widget; read-only shows the value without an edit.
/// An attachment's facts on one line: its type, size, moment and place, and its digest's start.
pub(super) fn facts_line(f: &kentos_commands::AttachmentFacts) -> String {
    if f.missing {
        return "Bulunamadı: dosya ya da kitaplıktaki fotoğraf yok.".into();
    }
    let mut parts: Vec<String> = Vec::new();
    parts.extend(f.mime.clone());
    if let Some(n) = f.size {
        parts.push(if n < 1024 {
            format!("{n} B")
        } else if n < 1024 * 1024 {
            format!("{:.1} KB", n as f64 / 1024.0).replace('.', ",")
        } else {
            format!("{:.1} MB", n as f64 / (1024.0 * 1024.0)).replace('.', ",")
        });
    }
    if let Some(at) = &f.taken {
        parts.push(format!("çekim {}", at.replacen('T', " ", 1)));
    }
    if let (Some(lat), Some(lon)) = (f.latitude, f.longitude) {
        parts.push(format!("{lat:.5}, {lon:.5}"));
    }
    if let Some(h) = &f.sha256 {
        parts.push(format!("SHA-256 {}…", &h[..12.min(h.len())]));
    }
    if parts.is_empty() {
        "Adres; indirilmedi.".into()
    } else {
        parts.join(" · ")
    }
}

/// The note the record's last workflow step left on a field (docs/adr/0442 §4), from the workflow's
/// notes field as the record holds it (the window's values aside).
pub(super) fn said_note(e: &Entry, key: &str) -> Option<String> {
    let notes = e.screen.prepared.workflow()?.notes.as_ref()?;
    let name = &e.fields.iter().find(|f| f.key() == notes)?.name;
    let text = e.before.get(name)?;
    let map: BTreeMap<String, String> = serde_json::from_str(text).ok()?;
    map.get(key).cloned()
}

/// The item's placeholder (a made value said, docs/adr/0753 §2), else how the value is written.
pub(super) fn or_shape<'a>(placeholder: &'a str, shape: &'a str) -> &'a str {
    if placeholder.is_empty() {
        shape
    } else {
        placeholder
    }
}
