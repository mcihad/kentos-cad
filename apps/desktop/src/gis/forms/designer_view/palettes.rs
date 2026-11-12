//! The form designer's palettes (fields, relations, value lists, sections), an item's translations and
//! review comments, and the form's overview with no item chosen (docs/adr/0441).

use super::*;
use iced::widget::column;

impl App {
    /// The layers a form can be made for, by their path.
    pub(super) fn form_layers(&self) -> Vec<(String, String)> {
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
    pub(super) fn palette_row<'a>(
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
    pub(super) fn relations_palette(&self, editable: bool) -> Element<'_, Message> {
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
    pub(super) fn domains_palette(&self, fields: &[LayerField]) -> Element<'_, Message> {
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
    pub(super) fn sections_palette<'a>(
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
    pub(super) fn section_save_row(&self) -> Element<'_, Message> {
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
    pub(super) fn translations_box<'a>(&'a self, body: &FormBody) -> Element<'a, Message> {
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

    pub(super) fn comments_box(&self, item: Option<&str>) -> Element<'_, Message> {
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
    pub(super) fn form_overview<'a>(&'a self, body: &FormBody) -> Element<'a, Message> {
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
}
