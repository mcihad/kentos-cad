//! The editor on screen, docked or in a window: its head, tabs and pages, footer and the designer's
//! preview; its controls found by their words.

use super::*;
use crate::gis;

impl App {
    pub(crate) fn form_entry_view(&self) -> Element<'_, Message> {
        let Some(e) = &self.gis.form_entry else {
            return iced::widget::text("").into();
        };
        let name = e.name_in();
        // One size a form, whether what is typed is saved or not (docs/adr/0746).
        let key = format!("{TITLE} · {name}");
        let title = if e.dirty() {
            format!("{key} ●")
        } else {
            key.clone()
        };
        // The pages take what the window has when it was given a size of its own (docs/adr/0746), else they
        // are 460 high (at 12 px text), less in a low window: the footer is always in sight (docs/adr/0749).
        let sized = kentos_ui::widget::sizable::sizes::get(&key).is_some();
        let locked = self.entry_locked(e);
        let mut head = self.entry_head(e, locked);
        let (tabs, page) = self.entry_tabs_and_page(e, locked, Some(f32::INFINITY));
        if let Some(tabs) = tabs {
            head = head.push(tabs);
        }
        let page = container(page.unwrap_or_else(|| space::vertical().into()))
            .height(Fill)
            .max_height(if sized {
                f32::INFINITY
            } else {
                typography::scaled(460.0)
            });
        let frame = Frame::new(title)
            .push(head)
            .fill(page)
            .footer(self.entry_footer(e, locked));
        frame.key(key).width(760.0).modal(e.msg(Event::Cancel))
    }

    /// The docked editor (docs/adr/0416): the form's title, its pages and its footer, scrolled in the
    /// room the table gives it; none when no record's form is docked.
    pub(crate) fn docked_entry_view(&self) -> Option<Element<'_, Message>> {
        let e = self.gis.form_entry.as_ref().filter(|e| e.docked)?;
        let title = if e.dirty() {
            format!("{} ●", e.name)
        } else {
            e.name.clone()
        };
        let body = self.entry_column(e, None, false);
        let pages = scrollable(
            container(
                Column::new()
                    .spacing(10)
                    .push(label::strong(title))
                    .push(body),
            )
            .padding([8, 12]),
        )
        .height(Fill);
        Some(
            Column::new()
                .push(pages)
                .push(kentos_ui::widget::horizontal_divider())
                .push(container(self.entry_footer(e, self.entry_locked(e))).padding([6, 12]))
                .height(Fill)
                .into(),
        )
    }

    /// The editor's column: what it shows, its pages scrolled `height` high (none: as tall as they
    /// are) and its footer.
    pub(super) fn entry_column<'a>(
        &'a self,
        e: &'a Entry,
        height: Option<f32>,
        footer: bool,
    ) -> Column<'a, Message> {
        let locked = self.entry_locked(e);
        let body = self.entry_head(e, locked);
        let mut body = self.entry_pages(e, locked, body, height);
        if footer {
            body = body.push(self.entry_footer(e, locked));
        }
        body
    }

    /// The record's topology findings (docs/adr/3800): how many, whether the drawing changed since the check, and a
    /// line each (problem, rule, state, measure); nothing when the check found none, there was no check or no rule
    /// names the layer. Öznitelikler's Topoloji section opens the tab on a finding.
    pub(super) fn entry_topology<'a>(
        &'a self,
        e: &'a Entry,
        body: Column<'a, Message>,
    ) -> Column<'a, Message> {
        let lines = self.entry_topology_lines(&e.uid);
        if lines.is_empty() {
            return body;
        }
        body.push(words::summary(lines.into_iter().map(|(kind, text)| words::text_line(kind, text)).collect()))
    }

    /// [`Self::entry_topology`]'s lines for the record of `uid`.
    pub(crate) fn entry_topology_lines(&self, uid: &str) -> Vec<(Kind, String)> {
        use crate::topology::object::ObjectFindings;
        let slot = self
            .document
            .as_ref()
            .and_then(|d| kentos_domain::Uuid::parse_str(uid).ok().and_then(|u| d.model.slot_of(u)));
        let Some(ObjectFindings::Checked { findings, old }) = slot.and_then(|s| self.object_findings(s)) else {
            return Vec::new();
        };
        if findings.is_empty() {
            return Vec::new();
        }
        let mut lines = vec![(
            Kind::Warn,
            format!(
                "Topoloji: son denetimde {} bulgu{}.",
                findings.len(),
                if old { "; çizim denetimden sonra değişti" } else { "" }
            ),
        )];
        for f in findings {
            let measure = if f.measure.is_empty() { String::new() } else { format!(" · {}", f.measure) };
            lines.push((Kind::Info, format!("{} · {} · {}{measure}", f.problem, f.rule, f.state)));
        }
        lines
    }

    /// What the editor shows above its pages: the record, its place among the layer's, how the form is
    /// opened, and what is wrong with its values.
    pub(super) fn entry_head<'a>(&'a self, e: &'a Entry, locked: bool) -> Column<'a, Message> {
        let mut body = Column::new().spacing(12);
        let mut head = if e.members.is_empty() {
            format!("{} · {}", e.screen.object.kind, e.screen.object.layer)
        } else {
            format!(
                "{} nesne · {} · değiştirdiğiniz alanlar hepsine yazılır",
                e.members.len(),
                e.screen.object.layer
            )
        };
        if e.draft {
            head.push_str(" · form yayınlanmadı, taslağıyla");
        }
        // The forms waiting under this one (docs/adr/0440 §7), the nearest last.
        if let Some(under) = self.gis.form_stack.last() {
            head.push_str(&format!(
                " · altta “{}” formu bekliyor{}",
                under.name,
                match self.gis.form_stack.len() {
                    1 => String::new(),
                    n => format!(" ({n} form)"),
                }
            ));
        }
        // ◀ 3 / 12 ▶: the layer's records one after another (docs/adr/0408 §3).
        let step = |glyph: Icon, to: i32, can: bool| {
            iced::widget::button(icon(glyph).size(12.0))
                .padding(kentos_ui::theme::metrics::icon_small(12.0))
                .style(style::button::tool(false))
                .on_press_maybe(can.then(|| e.msg(Event::Step(to))))
        };
        let mut top = Row::new()
            .spacing(6)
            .align_y(Center)
            .push(container(label::caption(head).style(muted)).width(Fill));
        if let Some((at, n)) = e.place.filter(|(_, n)| *n > 1) {
            top = top
                .push(step(Icon::ChevronLeft, -1, at > 0))
                .push(label::caption(format!("{} / {n}", at + 1)).style(muted))
                .push(step(Icon::ChevronRight, 1, at + 1 < n));
        }
        // How the form is opened (docs/adr/0444 §3): its release, whose rules write, what it needs and
        // why it is read-only.
        let (lines, stale) = self.entry_how(e, locked);
        top = top.push(kentos_ui::widget::tip(
            icon(Icon::Info)
                .size(13.0)
                .tone(kentos_ui::icon::Tone::Muted),
            kentos_ui::widget::Tip::new("Bu form").body(lines.join("\n")),
            iced::widget::tooltip::Position::Bottom,
        ));
        body = body.push(top);
        if stale && !e.preview {
            body = body.push(words::summary(vec![words::text_line(
                Kind::Warn,
                "Katmanın alanları bu yayından sonra değişti; tasarımcı formu gözden geçirip yeniden yayınlamalı.",
            )]));
        }
        if e.read_only {
            body = body.push(words::summary(vec![words::text_line(
                Kind::Info,
                "İlişkili kayıtlar öğesi bu alt kaydı yalnız okumaya açıyor: kayıt görüntülenir, değiştirilmez.",
            )]));
        } else if locked {
            body = body.push(words::summary(vec![words::text_line(
                Kind::Warn,
                "Katman kilitli ya da yetkiniz yok: kayıt yalnız görüntülenir.",
            )]));
        }
        // The record's topology findings of the Topoloji tab's last check (docs/adr/3800), one record's alone.
        if e.members.is_empty() && !e.preview && self.shows_property_rows(crate::module_surfaces::PropertyRows::Topology) {
            body = self.entry_topology(e, body);
        }
        // The lists' sources that are not ready, each with its way out (docs/adr/0763).
        body = sources::sources_lines(e, body);
        // The record's state locks it (docs/adr/0428 §2): a step takes it out.
        if !locked && e.screen.prepared.locked_by_state(&e.before) {
            let state = e.screen.prepared.state_of(&e.before).unwrap_or_default();
            let words_of = e
                .screen
                .prepared
                .workflow()
                .and_then(|w| e.fields.iter().find(|f| f.key() == w.field))
                .and_then(|f| f.values.as_ref())
                .and_then(|v| v.iter().find(|c| c.code == state))
                .map_or(state.clone(), |c| c.label.clone());
            body = body.push(words::summary(vec![words::text_line(
                Kind::Info,
                format!(
                    "Kayıt “{words_of}” durumunda; değiştirilemez. İş akışının bir adımı onu düzenlemeye açar."
                ),
            )]));
        }
        // The snapshot the last step was taken on, against the record now (docs/adr/0440 §4).
        let logged = e
            .screen
            .prepared
            .workflow()
            .and_then(|w| w.log.as_ref())
            .and_then(|k| e.fields.iter().find(|f| f.key() == k))
            .and_then(|f| e.before.get(&f.name))
            .and_then(|log| {
                kentos_native_application::forms::logged_snapshot(log).map(str::to_owned)
            });
        if let (Some(then), Some(now)) = (logged, e.snapshot.as_ref()) {
            body = body.push(words::summary(vec![if then == *now {
                words::text_line(
                    Kind::Ok,
                    format!("Son adım bu kayıt üzerinde atıldı (anlık görüntü #{then})."),
                )
            } else {
                words::text_line(
                    Kind::Warn,
                    format!(
                        "Kayıt, alt kayıtları ya da ekleri son adımdan sonra değişti: adım #{then} üzerinde atıldı, şimdi #{now}."
                    ),
                )
            }]));
        }
        // A draft (docs/adr/0432): kept incomplete; the form's requirements warn until a step that
        // asks for a valid record.
        if !locked && e.screen.prepared.in_draft(&e.after) {
            let missing = e
                .issues
                .iter()
                .filter(|i| {
                    i.code == kentos_forms::codes::REQUIRED && i.severity == FormSeverity::Warning
                })
                .count();
            if missing > 0 {
                body = body.push(words::summary(vec![words::text_line(
                    Kind::Info,
                    format!(
                        "Taslak: {missing} zorunlu alan boş; kayıt böyle kaydedilir, ilerletmeden önce doldurun."
                    ),
                )]));
            }
        }
        // The record's kind changed under values it does not offer (docs/adr/0422 §2): kept until the
        // user chooses, never emptied silently.
        let misfits = e.misfits();
        if !misfits.is_empty() && !locked {
            let names: Vec<&str> = misfits.iter().map(|k| e.label_of(k).unwrap_or(k)).collect();
            body = body.push(
                row![
                    container(words::summary(vec![words::text_line(
                        Kind::Warn,
                        format!(
                            "Kaydın alt türü şu değerleri sunmuyor: {}. Alt türün değerlerinden birini seçin ya da boşaltın.",
                            names.join(", ")
                        ),
                    )]))
                    .width(Fill),
                    words::secondary("Uymayanları boşalt", Some(e.msg(Event::ClearMisfits))),
                ]
                .spacing(8)
                .align_y(Center),
            );
        }
        body
    }

    /// The editor's footer: its problems and warnings, Vazgeç and Kaydet.
    pub(super) fn entry_footer<'a>(&'a self, e: &'a Entry, locked: bool) -> Row<'a, Message> {
        let (errors, warnings) = if e.tried {
            (e.errors().count(), e.issues.len() - e.errors().count())
        } else {
            (0, 0)
        };
        let mut footer = Row::new().spacing(8).align_y(Center);
        // Changed since the window opened (docs/adr/0438 §6, §7): reload keeping what was typed, or
        // write over it.
        // A new release (docs/adr/0442 §2): the record opened again by it, the values typed carried.
        if e.conflict.is_some() && e.release_changed {
            return footer
                .push(words::text_line(
                    Kind::Warn,
                    "Form siz düzenlerken yeniden yayınlandı; kayıt yeni sürümün kurallarıyla yazılır.",
                ))
                .push(space::horizontal().width(Fill))
                .push(words::secondary(CANCEL, Some(e.msg(Event::Cancel))))
                .push(words::primary(NEW_RELEASE, Some(e.msg(Event::Reload))));
        }
        if e.asking_close {
            return footer
                .push(words::text_line(Kind::Warn, UNSAVED))
                .push(space::horizontal().width(Fill))
                .push(words::secondary(DISCARD, Some(e.msg(Event::Discard))))
                .push(words::secondary(
                    KEEP_EDITING,
                    Some(e.msg(Event::KeepEditing)),
                ))
                .push(words::primary(
                    SAVE_AND_CLOSE,
                    (!locked).then(|| e.msg(Event::Save)),
                ));
        }
        if e.conflict.is_some() {
            return footer
                .push(words::text_line(
                    Kind::Warn,
                    "Kayıt ya da form siz düzenlerken değişti.",
                ))
                .push(space::horizontal().width(Fill))
                .push(words::secondary(CANCEL, Some(e.msg(Event::Cancel))))
                .push(words::secondary(RELOAD, Some(e.msg(Event::Reload))))
                .push(words::primary(
                    OVERWRITE,
                    (!locked).then(|| e.msg(Event::Overwrite)),
                ));
        }
        if errors > 0 {
            // Where the last press went, then the next (docs/adr/0443 §6).
            let (said, press) = if e.issue_at == 0 || errors == 1 {
                (format!("{errors} sorun"), FIRST_ISSUE)
            } else {
                (
                    format!("{}/{errors} sorun", (e.issue_at - 1) % errors + 1),
                    NEXT_ISSUE,
                )
            };
            footer = footer
                .push(words::text_line(Kind::Error, said))
                .push(words::ghost(press, Some(e.msg(Event::FirstIssue))));
        }
        if warnings > 0 {
            footer = footer.push(words::text_line(Kind::Warn, format!("{warnings} uyarı")));
        }
        footer = footer
            .push(space::horizontal().width(Fill))
            .push(words::secondary(CANCEL, Some(e.msg(Event::Cancel))));
        // Adım adım (docs/adr/0437): Geri and İleri; Kaydet and the workflow's steps on the last step.
        if e.body.steps {
            let last = e.shown_tabs().len().saturating_sub(1);
            if e.tab > 0 {
                footer = footer.push(words::secondary(BACK, Some(e.msg(Event::Tab(e.tab - 1)))));
            }
            if e.tab < last {
                return footer.push(words::primary(NEXT, Some(e.msg(Event::Tab(e.tab + 1)))));
            }
        }
        // The workflow's steps from the record's state (docs/adr/0428), before Kaydet.
        let live = !e.preview && !e.uid.is_empty() && e.members.is_empty();
        if live
            && !locked
            && e.screen
                .prepared
                .workflow()
                .is_some_and(|w| w.notes.is_some())
        {
            footer = footer.push(
                iced::widget::toggler(e.reviewing)
                    .label(FIELD_NOTES)
                    .size(16.0)
                    .text_size(typography::body())
                    .on_toggle(|on| e.msg(Event::Reviewing(on))),
            );
        }
        // Kaynağa da yaz (docs/adr/0532 §1): only where the record's layer is bound to a source.
        if live && !locked && self.bound_link(&e.screen.layer).is_some() {
            footer = footer.push(
                iced::widget::toggler(e.commit)
                    .label(COMMIT)
                    .size(16.0)
                    .text_size(typography::body())
                    .on_toggle(|on| e.msg(Event::Commit(on))),
            );
        }
        // The form's languages (docs/adr/0443 §5): only what is shown changes.
        if !e.body.translations.is_empty() {
            let tags: Vec<Option<String>> = std::iter::once(None)
                .chain(e.body.translations.keys().cloned().map(Some))
                .collect();
            let chosen = tags.iter().position(|t| *t == e.language);
            let picks = tags.clone();
            footer = footer.push(
                container(
                    Select::new(
                        tags.iter()
                            .map(|t| Choice::new(language_name(t.as_deref()))),
                        chosen,
                        move |i| {
                            Message::Gis(gis::Event::FormEntry(Event::Language(picks[i].clone())))
                        },
                    )
                    .placeholder("Dil"),
                )
                .width(140.0),
            );
        }
        let by_state = e.screen.prepared.locked_by_state(&e.after);
        if let (Some(w), Some(state), true) = (
            e.screen.prepared.workflow(),
            e.screen.prepared.state_of(&e.before),
            live,
        ) {
            for step in w.steps_from(&state) {
                footer = footer.push(words::secondary(
                    step.label.as_str(),
                    (!locked).then(|| e.msg(Event::Transition(step.label.clone()))),
                ));
            }
        }
        footer.push(words::primary(
            SAVE,
            (!locked && !by_state).then(|| e.msg(Event::Save)),
        ))
    }

    pub(super) fn entry_locked(&self, e: &Entry) -> bool {
        e.read_only
            || self.forms_locked().is_some()
            || self
                .document
                .as_ref()
                .is_some_and(|d| d.model.layers().is_locked(&e.screen.layer))
    }

    /// The form's pages under `body`: its tabs and the chosen one's items, or its items; scrolled
    /// `height` high, else as tall as they are (the designer's preview scrolls them itself).
    pub(super) fn entry_pages<'a>(
        &'a self,
        e: &'a Entry,
        locked: bool,
        mut body: Column<'a, Message>,
        height: Option<f32>,
    ) -> Column<'a, Message> {
        let (tabs, page) = self.entry_tabs_and_page(e, locked, height);
        if let Some(tabs) = tabs {
            body = body.push(tabs);
        }
        if let Some(page) = page {
            body = body.push(page);
        }
        body
    }

    /// The form's tabs (none when it has none) and the chosen tab's items or its items, scrolled `height`
    /// high, else as tall as they are.
    pub(super) fn entry_tabs_and_page<'a>(
        &'a self,
        e: &'a Entry,
        locked: bool,
        height: Option<f32>,
    ) -> (Option<Element<'a, Message>>, Option<Element<'a, Message>>) {
        let scrolled = |content: Element<'a, Message>| -> Element<'a, Message> {
            match height {
                // Infinite: as high as the window gives (a resized window, docs/adr/0746).
                Some(h) if h.is_infinite() => scrollable(container(content).padding(SCROLLED))
                    .height(Fill)
                    .into(),
                Some(h) => scrollable(container(content).padding(SCROLLED))
                    .height(typography::scaled(h))
                    .into(),
                None => content,
            }
        };
        let tabbed = matches!(e.body.items.first(), Some(FormItem::Tab { .. }));
        if !tabbed {
            return (
                None,
                Some(scrolled(self.entry_block(e, &e.body.items, locked))),
            );
        }
        let shown: Vec<&FormItem> = e
            .body
            .items
            .iter()
            .filter(|t| e.state(t.id()).is_some_and(|s| s.shown))
            .collect();
        let tabs: Vec<Tab<'_>> = shown
            .iter()
            .map(|t| {
                let label = match t {
                    FormItem::Tab { id, label, .. } => {
                        e.words_of(id, Some(label)).unwrap_or_default()
                    }
                    _ => String::new(),
                };
                let count = if e.tried {
                    e.issues
                        .iter()
                        .filter(|i| {
                            i.item
                                .as_deref()
                                .is_some_and(|id| kentos_forms_find(t.items(), id))
                        })
                        .count()
                } else {
                    0
                };
                Tab::new(label).badge(count).closable(false)
            })
            .collect();
        // A step form's tabs are numbered (docs/adr/0437).
        let tabs: Vec<Tab<'_>> = if e.body.steps {
            tabs.into_iter()
                .zip(shown.iter())
                .enumerate()
                .map(|(k, (_, t))| {
                    let label = match t {
                        FormItem::Tab { id, label, .. } => {
                            e.words_of(id, Some(label)).unwrap_or_default()
                        }
                        _ => String::new(),
                    };
                    Tab::new(format!("{}. {label}", k + 1)).closable(false)
                })
                .collect()
        } else {
            tabs
        };
        let page = shown
            .get(e.tab.min(shown.len().saturating_sub(1)))
            .map(|tab| scrolled(self.entry_block(e, tab.items(), locked)));
        (
            Some(Tabs::new(tabs, e.tab, |i| e.msg(Event::Tab(i))).into()),
            page,
        )
    }

    /// The designer's preview of its copy (docs/adr/0403 §3): what the record is, the pages, and
    /// what is wrong with it; nothing here writes.
    pub(crate) fn entry_preview<'a>(&'a self, e: &'a Entry) -> Element<'a, Message> {
        let head = if e.uid.is_empty() {
            format!("Yeni kayıt · {}", e.screen.object.layer)
        } else {
            format!(
                "Seçili nesne · {} · {}",
                e.screen.object.kind, e.screen.object.layer
            )
        };
        let mut body = Column::new()
            .spacing(10)
            .push(label::caption(head).style(muted));
        body = self.entry_pages(e, false, body, None);
        let errors = e.errors().count();
        let warnings = e.issues.len() - errors;
        let mut summary = Vec::new();
        if errors > 0 {
            summary.push(words::text_line(Kind::Error, format!("{errors} sorun")));
        }
        if warnings > 0 {
            summary.push(words::text_line(Kind::Warn, format!("{warnings} uyarı")));
        }
        if summary.is_empty() {
            summary.push(words::text_line(
                Kind::Ok,
                "Kayıt formun kurallarına uyuyor.",
            ));
        }
        body.push(Row::with_children(summary).spacing(12)).into()
    }

    /// The window's controls by their words (a trace's `dialog` step): a field by its label.
    pub(crate) fn form_entry_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(e) = &self.gis.form_entry else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(FIRST_ISSUE | NEXT_ISSUE) => Some(msg(Event::FirstIssue)),
            Control::Press(DISCARD) => Some(msg(Event::Discard)),
            Control::Press(KEEP_EDITING) => Some(msg(Event::KeepEditing)),
            Control::Press(SAVE_AND_CLOSE) => Some(msg(Event::Save)),
            Control::Fill(words, text) => {
                let key = e
                    .fields
                    .iter()
                    .find(|f| f.label() == words || f.name == words)
                    .map(|f| f.key().to_owned())
                    .ok_or_else(|| format!("“{TITLE}” penceresinde “{words}” alanı yok"))?;
                Some(msg(Event::Value(key, text.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
