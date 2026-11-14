//! Every event of the Formlar window (docs/adr/0403): the copy's items added, changed, moved and
//! removed; forms created, saved, published and removed; the draft kept on this device as it is edited.

use super::*;

impl App {
    pub(super) fn forms_step(&mut self, event: Event) -> Task<Message> {
        let Some(mut w) = self.gis.forms_window.clone() else {
            return Task::none();
        };
        let fields = w
            .chosen
            .as_deref()
            .and_then(|id| self.project_forms().iter().find(|f| f.id == id))
            .map(|f| self.layer_fields_of(&f.layer))
            .unwrap_or_default();
        let chosen_item = w.item.clone();
        let edited = |w: &mut Window, change: &dyn Fn(&mut FormBody)| {
            if let Some(body) = w.body.as_mut() {
                change(body);
                w.dirty = true;
            }
        };
        match event {
            Event::Choose(id) => {
                if w.dirty && w.chosen.as_deref() != Some(id.as_str()) {
                    self.say(
                        Level::Warn,
                        "Formdaki değişiklikler kaydedilmedi: önce Kaydet’e ya da Vazgeç’e basın.",
                    );
                    return Task::none();
                }
                self.choose_into(&mut w, &id);
            }
            Event::NewForm => {
                let layer = self
                    .document
                    .as_ref()
                    .map(|d| d.model.layers().active().to_owned());
                w.new_form = Some(NewForm {
                    name: String::new(),
                    layer,
                });
            }
            Event::NewName(text) => {
                if let Some(n) = w.new_form.as_mut() {
                    n.name = text;
                }
            }
            Event::NewLayer(layer) => {
                if let Some(n) = w.new_form.as_mut() {
                    n.layer = Some(layer);
                }
            }
            Event::CancelNew => w.new_form = None,
            Event::Create => {
                let Some(n) = w.new_form.clone() else {
                    return Task::none();
                };
                let Some(layer) = n.layer else {
                    self.warn("Formun katmanını seçin.");
                    return Task::none();
                };
                let name = if n.name.trim().is_empty() {
                    format!("{} formu", self.form_layer_name(&layer))
                } else {
                    n.name.clone()
                };
                if let Some(done) = self.edit_forms(FormsOp::Create {
                    id: None,
                    name,
                    layer,
                    body: None,
                }) && let Some(made) = done.form
                {
                    self.say(
                        Level::Success,
                        format!("“{}” formu katmanın alanlarından oluşturuldu.", made.name),
                    );
                    self.choose_into(&mut w, &made.id);
                }
            }
            Event::SelectItem(id) => w.item = Some(id),
            Event::AddTab => {
                let mut made = None;
                if let Some(body) = w.body.as_mut() {
                    made = Some(add_tab(body));
                    w.dirty = true;
                }
                w.item = made;
            }
            Event::AddGroup => {
                let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                let id = kentos_forms::item_id_of("grup", &taken);
                let item = FormItem::Group {
                    across: false,
                    id: id.clone(),
                    label: Some("Grup".into()),
                    columns: None,
                    visible_when: None,
                    collapsed: false,
                    items: Vec::new(),
                };
                edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                w.item = Some(id);
            }
            Event::AddNote => {
                let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                let id = kentos_forms::item_id_of("not", &taken);
                let item = FormItem::Note {
                    id: id.clone(),
                    text: "Açıklama".into(),
                };
                edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                w.item = Some(id);
            }
            Event::AddField(key) => {
                let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                let name = fields
                    .iter()
                    .find(|f| f.key() == key)
                    .map_or(key.as_str(), |f| f.name.as_str());
                let id = kentos_forms::item_id_of(name, &taken);
                let mut field = FormField::new(id.clone(), key.clone());
                field.required = fields.iter().any(|f| f.key() == key && f.required);
                let item = FormItem::Field(field);
                edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                w.item = Some(id);
            }
            Event::AddRelated => {
                let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                let id = kentos_forms::item_id_of("iliskili", &taken);
                let item = FormItem::Related {
                    min: None,
                    max: None,
                    id: id.clone(),
                    label: Some("İlişkili kayıtlar".into()),
                    layer: String::new(),
                    field: String::new(),
                    visible_when: None,
                    view: kentos_commands::RelatedView::List,
                    actions: None,
                    columns: Vec::new(),
                };
                edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                w.item = Some(id);
            }
            Event::AddReference => {
                let form_layer = self.chosen_form().map(|f| f.layer.clone());
                let relations = self
                    .document
                    .as_ref()
                    .map(|d| d.model.settings().relations.clone())
                    .unwrap_or_default();
                let held: Vec<String> = w
                    .body
                    .as_ref()
                    .map(|b| {
                        b.walk()
                            .into_iter()
                            .filter_map(|(_, i)| match i {
                                FormItem::Field(f) => Some(f.field.clone()),
                                _ => None,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let Some(r) = relations
                    .iter()
                    .find(|r| Some(&r.child) == form_layer.as_ref() && !held.contains(&r.field))
                    .or_else(|| {
                        relations
                            .iter()
                            .find(|r| Some(&r.child) == form_layer.as_ref())
                    })
                    .cloned()
                else {
                    self.warn("Bu formun katmanını alt katman sayan bir ilişki yok; önce İlişkiler penceresinden bir ilişki kurun.");
                    return Task::none();
                };
                let label = self
                    .layer_fields_of(&r.parent)
                    .iter()
                    .find(|f| f.kind == kentos_commands::LayerFieldKind::Text)
                    .map(|f| f.key().to_owned());
                let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                let id = kentos_forms::item_id_of(&r.field, &taken);
                let mut field = FormField::new(id.clone(), r.field.clone());
                field.label = Some(r.name.clone());
                field.lookup = Some(kentos_commands::FormLookup {
                    layer: r.parent.clone(),
                    key: kentos_commands::LOOKUP_UID.into(),
                    label,
                    ..kentos_commands::FormLookup::default()
                });
                let item = FormItem::Field(field);
                edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                w.item = Some(id);
            }
            Event::RelatedFrom(relation) => {
                let r = self.document.as_ref().and_then(|d| {
                    d.model
                        .settings()
                        .relations
                        .iter()
                        .find(|r| r.id == relation)
                        .cloned()
                });
                if let (Some(id), Some(r)) = (chosen_item.clone(), r) {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Related {
                            layer,
                            field,
                            label,
                            ..
                        }) = find_mut(&mut b.items, &id)
                        {
                            *layer = r.child.clone();
                            *field = r.field.clone();
                            if label.as_deref().is_none_or(|l| l == "İlişkili kayıtlar") {
                                *label = Some(r.name.clone());
                            }
                        }
                    });
                }
            }
            Event::Palette(p) => w.palette = p,
            Event::DropPublish => {}
            Event::OpenDomains => {
                self.gis.forms_window = Some(w);
                return self.open_domains();
            }
            Event::AddRelationItem(relation) => {
                let form_layer = self.chosen_form().map(|f| f.layer.clone());
                let r = self.document.as_ref().and_then(|d| {
                    d.model
                        .settings()
                        .relations
                        .iter()
                        .find(|r| r.id == relation)
                        .cloned()
                });
                let Some(r) = r else {
                    return Task::none();
                };
                let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                if Some(&r.parent) == form_layer.as_ref() {
                    let id = kentos_forms::item_id_of(&r.id, &taken);
                    let item = FormItem::Related {
                        min: None,
                        max: None,
                        id: id.clone(),
                        label: Some(r.name.clone()),
                        layer: r.child.clone(),
                        field: r.field.clone(),
                        visible_when: None,
                        view: kentos_commands::RelatedView::List,
                        actions: None,
                        columns: Vec::new(),
                    };
                    edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                    w.item = Some(id);
                } else if Some(&r.child) == form_layer.as_ref() {
                    let label = self
                        .layer_fields_of(&r.parent)
                        .iter()
                        .find(|f| f.kind == kentos_commands::LayerFieldKind::Text)
                        .map(|f| f.key().to_owned());
                    let id = kentos_forms::item_id_of(&r.field, &taken);
                    let mut field = FormField::new(id.clone(), r.field.clone());
                    field.label = Some(r.name.clone());
                    field.lookup = Some(kentos_commands::FormLookup {
                        layer: r.parent.clone(),
                        key: kentos_commands::LOOKUP_UID.into(),
                        label,
                        ..kentos_commands::FormLookup::default()
                    });
                    let item = FormItem::Field(field);
                    edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                    w.item = Some(id);
                }
            }
            Event::SectionName(text) => w.section_name = text,
            Event::SaveSection => {
                let item = chosen_item
                    .as_deref()
                    .and_then(|id| w.body.as_ref().and_then(|b| find(&b.items, id)))
                    .cloned();
                let Some(item) = item else {
                    self.warn("Bölüm kitaplığına koymak için bir öğe seçin.");
                    self.gis.forms_window = Some(w);
                    return Task::none();
                };
                let name = if w.section_name.trim().is_empty() {
                    crate::forms::designer_view::item_words(&item)
                } else {
                    w.section_name.trim().to_owned()
                };
                let saved = self.form_moment().0.unwrap_or_default();
                match self
                    .gis
                    .form_files
                    .save_section(&name, &item, &fields, saved)
                {
                    Ok(v) => {
                        self.say(
                            Level::Success,
                            format!("“{name}” bölüm kitaplığına {v}. sürüm olarak kondu."),
                        );
                        w.section_name.clear();
                        w.palette = Palette::Sections;
                    }
                    Err(why) => self.warn(format!("Bölüm kaydedilemedi: {why}")),
                }
            }
            Event::InsertSection(i) => {
                let section = self.gis.form_files.sections().into_iter().nth(i);
                if let (Some(s), Some(body)) = (section, w.body.as_ref()) {
                    let missing: Vec<String> = crate::forms::library::bindings(&s, &fields)
                        .into_iter()
                        .filter(|(_, to)| to.is_none())
                        .map(|(from, _)| from)
                        .collect();
                    match pasted(body, s.item.clone(), &s.fields, &fields) {
                        Ok(item) => {
                            let id = item.id().to_owned();
                            let tab = matches!(item, FormItem::Tab { .. });
                            edited(&mut w, &|b| {
                                if tab {
                                    b.items.push(item.clone());
                                } else {
                                    place(b, chosen_item.as_deref(), item.clone());
                                }
                            });
                            w.item = Some(id);
                            if missing.is_empty() {
                                self.say(
                                    Level::Success,
                                    format!(
                                        "“{}” ({}. sürüm) eklendi; alanları bağlandı.",
                                        s.name, s.version
                                    ),
                                );
                            } else {
                                self.say(
                                    Level::Warn,
                                    format!(
                                        "“{}” ({}. sürüm) eklendi; katmanda olmayan alanlar: {}. Ön denetim onları gösterir.",
                                        s.name,
                                        s.version,
                                        missing.join(", ")
                                    ),
                                );
                            }
                        }
                        Err(why) => self.warn(why),
                    }
                }
            }
            Event::CommentText(text) => w.comment_text = text,
            Event::AddComment => {
                let Some(form) = w.chosen.clone() else {
                    return Task::none();
                };
                if w.comment_text.trim().is_empty() {
                    self.gis.forms_window = Some(w);
                    return Task::none();
                }
                let (at, author) = self.form_moment();
                let item = chosen_item.clone().filter(|id| {
                    self.chosen_form()
                        .is_some_and(|f| find(&f.draft.items, id).is_some())
                });
                let text = std::mem::take(&mut w.comment_text);
                self.gis.forms_window = Some(w);
                if self
                    .edit_forms(FormsOp::Comment {
                        id: form,
                        item,
                        text,
                        author,
                        at,
                    })
                    .is_some()
                {
                    self.say(Level::Success, "Yorum forma eklendi.");
                }
                return Task::none();
            }
            Event::ResolveComment(comment, resolved) => {
                let Some(form) = w.chosen.clone() else {
                    return Task::none();
                };
                self.gis.forms_window = Some(w);
                self.edit_forms(FormsOp::Resolve {
                    id: form,
                    comment,
                    resolved,
                });
                return Task::none();
            }
            Event::RestoreDraft => {
                if let Some(k) = w.kept.take() {
                    w.undo.push(w.body.clone().unwrap_or_default());
                    w.body = Some(k.body);
                    w.name = k.name;
                    w.dirty = true;
                    self.say(
                        Level::Info,
                        "Bu cihazda kalan taslak geri yüklendi; kaydetmek için Kaydet’e basın.",
                    );
                }
            }
            Event::DropKept => {
                if let (Some(_), Some(form)) = (w.kept.take(), w.chosen.as_deref()) {
                    self.gis
                        .form_files
                        .drop_draft(&self.form_project_key(), form);
                }
            }
            Event::AddDivider => {
                let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                let id = kentos_forms::item_id_of("bolucu", &taken);
                let item = FormItem::Divider {
                    id: id.clone(),
                    label: None,
                    visible_when: None,
                };
                edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                w.item = Some(id);
            }
            Event::AddAction => {
                let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                let id = kentos_forms::item_id_of("eylem", &taken);
                let item = FormItem::Action {
                    id: id.clone(),
                    label: "Öznitelik hesapla".into(),
                    kind: kentos_commands::ActionKind::Tool,
                    tool: Some("attributes.calculate".into()),
                    step: None,
                    runs: kentos_commands::ActionRuns::Auto,
                    pin: None,
                    visible_when: None,
                };
                edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                w.item = Some(id);
            }
            // An extension's tool is pinned to the extension installed now (docs/adr/8057 §12); choosing it
            // again pins it again.
            Event::PinnedTool(tool) => {
                if let Some(id) = chosen_item.clone() {
                    let pinned = self.tool_pin_of(&tool);
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Action { tool: t, pin, .. }) =
                            find_mut(&mut b.items, &id)
                        {
                            *t = Some(tool.clone());
                            *pin = pinned.clone();
                        }
                    });
                }
            }
            // An action's kind: its target follows it (docs/adr/0442 §5).
            Event::ActionKind(kind) => {
                if let Some(id) = chosen_item.clone() {
                    let first_step = w
                        .body
                        .as_ref()
                        .and_then(|b| b.workflow.as_ref())
                        .and_then(|f| f.steps.first())
                        .map(|s| s.label.clone());
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Action {
                            kind: k,
                            tool,
                            step,
                            runs,
                            pin,
                            ..
                        }) = find_mut(&mut b.items, &id)
                        {
                            *k = kind;
                            *tool = (kind == kentos_commands::ActionKind::Tool).then(|| {
                                tool.clone()
                                    .unwrap_or_else(|| "attributes.calculate".into())
                            });
                            *step = (kind == kentos_commands::ActionKind::Step)
                                .then(|| step.clone().or_else(|| first_step.clone()))
                                .flatten();
                            if kind != kentos_commands::ActionKind::Tool {
                                *runs = kentos_commands::ActionRuns::Auto;
                                *pin = None;
                            }
                        }
                    });
                }
            }
            Event::ActionStep(label) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Action { step, .. }) = find_mut(&mut b.items, &id) {
                            *step = Some(label.clone());
                        }
                    });
                }
            }
            Event::ActionRuns(where_) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Action { runs, .. }) = find_mut(&mut b.items, &id) {
                            *runs = where_;
                        }
                    });
                }
            }
            Event::AddAttachments => {
                let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                let id = kentos_forms::item_id_of("ekler", &taken);
                let item = FormItem::Attachments {
                    id: id.clone(),
                    label: Some("Ekler".into()),
                    accept: kentos_commands::AttachmentAccept::Any,
                    min: None,
                    max: None,
                    visible_when: None,
                };
                edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                w.item = Some(id);
            }
            Event::AddLocation => {
                let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                let id = kentos_forms::item_id_of("konum", &taken);
                let item = FormItem::Location {
                    id: id.clone(),
                    label: Some("Konum".into()),
                    visible_when: None,
                };
                edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                w.item = Some(id);
            }
            Event::Accept(to) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Attachments { accept, .. }) =
                            find_mut(&mut b.items, &id)
                        {
                            *accept = to;
                        }
                    });
                }
            }
            Event::CopyItem => {
                if let Some(item) = chosen_item
                    .as_deref()
                    .and_then(|id| w.body.as_ref().and_then(|b| find(&b.items, id)))
                {
                    self.form_clipboard = Some((item.clone(), fields.clone()));
                    self.say(
                        Level::Info,
                        "Öğe kopyalandı; bu ya da başka bir formda Yapıştır'a basın.",
                    );
                }
            }
            Event::PasteItem => {
                if let Some((item, from)) = self.form_clipboard.clone()
                    && let Some(body) = w.body.as_ref()
                {
                    match pasted(body, item, &from, &fields) {
                        Ok(item) => {
                            let id = item.id().to_owned();
                            let tab = matches!(item, FormItem::Tab { .. });
                            edited(&mut w, &|b| {
                                if tab {
                                    b.items.push(item.clone());
                                } else {
                                    place(b, chosen_item.as_deref(), item.clone());
                                }
                            });
                            w.item = Some(id);
                        }
                        Err(why) => self.warn(why),
                    }
                }
            }
            Event::AddMirror => {
                if let Some(id) = chosen_item.clone() {
                    let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                    let source = w.body.as_ref().and_then(|b| match find(&b.items, &id) {
                        Some(FormItem::Field(f)) if !f.mirror => Some(f.clone()),
                        _ => None,
                    });
                    if let Some(f) = source {
                        let mirror_id =
                            kentos_forms::item_id_of(&format!("{}-yansima", f.id), &taken);
                        let mut m =
                            kentos_commands::FormField::new(mirror_id.clone(), f.field.clone());
                        m.label = f.label.clone();
                        m.mirror = true;
                        let item = FormItem::Field(m);
                        edited(&mut w, &|b| place(b, Some(id.as_str()), item.clone()));
                        w.item = Some(mirror_id);
                    }
                }
            }
            Event::Suggest(on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.suggest = on;
                        }
                    });
                }
            }
            Event::LookupLayer(layer) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.lookup = layer.clone().map(|layer| kentos_commands::FormLookup {
                                layer,
                                key: kentos_commands::LOOKUP_UID.into(),
                                ..kentos_commands::FormLookup::default()
                            });
                        }
                    });
                }
            }
            Event::LookupKey(key) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(l) = f.lookup.as_mut()
                        {
                            l.key = key.clone();
                        }
                    });
                }
            }
            Event::LookupLabel(label) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(l) = f.lookup.as_mut()
                        {
                            l.label = label.clone();
                        }
                    });
                }
            }
            Event::Lookup(edit) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(l) = f.lookup.as_mut()
                        {
                            edit.apply(l);
                        }
                    });
                }
            }
            Event::MinSearch(text) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.min_search = text.trim().parse::<u32>().ok().filter(|n| *n > 0);
                        }
                    });
                }
            }
            Event::AddMatch => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(l) = f.lookup.as_mut()
                        {
                            l.matches.push(kentos_commands::LookupMatch {
                                field: String::new(),
                                record: String::new(),
                                fold: false,
                            });
                        }
                    });
                }
            }
            Event::MatchField(i, key) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(m) = f.lookup.as_mut().and_then(|l| l.matches.get_mut(i))
                        {
                            m.field = key.clone();
                        }
                    });
                }
            }
            Event::MatchRecord(i, key) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(m) = f.lookup.as_mut().and_then(|l| l.matches.get_mut(i))
                        {
                            m.record = key.clone();
                        }
                    });
                }
            }
            Event::RemoveMatch(i) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(l) = f.lookup.as_mut()
                            && i < l.matches.len()
                        {
                            l.matches.remove(i);
                        }
                    });
                }
            }
            Event::RelatedLayer(layer) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Related {
                            layer: l, field, ..
                        }) = find_mut(&mut b.items, &id)
                        {
                            *l = layer.clone();
                            field.clear();
                        }
                        if let Some(FormItem::Related { columns, .. }) = find_mut(&mut b.items, &id)
                        {
                            columns.clear();
                        }
                    });
                }
            }
            Event::ShowInKind(code) => {
                if let (Some(id), Some(sorter)) = (chosen_item.clone(), self.forms_sorter()) {
                    let rule = Rule::Visible(id);
                    let now = w.body.as_ref().and_then(|b| rule_text(b, &rule));
                    if let Some(mut kinds) = kinds_of_rule(now.as_deref(), &sorter.0) {
                        if let Some(i) = kinds.iter().position(|k| *k == code) {
                            kinds.remove(i);
                        } else {
                            kinds.push(code);
                            kinds.sort_by_key(|k| sorter.1.iter().position(|c| c == k));
                        }
                        let text = rule_of_kinds(&kinds, &sorter.0);
                        edited(&mut w, &|b| set_rule(b, &rule, &text));
                    }
                }
            }
            Event::RelatedView(view) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Related { view: v, .. }) = find_mut(&mut b.items, &id)
                        {
                            *v = view;
                        }
                    });
                }
            }
            Event::RelatedAction(action) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Related { actions, .. }) = find_mut(&mut b.items, &id)
                        {
                            let mut now: Vec<_> =
                                kentos_commands::related_offers(actions.as_deref()).to_vec();
                            if let Some(i) = now.iter().position(|a| *a == action) {
                                now.remove(i);
                            } else {
                                now.push(action);
                            }
                            now.sort();
                            // Back to the default ones, nothing is written.
                            *actions =
                                (now != kentos_commands::RelatedAction::DEFAULT).then_some(now);
                        }
                    });
                }
            }
            Event::RelatedColumn(key) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Related { columns, .. }) = find_mut(&mut b.items, &id)
                        {
                            if let Some(i) = columns.iter().position(|c| *c == key) {
                                columns.remove(i);
                            } else if columns.len() < kentos_commands::MOST_RELATED_COLUMNS {
                                columns.push(key.clone());
                            }
                        }
                    });
                }
            }
            Event::RelatedField(key) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Related { field, .. }) = find_mut(&mut b.items, &id) {
                            *field = key.clone();
                        }
                    });
                }
            }
            Event::Move(by) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| shift(b, &id, by));
                }
            }
            Event::Drop(from, to, place) => {
                let ids = w.body.as_ref().map(|b| (id_at(b, from), id_at(b, to)));
                if let Some((Some(source), Some(target))) = ids
                    && let Some(body) = w.body.as_mut()
                {
                    let mut moved = body.clone();
                    match move_item(&mut moved, &source, &target, place) {
                        Ok(()) => {
                            *body = moved;
                            w.dirty = true;
                            w.item = Some(source);
                        }
                        Err(why) => self.say(Level::Warn, why),
                    }
                }
            }
            Event::Fold(id) => {
                if !w.folded.remove(&id) {
                    w.folded.insert(id);
                }
            }
            Event::Undo
            | Event::Redo
            | Event::Export
            | Event::Exported(_)
            | Event::Import
            | Event::Imported(_) => {}
            Event::Audit => {
                let form = w.chosen.clone();
                if w.dirty {
                    self.say(
                        Level::Warn,
                        "Kayıtlar formun kaydedilmiş hâliyle denetlenir: önce Kaydet’e basın.",
                    );
                    self.gis.forms_window = Some(w);
                    return Task::none();
                }
                self.gis.forms_window = None;
                self.gis.form_preview = None;
                self.dialog = None;
                return self.open_form_audit(form);
            }
            Event::Recompute => {
                if w.dirty {
                    self.say(
                        Level::Warn,
                        "Hesaplar formun kaydedilmiş hâliyle yenilenir: önce Kaydet’e basın.",
                    );
                } else if let Some(form) = self.chosen_form().cloned() {
                    self.gis.forms_window = Some(w);
                    self.recompute_form(&form);
                    return Task::none();
                }
            }
            Event::Changes => {
                w.changes = true;
                w.flow = false;
                w.previewing = false;
                self.gis.form_preview = None;
            }
            Event::Flow => {
                w.flow = true;
                w.changes = false;
                w.previewing = false;
                self.gis.form_preview = None;
            }
            Event::WorkflowAdd => {
                let layer = self
                    .chosen_form()
                    .map(|f| f.layer.clone())
                    .unwrap_or_default();
                let fields = self.layer_fields_of(&layer);
                let listed = fields
                    .iter()
                    .find(|f| f.values.as_ref().is_some_and(|v| v.len() >= 2) && !f.holds_many());
                match listed {
                    None => self.say(
                        Level::Warn,
                        "İş akışı için katmanda iki ya da daha çok değerli, tek değerli bir liste alanı gerekir (Taslak, Gönderildi, Onaylandı gibi).",
                    ),
                    Some(f) => {
                        let codes: Vec<String> =
                            f.values.iter().flatten().map(|c| c.code.clone()).collect();
                        let workflow = kentos_commands::FormWorkflow {
                            field: f.key().to_owned(),
                            initial: codes[0].clone(),
                            locked: Vec::new(),
                            steps: vec![kentos_commands::WorkflowStep {
                                label: "Gönder".into(),
                                from: vec![codes[0].clone()],
                                to: codes[1].clone(),
                                valid: true,
                                reason: None,
                            }],
                            log: None,
                            drafts: Vec::new(),
                            notes: None,
                        };
                        edited(&mut w, &|b| b.workflow = Some(workflow.clone()));
                    }
                }
            }
            Event::WorkflowRemove => edited(&mut w, &|b| b.workflow = None),
            Event::WorkflowField(key) => {
                let layer = self
                    .chosen_form()
                    .map(|f| f.layer.clone())
                    .unwrap_or_default();
                let first = self
                    .layer_fields_of(&layer)
                    .iter()
                    .find(|f| f.key() == key)
                    .and_then(|f| f.values.as_ref()?.first().map(|c| c.code.clone()))
                    .unwrap_or_default();
                edited(&mut w, &|b| {
                    if let Some(wf) = b.workflow.as_mut() {
                        wf.field = key.clone();
                        wf.initial = first.clone();
                        wf.locked.clear();
                        for s in &mut wf.steps {
                            s.from.clear();
                            s.to = first.clone();
                        }
                    }
                });
            }
            Event::WorkflowInitial(code) => edited(&mut w, &|b| {
                if let Some(wf) = b.workflow.as_mut() {
                    wf.initial = code.clone();
                }
            }),
            Event::WorkflowLocked(code) => edited(&mut w, &|b| {
                if let Some(wf) = b.workflow.as_mut() {
                    toggle(&mut wf.locked, &code);
                }
            }),
            Event::WorkflowDraft(code) => edited(&mut w, &|b| {
                if let Some(wf) = b.workflow.as_mut() {
                    toggle(&mut wf.drafts, &code);
                }
            }),
            Event::WorkflowLog(key) => edited(&mut w, &|b| {
                if let Some(wf) = b.workflow.as_mut() {
                    wf.log = key.clone();
                }
            }),
            Event::StepAdd => edited(&mut w, &|b| {
                if let Some(wf) = b.workflow.as_mut() {
                    let n = wf.steps.len() + 1;
                    wf.steps.push(kentos_commands::WorkflowStep {
                        label: format!("Adım {n}"),
                        from: Vec::new(),
                        to: wf.initial.clone(),
                        valid: false,
                        reason: None,
                    });
                }
            }),
            Event::StepRemove(i) => edited(&mut w, &|b| {
                if let Some(wf) = b.workflow.as_mut()
                    && i < wf.steps.len()
                {
                    wf.steps.remove(i);
                }
            }),
            Event::StepLabel(i, text) => edited(&mut w, &|b| {
                if let Some(s) = b.workflow.as_mut().and_then(|wf| wf.steps.get_mut(i)) {
                    s.label = text.clone();
                }
            }),
            Event::StepFrom(i, code) => edited(&mut w, &|b| {
                if let Some(s) = b.workflow.as_mut().and_then(|wf| wf.steps.get_mut(i)) {
                    toggle(&mut s.from, &code);
                }
            }),
            Event::StepTo(i, code) => edited(&mut w, &|b| {
                if let Some(s) = b.workflow.as_mut().and_then(|wf| wf.steps.get_mut(i)) {
                    s.to = code.clone();
                }
            }),
            Event::StepValid(i, on) => edited(&mut w, &|b| {
                if let Some(s) = b.workflow.as_mut().and_then(|wf| wf.steps.get_mut(i)) {
                    s.valid = on;
                }
            }),
            Event::StepReason(i, key) => edited(&mut w, &|b| {
                if let Some(s) = b.workflow.as_mut().and_then(|wf| wf.steps.get_mut(i)) {
                    s.reason = key.clone();
                }
            }),
            Event::Preview(on) => {
                w.changes = false;
                w.flow = false;
                w.previewing = on;
                if !on {
                    self.gis.form_preview = None;
                }
            }
            Event::RemoveItem => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| remove(b, &id));
                    w.item = None;
                }
            }
            Event::Text(which, text) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| set_text(b, &id, &which, &text));
                }
            }
            Event::Widget(widget) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.widget = widget;
                        }
                    });
                }
            }
            Event::Required(on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.required = on;
                        }
                    });
                }
            }
            Event::ReadOnly(on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.read_only = on;
                        }
                    });
                }
            }
            Event::Unique(on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.unique = on;
                        }
                    });
                }
            }
            Event::ScenarioName(text) => w.scenario_name = text,
            Event::RemoveScenario(i) => {
                edited(&mut w, &|b| {
                    if i < b.tests.len() {
                        b.tests.remove(i);
                    }
                });
            }
            Event::SaveScenario => {
                let typed = self.gis.form_preview.as_ref().map(|e| e.typed());
                let form = self.chosen_form().cloned();
                if let (Some((values, new)), Some(form), Some(body), Some(doc)) =
                    (typed, form, w.body.clone(), self.document.as_ref())
                {
                    let name = match w.scenario_name.trim() {
                        "" => format!("Senaryo {}", body.tests.len() + 1),
                        n => n.to_owned(),
                    };
                    match kentos_native_application::forms_test::prepared_for(
                        &doc.model, &form, &body,
                    ) {
                        Ok(prepared) if !body.tests.iter().any(|t| t.name == name) => {
                            let test =
                                kentos_forms::scenario::capture(&prepared, &name, values, new);
                            edited(&mut w, &|b| b.tests.push(test.clone()));
                            w.scenario_name.clear();
                        }
                        Ok(_) => self.say(
                            Level::Warn,
                            format!("“{name}” adlı bir senaryo var; başka bir ad verin."),
                        ),
                        Err(why) => self.say(Level::Warn, why),
                    }
                }
            }
            Event::Overridable(on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.overridable = on;
                        }
                    });
                }
            }
            Event::ReuseLast(on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.reuse_last = on;
                        }
                    });
                }
            }
            Event::Redact(on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.redact = on;
                        }
                    });
                }
            }
            Event::CheckWarning(i, on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(c) = f.checks.get_mut(i)
                        {
                            c.warning = on;
                        }
                    });
                }
            }
            Event::CheckMoment(i, on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(c) = f.checks.get_mut(i)
                        {
                            c.on = on;
                        }
                    });
                }
            }
            Event::Collapsed(on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Group { collapsed, .. }) = find_mut(&mut b.items, &id)
                        {
                            *collapsed = on;
                        }
                    });
                }
            }
            Event::ClearHidden(on) => {
                edited(&mut w, &|b| b.clear_hidden = on);
            }
            Event::Steps(on) => {
                edited(&mut w, &|b| b.steps = on);
            }
            Event::Scope(text) => {
                edited(&mut w, &|b| {
                    b.scope = (!text.is_empty()).then(|| text.clone());
                });
            }
            Event::LanguageText(text) => w.new_language = text,
            Event::LanguageAdd => {
                let tag = w.new_language.trim().to_owned();
                if kentos_commands::language_tag_ok(&tag) {
                    edited(&mut w, &|b| {
                        b.translations.entry(tag.clone()).or_default();
                    });
                    w.language = Some(tag);
                    w.new_language.clear();
                }
            }
            Event::LanguagePick(tag) => w.language = Some(tag),
            Event::LanguageRemove(tag) => {
                edited(&mut w, &|b| {
                    b.translations.remove(&tag);
                });
                if w.language.as_deref() == Some(tag.as_str()) {
                    w.language = None;
                }
            }
            Event::Translate(item, said, text) => {
                if let Some(tag) = w.language.clone() {
                    edited(&mut w, &|b| {
                        let words = b.translations.entry(tag.clone()).or_default();
                        let value = (!text.trim().is_empty()).then(|| text.clone());
                        match &item {
                            None => words.name = value.map(|v| v.trim().to_owned()),
                            Some(id) => {
                                let w = words.items.entry(id.clone()).or_default();
                                match &said {
                                    Said::Label => w.label = value,
                                    Said::Help => w.help = value,
                                    Said::Placeholder => w.placeholder = value,
                                    Said::Choice(code) => match value {
                                        Some(v) => {
                                            w.choices.insert(code.clone(), v);
                                        }
                                        None => {
                                            w.choices.remove(code);
                                        }
                                    },
                                }
                                if w.label.is_none()
                                    && w.help.is_none()
                                    && w.placeholder.is_none()
                                    && w.choices.is_empty()
                                {
                                    words.items.remove(id);
                                }
                            }
                        }
                    });
                }
            }
            Event::Across(on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Group { across, .. }) = find_mut(&mut b.items, &id) {
                            *across = on;
                        }
                    });
                }
            }
            Event::Columns(n) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Group { columns, .. }) = find_mut(&mut b.items, &id) {
                            *columns = (n == 2).then_some(2);
                        }
                    });
                }
            }
            Event::RuleText(rule, text) => {
                edited(&mut w, &|b| set_rule(b, &rule, &text));
            }
            Event::Builder(rule) => {
                let value = w
                    .body
                    .as_ref()
                    .and_then(|b| rule_text(b, &rule))
                    .unwrap_or_default();
                let layer = self.chosen_form().map(|f| f.layer.clone());
                self.gis.forms_window = Some(w);
                return match layer {
                    Some(layer) => self.open_builder_for_form(rule, &layer, &value),
                    None => Task::none(),
                };
            }
            Event::AddCheck => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.checks.push(FormCheck {
                                rule: String::new(),
                                message: String::new(),
                                warning: false,
                                on: None,
                            });
                        }
                    });
                }
            }
            Event::RemoveCheck(i) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && i < f.checks.len()
                        {
                            f.checks.remove(i);
                        }
                    });
                }
            }
            Event::Name(text) => {
                w.name = text;
            }
            Event::Save => {
                self.gis.forms_window = Some(w);
                let name = self
                    .gis
                    .forms_window
                    .as_ref()
                    .map(|w| w.name.trim().to_owned());
                if self.save_form() {
                    self.gis.forms_window = None;
                    self.dialog = None;
                    self.say(
                        Level::Success,
                        format!("“{}” formu kaydedildi.", name.unwrap_or_default()),
                    );
                }
                return Task::none();
            }
            // Yine de yayınla skips the question of the records a release breaks (docs/adr/0442 §2).
            ev @ (Event::Publish | Event::PublishAnyway) => {
                let anyway = matches!(ev, Event::PublishAnyway);
                w.breaking = None;
                self.gis.forms_window = Some(w);
                if !self.save_form() {
                    return Task::none();
                }
                let id = self
                    .gis
                    .forms_window
                    .as_ref()
                    .and_then(|w| w.chosen.clone())
                    .unwrap_or_default();
                // The records the release would break are asked about first (docs/adr/0442 §2); a failing
                // scenario stops it here, said by its refusal.
                if !anyway && let Some(doc) = self.document.as_mut() {
                    let planned = forms_edit::plan(
                        &ExecutionContext::new(&mut doc.model),
                        &FormsEdit {
                            edit: FormsOp::Publish {
                                id: id.clone(),
                                note: None,
                            },
                            expected_revision: None,
                        },
                    );
                    match planned {
                        kentos_commands::CommandResult::Completed { output, .. } => {
                            if let Some(b) = output.breaking {
                                if let Some(w) = self.gis.forms_window.as_mut() {
                                    w.breaking = Some(b);
                                }
                                return Task::none();
                            }
                        }
                        other => {
                            let _ = self.answered(other);
                            return Task::none();
                        }
                    }
                }
                if let Some(done) = self.edit_forms(FormsOp::Publish {
                    id: id.clone(),
                    note: None,
                }) && let Some(f) = done.form
                {
                    let version = f.releases.last().map_or(0, |r| r.version);
                    let said = if done.changed {
                        format!("“{}” formu yayınlandı: {version}. sürüm.", f.name)
                    } else {
                        format!(
                            "“{}” formunun taslağı son yayınla aynı ({version}. sürüm).",
                            f.name
                        )
                    };
                    self.say(Level::Success, said);
                }
                return Task::none();
            }
            Event::Enforce(on) => {
                if let Some(id) = w.chosen.clone()
                    && self.edit_forms(FormsOp::Enforce { id, on }).is_some()
                {
                    let said = if on {
                        "Formun son yayını katmanın öznitelik yazmalarında uygulanacak."
                    } else {
                        "Formun kuralları artık yalnız formla yazarken uygulanır."
                    };
                    self.say(Level::Success, said);
                }
            }
            Event::Withdraw(on) => {
                if let Some(id) = w.chosen.clone()
                    && self.edit_forms(FormsOp::Withdraw { id, on }).is_some()
                {
                    let said = if on {
                        "Form yayından kaldırıldı: kayıt bu formla yazılmaz; açık pencereler yazdıklarını korur."
                    } else {
                        "Form yeniden yayında: kayıtlar son yayınıyla yazılır."
                    };
                    self.say(Level::Success, said);
                }
            }
            Event::RemoveForm => {
                if let Some(id) = w.chosen.clone() {
                    let name = w.name.clone();
                    if self.edit_forms(FormsOp::Remove { id }).is_some() {
                        self.say(Level::Success, format!("“{name}” formu silindi."));
                        w = Window::default();
                        if let Some(first) = self.project_forms().first().map(|f| f.id.clone()) {
                            self.choose_into(&mut w, &first);
                        }
                    }
                }
            }
            Event::Cancel => {
                // Vazgeç lets the window's changes go, the draft kept on this device with them.
                if let Some(form) = w.chosen.as_deref() {
                    self.gis
                        .form_files
                        .drop_draft(&self.form_project_key(), form);
                }
                self.gis.forms_window = None;
                self.gis.form_preview = None;
                self.dialog = None;
                return Task::none();
            }
        }
        // The draft kept on this device as it is edited (docs/adr/0441 §5): a crash does not lose it.
        if w.dirty
            && let (Some(form), Some(body)) = (w.chosen.clone(), w.body.clone())
        {
            self.gis.form_files.keep_draft(
                &self.form_project_key(),
                &crate::forms::library::KeptDraft {
                    form,
                    name: w.name.clone(),
                    body,
                },
            );
        }
        self.gis.forms_window = Some(w);
        Task::none()
    }
}
