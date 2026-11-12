//! The copy's scenarios (docs/adr/0420), its workflow (docs/adr/0428) and its changes against the form's
//! last release (docs/adr/0427).

use super::*;
use iced::widget::column;

impl App {
    /// Senaryolar (docs/adr/0420): the copy's scenarios run on it as the command runs them, the
    /// preview's values kept as a new one.
    pub(super) fn scenarios_box<'a>(&self, w: &'a Window) -> Element<'a, Message> {
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
    pub(super) fn forms_flow<'a>(&self, w: &'a Window) -> Element<'a, Message> {
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
    pub(super) fn forms_changes<'a>(&self, w: &'a Window) -> Element<'a, Message> {
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
