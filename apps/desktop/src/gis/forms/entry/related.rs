//! Related records (docs/adr/0404, 0440): linking, adding, opening and deleting them from a record; their
//! box, offers and footer; the list as rows, a table, bars or cards.

use super::*;
use crate::gis;

impl App {
    /// A related item's object linked to the record (its field takes the record's id) or unlinked (its
    /// field emptied), through `cad.entities.set` (the object's own form's rules hold); the window's
    /// lists follow, its values stay.
    pub(super) fn link_related(&mut self, item: &str, uid: &str, link: bool) -> Task<Message> {
        let action = if link {
            RelatedAction::Link
        } else {
            RelatedAction::Unlink
        };
        if let Some(why) = self.related_refusal(item, action) {
            self.say(Level::Warn, why);
            return Task::none();
        }
        let Some(e) = self.gis.form_entry.as_ref() else {
            return Task::none();
        };
        let Some(FormItem::Related { layer, field, .. }) =
            crate::forms::designer::find(&e.body.items, item).cloned()
        else {
            return Task::none();
        };
        let record = e.uid.clone();
        // Through a project relation, the link waits for Kaydet with the record (docs/adr/0440 §2).
        let relation = self.document.as_ref().and_then(|d| {
            let form_layer = d
                .model
                .settings()
                .forms
                .iter()
                .find(|f| f.id == e.form)
                .map(|f| f.layer.clone())?;
            d.model
                .settings()
                .relations
                .iter()
                .find(|r| r.parent == form_layer && r.child == layer && r.field == field)
                .map(|r| r.id.clone())
        });
        if let Some(relation) = relation {
            if let Some(e) = self.gis.form_entry.as_mut() {
                let undone = e
                    .links
                    .iter()
                    .position(|(i, c)| *i == item && c.uid == uid && c.unlink == link);
                match undone {
                    // Linking what was unlinked here (or the other way) takes the pending one back.
                    Some(at) => {
                        e.links.remove(at);
                    }
                    None => e.links.push((
                        item.to_owned(),
                        kentos_commands::FormChild {
                            relation,
                            uid: uid.to_owned(),
                            values: BTreeMap::new(),
                            unlink: !link,
                        },
                    )),
                }
            }
            // The bounds count the pending link now (docs/adr/0444 §1).
            if let (Some(e), Some(doc)) = (self.gis.form_entry.as_mut(), self.document.as_ref()) {
                e.recompute(&doc.model);
            }
            self.say(
                Level::Info,
                if link {
                    "Nesne bağlanacak; Kaydet ile kayıtla birlikte yazılır."
                } else {
                    "Nesnenin bağı koparılacak; Kaydet ile kayıtla birlikte yazılır."
                },
            );
            return Task::none();
        }
        let Some((name, many)) = self
            .layer_fields_of(&layer)
            .iter()
            .find(|f| f.key() == field)
            .map(|f| (f.name.clone(), f.multiple))
        else {
            return Task::none();
        };
        // A many-to-many field keeps its other ids: the record's is added or taken off
        // (docs/adr/0409 §1).
        let held: Vec<String> = if many {
            self.document
                .as_ref()
                .and_then(|d| {
                    let slot = d.model.slot_of(kentos_domain::Uuid::parse_str(uid).ok()?)?;
                    d.model.get(slot)?.base().attrs.get(&name).cloned()
                })
                .map(|v| {
                    v.split(';')
                        .map(|h| h.trim().to_owned())
                        .filter(|h| !h.is_empty() && *h != record)
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let value = match (many, link) {
            (true, true) => Some(
                [held, vec![record]]
                    .concat()
                    .join(kentos_commands::MULTIPLE_JOIN),
            ),
            (true, false) => (!held.is_empty()).then(|| held.join(kentos_commands::MULTIPLE_JOIN)),
            (false, true) => Some(record),
            (false, false) => None,
        };
        let input = serde_json::json!({
            "uids": [uid],
            "attrs": { name: value },
            "operation": "attributes",
        });
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let answer = kentos_native_application::registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            kentos_commands::CAD_ENTITIES_SET,
            kentos_commands::CallOp::Execute,
            input,
        );
        let refused = match answer {
            Ok(v) if v["status"] == "completed" => None,
            Ok(v) => Some(v["error"]["message"].as_str().unwrap_or("").to_owned()),
            Err(e) => Some(format!("{e:?}")),
        };
        if let Some(why) = refused {
            self.say(Level::Warn, why);
            return Task::none();
        }
        self.say(
            Level::Success,
            if link {
                "Nesne kayda bağlandı."
            } else {
                "Nesnenin kayda bağı koparıldı."
            },
        );
        self.refresh_entry()
    }

    /// Yeni of a related item (docs/adr/0409 §3): a point of the other layer at the record's place,
    /// its field holding the record's id (`cad.point.create`, the child's own form's rules too), then
    /// opened by its layer's form.
    pub(super) fn new_related(&mut self, item: &str) -> Task<Message> {
        if let Some(why) = self.related_refusal(item, RelatedAction::Create) {
            self.say(Level::Warn, why);
            return Task::none();
        }
        let Some(e) = self.gis.form_entry.as_ref() else {
            return Task::none();
        };
        let Some(FormItem::Related { layer, field, .. }) =
            crate::forms::designer::find(&e.body.items, item).cloned()
        else {
            return Task::none();
        };
        let anchor = e.screen.object.measured.anchor;
        let record = e.uid.clone();
        let Some(name) = self
            .layer_fields_of(&layer)
            .iter()
            .find(|f| f.key() == field)
            .map(|f| f.name.clone())
        else {
            return Task::none();
        };
        // A record without a geometry (docs/adr/0410 §4) when the child layer holds records, or the
        // record itself has no place; else a point at the record's place.
        let (east, north) = match anchor {
            Some(at) if !self.layer_holds_records(&layer) => at,
            _ => {
                let attrs = [(name, record)].into_iter().collect();
                let Some(slot) = self.add_record(&layer, attrs) else {
                    return Task::none();
                };
                self.say(Level::Success, "Alt kayıt oluşturuldu ve kayda bağlandı.");
                if let Some(form) = self.form_of_layer(&layer) {
                    self.selection.set([slot]);
                    return self.open_entry(slot, &form);
                }
                return self.refresh_entry();
            }
        };
        let input = serde_json::json!({
            "p": { "x": east, "y": north },
            "layerId": layer,
            "attrs": { name: record },
        });
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let answer = kentos_native_application::registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            kentos_commands::CAD_POINT_CREATE,
            kentos_commands::CallOp::Execute,
            input,
        );
        let made = match answer {
            Ok(v) if v["status"] == "completed" => v["output"]["uid"]
                .as_str()
                .and_then(|u| kentos_domain::Uuid::parse_str(u).ok())
                .and_then(|u| doc.model.slot_of(u)),
            Ok(v) => {
                let why = v["error"]["message"].as_str().unwrap_or("").to_owned();
                self.say(Level::Warn, format!("Alt kayıt oluşturulamadı: {why}"));
                return Task::none();
            }
            Err(e) => {
                self.say(Level::Warn, format!("Alt kayıt oluşturulamadı: {e:?}"));
                return Task::none();
            }
        };
        let Some(slot) = made else {
            return Task::none();
        };
        self.say(Level::Success, "Alt kayıt oluşturuldu ve kayda bağlandı.");
        // The child's own form, else the record shows its new child.
        if let Some(form) = self.form_of_layer(&layer) {
            self.selection.set([slot]);
            return self.open_entry(slot, &form);
        }
        self.refresh_entry()
    }

    /// The window's record prepared again (another object changed): the values typed stay, the
    /// drawing's revision is the new one.
    pub(crate) fn refresh_entry(&mut self) -> Task<Message> {
        let Some(e) = self.gis.form_entry.as_ref() else {
            return Task::none();
        };
        let (form, uid, draft) = (e.form.clone(), e.uid.clone(), e.draft);
        let (moment, user) = (e.moment.clone(), e.user.clone());
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Ok(screen) = record_screen(
            &doc.model,
            &form,
            &uid,
            draft,
            moment.as_deref(),
            user.as_deref(),
        ) else {
            return Task::none();
        };
        let revision = doc.model.revision().to_string();
        if let (Some(e), Some(doc)) = (self.gis.form_entry.as_mut(), self.document.as_ref()) {
            // The record as the drawing holds it now (a related record's change worked its calculations
            // out again, docs/adr/0761): what the typed values go onto.
            if !e.preview && !e.new_record && e.members.is_empty() {
                e.before = screen.attrs.clone();
            }
            e.screen = screen;
            e.revision = revision;
            e.unique_asked.clear();
            e.recompute(&doc.model);
        }
        Task::none()
    }

    /// The form closed: the record's form under it, if one waits, comes back with its draft and its
    /// related objects read anew (docs/adr/0440 §7); else the window closes.
    pub(crate) fn entry_closed(&mut self) -> Task<Message> {
        self.gis.form_entry = self.gis.form_stack.pop();
        if self.gis.form_entry.is_none() {
            self.dialog = None;
            return Task::none();
        }
        let task = self.refresh_entry();
        if let (Some(e), Some(doc)) = (self.gis.form_entry.as_mut(), self.document.as_ref()) {
            e.recompute(&doc.model);
        }
        task
    }

    /// A related object's form over the record's (docs/adr/0440 §7): the record's form waits with its
    /// draft and comes back when the child's closes; at most [`MOST_NESTED`] forms deep.
    pub(super) fn open_related(&mut self, item: &str, uid: &str) -> Task<Message> {
        // Read-only when the item offers reading its children and not changing them, or the user
        // may not change them (docs/adr/0762 §1, §2).
        let read_only = self.gis.form_entry.as_ref().is_some_and(|e| {
            !self
                .related_offers(e, item)
                .iter()
                .any(|o| o.action == RelatedAction::Update && o.allowed())
        });
        if self.gis.form_stack.len() + 1 >= MOST_NESTED {
            self.say(
                Level::Warn,
                format!(
                    "En çok {MOST_NESTED} form iç içe açılır; önce açık formlardan birini kapatın."
                ),
            );
            return Task::none();
        }
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Some(slot) = kentos_domain::Uuid::parse_str(uid)
            .ok()
            .and_then(|u| doc.model.slot_of(u))
        else {
            return Task::none();
        };
        let Some(layer) = doc.model.get(slot).map(|e| e.base().layer_id.clone()) else {
            return Task::none();
        };
        match self.form_of_layer(&layer) {
            Some(form) => {
                if let Some(parent) = self.gis.form_entry.take() {
                    self.gis.form_stack.push(parent);
                }
                self.selection.set([slot]);
                let task = self.open_entry(slot, &form);
                if let Some(e) = self.gis.form_entry.as_mut() {
                    e.read_only = read_only;
                }
                task
            }
            None => {
                self.say(
                    Level::Warn,
                    "İlişkili nesnenin katmanının formu yok; Formlar penceresinden bir form oluşturun.",
                );
                Task::none()
            }
        }
    }

    /// A related item (docs/adr/0404 §4): the objects referring to the record, each opened by its own
    /// form or shown in the drawing (not from the designer's preview).
    pub(super) fn related_box<'a>(
        &'a self,
        e: &'a Entry,
        id: &str,
        title: Option<&str>,
    ) -> Element<'a, Message> {
        let uids = e.related_now(id);
        let mut col = Column::new().spacing(6).push(
            row![
                icon(Icon::Link).size(13.0),
                label::strong(title.unwrap_or("İlişkili kayıtlar").to_owned()),
                label::caption(format!("{}", uids.len())).style(muted),
            ]
            .spacing(6)
            .align_y(Center),
        );
        // Its bounds' issue under its title, before a long list (docs/adr/0444 §1).
        if let Some(issue) = e.issues.iter().find(|i| i.item.as_deref() == Some(id)) {
            col = col.push(words::text_line(Kind::Error, issue.message.clone()));
        }
        if uids.is_empty() {
            col = col.push(label::caption("Bu kayda başvuran nesne yok.").style(muted));
        }
        let doc = self.document.as_ref();
        let live = !e.preview && !e.uid.is_empty();
        let offers = self.related_offers(e, id);
        // The other layer's objects not yet linked to the record, to link one (docs/adr/0405 §4).
        let candidates: Vec<(String, String)> =
            match (doc, crate::forms::designer::find(&e.body.items, id)) {
                (Some(d), Some(FormItem::Related { layer, field, .. })) if live => {
                    let node = d.model.layers().get(layer);
                    let name = node
                        .and_then(|n| n.fields.iter().find(|f| f.key() == field))
                        .map(|f| f.name.clone())
                        .unwrap_or_default();
                    d.model
                        .by_layer_with_uids(layer)
                        .filter(|(u, x)| {
                            !uids.contains(&u.to_string())
                                && x.base().attrs.get(&name).is_none_or(|v| v.trim() != e.uid)
                        })
                        .take(200)
                        .map(|(u, x)| {
                            let first = node.and_then(|n| {
                                n.fields.iter().find_map(|f| {
                                    x.base().attrs.get(&f.name).filter(|v| !v.is_empty())
                                })
                            });
                            let words = x
                                .base()
                                .label
                                .clone()
                                .or_else(|| first.cloned())
                                .unwrap_or_else(|| u.to_string().chars().take(8).collect());
                            (u.to_string(), words)
                        })
                        .collect()
                }
                _ => Vec::new(),
            };
        // A table of the other layer's fields, its numbers totalled (docs/adr/0425), or a card for each
        // (docs/adr/0440 §6), a page at a time.
        // A bar for each value of the first column (docs/adr/0441 §3).
        if let (
            Some(d),
            Some(FormItem::Related {
                layer,
                field,
                view: kentos_commands::RelatedView::Chart,
                columns,
                ..
            }),
        ) = (doc, crate::forms::designer::find(&e.body.items, id))
        {
            col = col.push(related_chart(d, layer, field, columns, &uids));
            return self.related_footer(e, id, col, live, candidates);
        }
        if let (
            Some(d),
            Some(FormItem::Related {
                layer,
                field,
                view:
                    view @ (kentos_commands::RelatedView::Table | kentos_commands::RelatedView::Cards),
                columns,
                ..
            }),
        ) = (doc, crate::forms::designer::find(&e.body.items, id))
        {
            let page = e
                .related_pages
                .get(id)
                .copied()
                .unwrap_or(0)
                .min(uids.len().saturating_sub(1) / RELATED_PAGE);
            let shown_uids: Vec<String> = uids
                .iter()
                .skip(page * RELATED_PAGE)
                .take(RELATED_PAGE)
                .cloned()
                .collect();
            col = col.push(if *view == kentos_commands::RelatedView::Cards {
                related_cards(d, layer, field, columns, &shown_uids, e, id, live, &offers)
            } else {
                related_table(
                    d,
                    layer,
                    field,
                    columns,
                    &uids,
                    &shown_uids,
                    e,
                    id,
                    live,
                    &offers,
                )
            });
            if uids.len() > RELATED_PAGE {
                let pages = uids.len().div_ceil(RELATED_PAGE);
                let first = page * RELATED_PAGE + 1;
                let last = (first + RELATED_PAGE - 1).min(uids.len());
                let go = |to: usize, glyph: Icon| {
                    iced::widget::button(icon(glyph).size(12.0))
                        .padding(kentos_ui::theme::metrics::icon_small(12.0))
                        .style(style::button::tool(false))
                        .on_press_maybe(
                            (to != page && to < pages)
                                .then(|| e.msg(Event::RelatedPage(id.to_owned(), to))),
                        )
                };
                col = col.push(
                    row![
                        go(page.saturating_sub(1), Icon::ChevronLeft),
                        label::caption(format!("{first}–{last} / {}", uids.len())).style(muted),
                        go(page + 1, Icon::ChevronRight),
                    ]
                    .spacing(6)
                    .align_y(Center),
                );
            }
            return self.related_footer(e, id, col, live, candidates);
        }
        for uid in uids.iter().take(50) {
            let words = doc
                .and_then(|d| {
                    let slot = d.model.slot_of(kentos_domain::Uuid::parse_str(uid).ok()?)?;
                    let e = d.model.get(slot)?;
                    // Its label, else its first field's value, else its kind and id.
                    let first = d
                        .model
                        .layers()
                        .get(&e.base().layer_id)
                        .and_then(|n| {
                            n.fields
                                .iter()
                                .find_map(|f| e.base().attrs.get(&f.name).filter(|v| !v.is_empty()))
                        })
                        .cloned();
                    Some(e.base().label.clone().or(first).unwrap_or_else(|| {
                        format!(
                            "{} {}",
                            kentos_forms::kind_label(e),
                            uid.chars().take(8).collect::<String>()
                        )
                    }))
                })
                .unwrap_or_else(|| uid.clone());
            col = col.push(related_row(e, id, uid, words, live, &offers));
        }
        if uids.len() > 50 {
            col = col.push(label::caption(format!("{} nesne daha.", uids.len() - 50)).style(muted));
        }
        self.related_footer(e, id, col, live, candidates)
    }

    /// What the related item `item` offers and, for each, whether the user may (docs/adr/0762 §2).
    pub(crate) fn related_offers(&self, e: &Entry, item: &str) -> Vec<kentos_forms::Offer> {
        let Some(FormItem::Related {
            layer,
            field,
            actions,
            ..
        }) = crate::forms::designer::find(&e.body.items, item)
        else {
            return Vec::new();
        };
        let doc = self.document.as_ref();
        let layer_name = |id: &str| {
            doc.and_then(|d| d.model.layers().get(id).map(|n| n.name.clone()))
                .unwrap_or_else(|| id.to_owned())
        };
        let record_locked = if e.read_only {
            Some("Kayıt yalnız okumaya açıldı; alt kayıtları da yazılamaz.".to_owned())
        } else if let Some(why) = self.forms_locked() {
            Some(why.to_owned())
        } else if doc.is_some_and(|d| d.model.layers().is_locked(&e.screen.layer)) {
            Some(format!(
                "“{}” katmanı kilitli; kayıt da alt kayıtları da yazılamaz.",
                layer_name(&e.screen.layer)
            ))
        } else {
            None
        };
        let attributes_only = doc.is_some_and(|d| {
            d.model.object_policy() == kentos_domain::ObjectPolicy::AttributesOnly
        });
        // A Veri girişi writes a child through the child layer's own form (docs/adr/0764 §4).
        let child = doc.filter(|_| attributes_only).map(|d| {
            let fields_of = |l: &str| {
                d.model
                    .layers()
                    .get(l)
                    .map_or(&[][..], |n| n.fields.as_slice())
            };
            let reach = kentos_forms::reach::Reach::new(
                &d.model.settings().forms,
                fields_of,
                &[],
                kentos_expression::Unit::METRE,
                kentos_forms::reach::Caller::through_forms(),
            );
            let name = fields_of(layer)
                .iter()
                .find(|f| f.key() == field)
                .map_or_else(|| field.clone(), |f| f.name.clone());
            kentos_forms::ChildEntry::of(&reach, layer, &layer_name(layer), &name)
        });
        let permits = kentos_forms::ChildPermits {
            saved: !e.preview && !e.uid.is_empty(),
            record_locked,
            child_locked: doc
                .is_some_and(|d| d.model.layers().is_locked(layer))
                .then(|| layer_name(layer)),
            no_writes: doc
                .and_then(|d| d.cloud_source())
                .is_some_and(|s| !crate::cloud::access_of(&s.info).0),
            attributes_only,
            child,
        };
        kentos_forms::related_offer(actions.as_deref(), &permits)
    }

    /// Why the related item `item` does not let its action be taken now: it does not offer it, or the
    /// user may not (docs/adr/0762 §2); none when it may.
    pub(super) fn related_refusal(&self, item: &str, action: RelatedAction) -> Option<String> {
        let e = self.gis.form_entry.as_ref()?;
        match self
            .related_offers(e, item)
            .into_iter()
            .find(|o| o.action == action)
        {
            None => Some(format!(
                "Bu ilişkili kayıtlar öğesi “{}” sunmuyor; formun tasarımında öğenin eylemlerine ekleyin.",
                action.label()
            )),
            Some(o) => o.refused,
        }
    }

    /// The child asked to be deleted, deleted (docs/adr/0762 §4): `cad.entities.delete`, its own undo
    /// step; the project relations' delete rules hold.
    pub(super) fn delete_related(&mut self) -> Task<Message> {
        let Some((item, uid)) = self.gis.form_entry.as_mut().and_then(|e| e.removing.take()) else {
            return Task::none();
        };
        if let Some(why) = self.related_refusal(&item, RelatedAction::Delete) {
            self.say(Level::Warn, why);
            return Task::none();
        }
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let answer = kentos_native_application::registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            kentos_commands::CAD_ENTITIES_DELETE,
            kentos_commands::CallOp::Execute,
            serde_json::json!({ "uids": [uid] }),
        );
        match answer {
            Ok(v) if v["status"] == "completed" => {
                self.say(Level::Success, "Alt kayıt silindi.");
                self.refresh_entry()
            }
            Ok(v) => {
                let why = v["error"]["message"].as_str().unwrap_or("").to_owned();
                self.say(Level::Warn, format!("Alt kayıt silinemedi: {why}"));
                Task::none()
            }
            Err(e) => {
                self.say(Level::Warn, format!("Alt kayıt silinemedi: {e:?}"));
                Task::none()
            }
        }
    }

    /// A related item's Yeni and Bağla, and its frame.
    pub(super) fn related_footer<'a>(
        &'a self,
        e: &'a Entry,
        id: &str,
        mut col: Column<'a, Message>,
        live: bool,
        candidates: Vec<(String, String)>,
    ) -> Element<'a, Message> {
        let offers = self.related_offers(e, id);
        let mut actions = Row::new().spacing(6).align_y(Center);
        if live
            && let Some(b) = offered_word(
                &offers,
                RelatedAction::Create,
                e.msg(Event::NewRelated(id.to_owned())),
                words::secondary,
            )
        {
            actions = actions.push(b);
        }
        let link = offers.iter().find(|o| o.action == RelatedAction::Link);
        if let Some(why) = link.and_then(|o| o.refused.clone())
            && live
        {
            actions = actions.push(kentos_ui::widget::tip(
                words::ghost("Bağla", None),
                kentos_ui::widget::Tip::new("Bağla").body(why),
                iced::widget::tooltip::Position::Top,
            ));
        } else if link.is_some() && !candidates.is_empty() {
            let item = id.to_owned();
            let picks = candidates.clone();
            actions = actions.push(
                container(
                    Select::new(
                        candidates.iter().map(|(_, w)| Choice::new(w.clone())),
                        None,
                        move |i| {
                            Message::Gis(gis::Event::FormEntry(Event::Link(
                                item.clone(),
                                picks[i].0.clone(),
                            )))
                        },
                    )
                    .placeholder("Bağla: bir nesne seçin"),
                )
                .width(Fill),
            );
        }
        col = col.push(actions);
        // The offered actions the user may not take, each reason once (docs/adr/0762 §2).
        if live {
            let mut said: Vec<&str> = Vec::new();
            for why in offers.iter().filter_map(|o| o.refused.as_deref()) {
                if !said.contains(&why) {
                    said.push(why);
                }
            }
            for why in said {
                col = col.push(label::caption(why.to_owned()).style(style::text::muted));
            }
        }
        container(col)
            .padding(10)
            .style(style::container::field_box)
            .width(Fill)
            .into()
    }
}

pub(super) fn muted(theme: &iced::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(Tokens::of(theme).muted),
    }
}

/// The related objects as a table (docs/adr/0425): the item's columns (the other layer's first four
/// fields when it names none), each object a row with Aç, Git and Bağı kopar; the decimal columns'
/// totals under them.
#[allow(clippy::too_many_arguments)]
/// The related action's button when the item offers it: pressed when the user may, else quiet with
/// the reason in its tip (docs/adr/0762 §2); none when it is not offered.
pub(super) fn offered_word<'a>(
    offers: &[kentos_forms::Offer],
    action: RelatedAction,
    on: Message,
    look: fn(&'a str, Option<Message>) -> Element<'a, Message>,
) -> Option<Element<'a, Message>> {
    let offer = offers.iter().find(|o| o.action == action)?;
    Some(match &offer.refused {
        None => look(action.label(), Some(on)),
        Some(why) => kentos_ui::widget::tip(
            look(action.label(), None),
            kentos_ui::widget::Tip::new(action.label()).body(why.clone()),
            iced::widget::tooltip::Position::Top,
        ),
    })
}

/// Aç: offered when the item offers reading or changing its children.
pub(super) fn open_offer(offers: &[kentos_forms::Offer]) -> Option<&kentos_forms::Offer> {
    offers
        .iter()
        .find(|o| o.action == RelatedAction::View)
        .or_else(|| offers.iter().find(|o| o.action == RelatedAction::Update))
}

/// A related child in the list: its words and the actions its item offers; Sil asks in the row.
pub(super) fn related_row<'a>(
    e: &'a Entry,
    id: &str,
    uid: &str,
    words_of: String,
    live: bool,
    offers: &[kentos_forms::Offer],
) -> Element<'a, Message> {
    let mut line = Row::new()
        .spacing(6)
        .align_y(Center)
        .push(label::body(words_of).width(Fill));
    if e.removing
        .as_ref()
        .is_some_and(|(i, u)| i == id && u == uid)
    {
        return line
            .push(label::caption("Silinsin mi?").style(style::text::muted))
            .push(
                iced::widget::button(label::body("Sil"))
                    .on_press(e.msg(Event::ConfirmDelete))
                    .padding(kentos_ui::theme::metrics::action())
                    .style(style::button::danger),
            )
            .push(words::ghost("Vazgeç", Some(e.msg(Event::KeepChild))))
            .into();
    }
    if !live {
        return line.into();
    }
    let pairs = [
        (RelatedAction::Show, Event::ShowObject(uid.to_owned())),
        (
            RelatedAction::Unlink,
            Event::Unlink(id.to_owned(), uid.to_owned()),
        ),
        (
            RelatedAction::Delete,
            Event::DeleteRelated(id.to_owned(), uid.to_owned()),
        ),
    ];
    for (action, event) in pairs {
        if let Some(b) = offered_word(offers, action, e.msg(event), words::ghost) {
            line = line.push(b);
        }
    }
    if let Some(open) = open_offer(offers) {
        let on = e.msg(Event::OpenRelated(id.to_owned(), uid.to_owned()));
        line = line.push(match &open.refused {
            None => words::secondary("Aç", Some(on)),
            Some(why) => kentos_ui::widget::tip(
                words::secondary("Aç", None),
                kentos_ui::widget::Tip::new("Aç").body(why.clone()),
                iced::widget::tooltip::Position::Top,
            ),
        });
    }
    line.into()
}

/// A related child's tools in a table row or card: the offered actions as icons, a refused one quiet
/// with its reason; Sil asks first.
pub(super) fn related_tools<'a>(
    e: &'a Entry,
    id: &str,
    uid: &str,
    live: bool,
    offers: &[kentos_forms::Offer],
) -> Row<'a, Message> {
    let mut tools = Row::new().spacing(2).align_y(Center);
    if e.removing
        .as_ref()
        .is_some_and(|(i, u)| i == id && u == uid)
    {
        return tools
            .push(
                iced::widget::button(label::caption("Sil"))
                    .on_press(e.msg(Event::ConfirmDelete))
                    .padding(kentos_ui::theme::metrics::action_small())
                    .style(style::button::danger),
            )
            .push(
                iced::widget::button(icon(Icon::Close).size(13.0))
                    .padding(kentos_ui::theme::metrics::icon_small(13.0))
                    .style(style::button::tool(false))
                    .on_press(e.msg(Event::KeepChild)),
            );
    }
    let tool =
        |glyph: Icon, tip: &'static str, offer: Option<&kentos_forms::Offer>, on: Message| {
            let offer = offer?;
            let allowed = live && offer.allowed();
            let card = match (&offer.refused, live) {
                (Some(why), true) => kentos_ui::widget::Tip::new(tip).body(why.clone()),
                _ => kentos_ui::widget::Tip::new(tip),
            };
            Some(kentos_ui::widget::tip(
                iced::widget::button(icon(glyph).size(13.0))
                    .padding(kentos_ui::theme::metrics::icon_small(13.0))
                    .style(style::button::tool(false))
                    .on_press_maybe(allowed.then_some(on)),
                card,
                iced::widget::tooltip::Position::Top,
            ))
        };
    let find = |a: RelatedAction| offers.iter().find(|o| o.action == a);
    let all = [
        tool(
            Icon::Open,
            "Aç",
            open_offer(offers),
            e.msg(Event::OpenRelated(id.to_owned(), uid.to_owned())),
        ),
        tool(
            Icon::Target,
            "Çizimde göster",
            find(RelatedAction::Show),
            e.msg(Event::ShowObject(uid.to_owned())),
        ),
        tool(
            Icon::Close,
            "Bağı kopar",
            find(RelatedAction::Unlink),
            e.msg(Event::Unlink(id.to_owned(), uid.to_owned())),
        ),
        tool(
            Icon::Eraser,
            "Sil",
            find(RelatedAction::Delete),
            e.msg(Event::DeleteRelated(id.to_owned(), uid.to_owned())),
        ),
    ];
    for t in all.into_iter().flatten() {
        tools = tools.push(t);
    }
    tools
}

#[allow(clippy::too_many_arguments)]
pub(super) fn related_table<'a>(
    d: &crate::document::Document,
    layer: &str,
    link: &str,
    columns: &[String],
    all: &[String],
    uids: &[String],
    e: &'a Entry,
    id: &str,
    live: bool,
    offers: &[kentos_forms::Offer],
) -> Element<'a, Message> {
    let fields: Vec<kentos_commands::LayerField> = d
        .model
        .layers()
        .get(layer)
        .map(|n| n.fields.clone())
        .unwrap_or_default();
    let shown: Vec<kentos_commands::LayerField> = if columns.is_empty() {
        fields
            .iter()
            .filter(|f| f.key() != link)
            .take(4)
            .cloned()
            .collect()
    } else {
        columns
            .iter()
            .filter_map(|c| fields.iter().find(|f| f.key() == c).cloned())
            .collect()
    };
    let numeric = |f: &kentos_commands::LayerField| {
        matches!(f.kind, LayerFieldKind::Integer | LayerFieldKind::Decimal)
    };
    let cell = |text: String, f: Option<&kentos_commands::LayerField>| -> Element<'a, Message> {
        let t = label::body(text);
        if f.is_some_and(numeric) {
            container(t)
                .width(Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .into()
        } else {
            container(t).width(Fill).into()
        }
    };
    let mut head = Row::new().spacing(8);
    for f in &shown {
        let words = label::caption(f.label().to_owned()).style(muted);
        head = head.push(if numeric(f) {
            container(words)
                .width(Fill)
                .align_x(iced::alignment::Horizontal::Right)
        } else {
            container(words).width(Fill)
        });
    }
    head = head.push(space::horizontal().width(112));
    let mut rows = Column::new().spacing(2).push(head);
    // A decimal is a measure and is totalled (an area, a share); an integer is often a number or a floor.
    let mut totals: Vec<Option<f64>> = shown
        .iter()
        .map(|f| (f.kind == LayerFieldKind::Decimal).then_some(0.0))
        .collect();
    // The totals are of every related object, not only the page's.
    for uid in all {
        let Some(x) = kentos_domain::Uuid::parse_str(uid)
            .ok()
            .and_then(|u| d.model.slot_of(u))
            .and_then(|s| d.model.get(s))
        else {
            continue;
        };
        for (i, f) in shown.iter().enumerate() {
            let raw = x.base().attrs.get(&f.name).map_or("", String::as_str);
            if let (Some(t), Ok(v)) = (totals[i].as_mut(), raw.trim().parse::<f64>()) {
                *t += v;
            }
        }
    }
    for uid in uids {
        let Some(x) = kentos_domain::Uuid::parse_str(uid)
            .ok()
            .and_then(|u| d.model.slot_of(u))
            .and_then(|s| d.model.get(s))
        else {
            continue;
        };
        let mut line = Row::new().spacing(8).align_y(Center);
        for f in &shown {
            let raw = x.base().attrs.get(&f.name).cloned().unwrap_or_default();
            line = line.push(cell(kentos_commands::display_value(f, &raw), Some(f)));
        }
        line = line.push(container(related_tools(e, id, uid, live, offers)).width(112));
        rows = rows.push(line);
    }
    if totals.iter().any(Option::is_some) && !uids.is_empty() {
        let mut foot = Row::new().spacing(8);
        for (i, f) in shown.iter().enumerate() {
            let text = match totals[i] {
                Some(t) => {
                    let scale = f.scale.unwrap_or(0) as usize;
                    format!("{t:.scale$}")
                }
                None if i == 0 => "Toplam".to_owned(),
                None => String::new(),
            };
            foot = foot.push(cell(
                text,
                Some(f).filter(|f| f.kind == LayerFieldKind::Decimal),
            ));
        }
        foot = foot.push(space::horizontal().width(112));
        rows = rows.push(container(foot).padding(iced::Padding {
            top: 4.0,
            ..iced::Padding::ZERO
        }));
    }
    rows.into()
}

/// The related objects as bars (docs/adr/0441 §3): each value of the first column (else the first field
/// that is not the link) with how many objects hold it, the most first; an empty value last.
pub(super) fn related_chart<'a>(
    d: &crate::document::Document,
    layer: &str,
    link: &str,
    columns: &[String],
    uids: &[String],
) -> Element<'a, Message> {
    let fields: Vec<kentos_commands::LayerField> = d
        .model
        .layers()
        .get(layer)
        .map(|n| n.fields.clone())
        .unwrap_or_default();
    let by = columns
        .first()
        .and_then(|c| fields.iter().find(|f| f.key() == c))
        .or_else(|| fields.iter().find(|f| f.key() != link))
        .cloned();
    let Some(by) = by else {
        return label::caption("Grafik için alt katmanın bir alanı gerekir.")
            .style(muted)
            .into();
    };
    let mut counts: Vec<(String, usize)> = Vec::new();
    for uid in uids {
        let Some(x) = kentos_domain::Uuid::parse_str(uid)
            .ok()
            .and_then(|u| d.model.slot_of(u))
            .and_then(|s| d.model.get(s))
        else {
            continue;
        };
        let v = x.base().attrs.get(&by.name).cloned().unwrap_or_default();
        match counts.iter_mut().find(|(k, _)| *k == v) {
            Some((_, n)) => *n += 1,
            None => counts.push((v, 1)),
        }
    }
    counts.sort_by(|a, b| {
        (a.0.is_empty(), std::cmp::Reverse(a.1), &a.0).cmp(&(
            b.0.is_empty(),
            std::cmp::Reverse(b.1),
            &b.0,
        ))
    });
    let most = counts.iter().map(|(_, n)| *n).max().unwrap_or(1).max(1) as f32;
    let mut rows = Column::new()
        .spacing(4)
        .push(label::caption(format!("{} sayısı", by.label())).style(muted));
    for (value, n) in counts {
        let words = if value.is_empty() {
            "(boş)".to_owned()
        } else {
            kentos_commands::display_value(&by, &value)
        };
        let share = n as f32 / most;
        rows = rows.push(
            row![
                container(label::caption(words)).width(140),
                kentos_ui::widget::progress::bar(Some(share))
                    .width(Fill)
                    .height(10.0),
                container(label::caption(n.to_string()))
                    .width(40)
                    .align_x(iced::alignment::Horizontal::Right),
            ]
            .spacing(8)
            .align_y(Center),
        );
    }
    rows.into()
}

/// The related objects as cards (docs/adr/0440 §6): each its columns' labels and values, as people
/// read them, with Aç, Çizimde göster and Bağı kopar.
#[allow(clippy::too_many_arguments)]
pub(super) fn related_cards<'a>(
    d: &crate::document::Document,
    layer: &str,
    link: &str,
    columns: &[String],
    uids: &[String],
    e: &'a Entry,
    id: &str,
    live: bool,
    offers: &[kentos_forms::Offer],
) -> Element<'a, Message> {
    let fields: Vec<kentos_commands::LayerField> = d
        .model
        .layers()
        .get(layer)
        .map(|n| n.fields.clone())
        .unwrap_or_default();
    let shown: Vec<kentos_commands::LayerField> = if columns.is_empty() {
        fields
            .iter()
            .filter(|f| f.key() != link)
            .take(4)
            .cloned()
            .collect()
    } else {
        columns
            .iter()
            .filter_map(|c| fields.iter().find(|f| f.key() == c).cloned())
            .collect()
    };
    let mut cards = Column::new().spacing(6);
    for uid in uids {
        let Some(x) = kentos_domain::Uuid::parse_str(uid)
            .ok()
            .and_then(|u| d.model.slot_of(u))
            .and_then(|s| d.model.get(s))
        else {
            continue;
        };
        let mut body = Column::new().spacing(2);
        for f in &shown {
            let raw = x.base().attrs.get(&f.name).cloned().unwrap_or_default();
            body = body.push(
                row![
                    container(label::caption(f.label().to_owned()).style(muted)).width(120),
                    label::body(kentos_commands::display_value(f, &raw)),
                ]
                .spacing(8),
            );
        }
        let tools = related_tools(e, id, uid, live, offers);
        cards = cards.push(
            container(
                row![container(body).width(Fill), tools]
                    .spacing(8)
                    .align_y(iced::alignment::Vertical::Top),
            )
            .padding(8)
            .width(Fill)
            .style(style::container::field_box),
        );
    }
    cards.into()
}

/// `codes` in the list's order (`list`), a held code the list no longer offers after them.
pub(super) fn ordered_codes(codes: &[&str], list: &[String]) -> String {
    let mut out: Vec<&str> = Vec::new();
    for c in codes {
        if !out.contains(c) {
            out.push(c);
        }
    }
    let rank = |c: &str| list.iter().position(|x| x == c).unwrap_or(usize::MAX);
    out.sort_by_key(|c| rank(c));
    out.join(kentos_commands::MULTIPLE_JOIN)
}

#[cfg(test)]
pub(crate) fn ordered_for_tests(codes: &[&str], list: &[&str]) -> String {
    let list: Vec<String> = list.iter().map(|s| (*s).to_owned()).collect();
    ordered_codes(codes, &list)
}
