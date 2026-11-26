//! The designer's inspector of the chosen item: its properties, subtypes and links, a field's list from
//! another layer with its order, columns and copies (docs/adr/0404, 0439); the preview of the copy.

use super::*;
use iced::widget::column;

impl App {
    pub(super) fn forms_inspector(
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
                actions,
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
                // What the item lets do with the children (docs/adr/0762 §1).
                let offered = kentos_commands::related_offers(actions.as_deref());
                let mut chips = Row::new().spacing(4);
                for a in kentos_commands::RelatedAction::ALL {
                    chips = chips.push(
                        button(label::caption(a.label()))
                            .on_press(msg(Event::RelatedAction(a)))
                            .padding([2, 8])
                            .style(style::button::chip(offered.contains(&a))),
                    );
                }
                col = col.push(words::field(
                    "Eylemler",
                    chips.wrap(),
                    Some(
                        "Seçilenler formda görünür; Sil yalnız seçilince. Kullanıcının izni ayrıca sorulur: kilitli katman ve rol düğmeyi soluk gösterir."
                            .into(),
                    ),
                ));
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
                pin,
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
                            .gis
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
                            move |i| msg(Event::PinnedTool(ids[i].clone())),
                        )
                        .placeholder("İşlem aracı");
                        let missing = at.is_none().then(|| {
                            format!("“{current}” kayıtlı bir işlem aracı değil; listeden seçin.")
                        });
                        // An extension's tool says the extension it is pinned to (docs/adr/8057 §12).
                        let pinned = pin.as_ref().map(|p| {
                            format!(
                                "{} {} ile sabitlendi.",
                                self.extension_name(&p.extension),
                                p.version
                            )
                        });
                        let places = [R::Auto, R::Here, R::Background];
                        let runs_pick = Select::new(
                            ["Otomatik", "Bu bilgisayarda", "Arka planda"].map(Choice::new),
                            places.iter().position(|r| r == runs),
                            move |i| msg(Event::ActionRuns(places[i])),
                        );
                        col = col
                            .push(words::field("Araç", pick, missing.or(pinned)))
                            .push(words::field("Nerede çalışır", runs_pick, None));
                    }
                    K::Step => {
                        let steps: Vec<String> = self
                            .gis
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
                // The field's name, its kind under it (DESIGN.md §13: no “A · B” title).
                let head = column![
                    label::strong(layer.map_or(f.field.clone(), |l| l.label().to_owned())),
                    label::caption(match layer {
                        Some(l) => format!("{} alanı", l.kind.label()),
                        None => "Katmanda yok".to_owned(),
                    })
                    .style(muted),
                ]
                .spacing(1);
                let label_field = words::field(
                    "Etiket",
                    input(
                        layer.map_or("", |l| l.label()),
                        f.label.as_deref().unwrap_or(""),
                        Text::Label,
                    ),
                    None,
                );
                let help_field = words::field(
                    "Yardım",
                    input(
                        "Alanın altında",
                        f.help.as_deref().unwrap_or(""),
                        Text::Help,
                    ),
                    None,
                );
                // A mirror has only its label, help, widget and visibility (docs/adr/0437).
                if f.mirror {
                    return col
                        .push(head)
                        .push(label_field)
                        .push(words::field("Öğe türü", widget, None))
                        .push(label::caption(
                            "Yansıma: alanın değeri burada salt okunur görünür; alanın kendi öğesi yazar.",
                        ).style(muted))
                        .push(help_field)
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
                // Five sections, Genel open and the rest folded until opened (twenty-odd controls one under another
                // before): a folded one says how many of its settings are in use, and one with an error opens.
                let toggled = self.gis.forms_window.as_ref().map(|w| &w.toggled_sections);
                let errs = |paths: &[String]| paths.iter().any(|p| error_of(p).is_some());
                let open = |name: &'static str, default: bool, error: bool| {
                    error || (default != toggled.is_some_and(|t| t.contains(name)))
                };
                let heading =
                    |name: &'static str, used: usize, open: bool| -> Element<'_, Message> {
                        let title = if used > 0 && !open {
                            format!("{name} ({used})")
                        } else {
                            name.to_owned()
                        };
                        kentos_ui::widget::Form::new()
                            .foldable_section(title, open, msg(Event::InspectorSection(name)))
                            .into()
                    };
                let values_used = [
                    f.default.is_some(),
                    f.default_expr.is_some(),
                    f.calculate.is_some(),
                    f.overridable,
                    f.reuse_last,
                    f.suggest,
                    f.unique,
                ]
                .iter()
                .filter(|b| **b)
                .count();
                let rules_used = [
                    f.required || layer_required,
                    f.read_only,
                    f.visible_when.is_some(),
                    f.required_when.is_some(),
                    f.read_only_when.is_some(),
                    f.choice_filter.is_some(),
                ]
                .iter()
                .filter(|b| **b)
                .count();
                let general_open = open(SECTION_GENERAL, true, error_of("lookup").is_some());
                let value_open = open(
                    SECTION_VALUE,
                    false,
                    error_of("default").is_some()
                        || errs(&[
                            Rule::Default(id.clone()).path(),
                            Rule::Calculate(id.clone()).path(),
                        ]),
                );
                let rules_open = open(
                    SECTION_RULES,
                    false,
                    errs(&[
                        Rule::Visible(id.clone()).path(),
                        Rule::Required(id.clone()).path(),
                        Rule::ReadOnly(id.clone()).path(),
                        Rule::ChoiceFilter(id.clone()).path(),
                    ]),
                );
                let check_paths: Vec<String> = (0..f.checks.len())
                    .map(|i| Rule::Check(id.clone(), i).path())
                    .collect();
                let checks_open = open(SECTION_CHECKS, false, errs(&check_paths));
                let more_open = open(SECTION_MORE, false, false);

                col = col
                    .push(head)
                    .push(heading(SECTION_GENERAL, 0, general_open));
                if general_open {
                    col = col
                        .push(label_field)
                        .push(words::field("Öğe türü", widget, None))
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
                    col = col.push(help_field).push(words::field(
                        "Yer tutucu",
                        input(
                            "Boşken",
                            f.placeholder.as_deref().unwrap_or(""),
                            Text::Placeholder,
                        ),
                        None,
                    ));
                }

                col = col.push(heading(SECTION_VALUE, values_used, value_open));
                if value_open {
                    col = col
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
                        .push(words::check(
                            f.reuse_last,
                            "Son değeri kullan: yeni kayıt bir öncekinin değeriyle başlar",
                            (f.calculate.is_none() || f.reuse_last)
                                .then(|| msg(Event::ReuseLast(!f.reuse_last))),
                        ))
                        .push(words::check(
                            f.suggest,
                            "Öneri: yazarken katmandaki değerler önerilir",
                            (layer.is_some_and(|l| {
                                l.kind == kentos_commands::LayerFieldKind::Text
                                    && l.values.as_ref().is_none_or(|v| v.is_empty())
                            }) && f.lookup.is_none()
                                || f.suggest)
                                .then(|| msg(Event::Suggest(!f.suggest))),
                        ))
                        .push(words::check(
                            f.unique,
                            "Tekil: aynı değer iki kayıtta olamaz",
                            Some(msg(Event::Unique(!f.unique))),
                        ));
                }

                col = col.push(heading(SECTION_RULES, rules_used, rules_open));
                if rules_open {
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
                        .extend(
                            required_scope_words(f.required, layer_required)
                                .map(|w| label::caption(w).style(muted).into()),
                        )
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
                    // Alt türlerde göster (docs/adr/0426): the visibility rule by the layer's subtypes.
                    if let Some(row) = self.kinds_row(item) {
                        col = col.push(row);
                    }
                }

                col = col.push(heading(SECTION_CHECKS, f.checks.len(), checks_open));
                if checks_open {
                    if f.checks.is_empty() {
                        col = col.push(
                            label::caption("Denetim yok. Bir denetim, değer kurala uymazsa ne söyleneceğini yazar.")
                                .style(muted),
                        );
                    }
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

                col = col.push(heading(SECTION_MORE, usize::from(f.redact), more_open));
                if more_open {
                    col = col
                        // Kept out of the recorder's examples (docs/adr/0443 §2).
                        .push(words::check(
                            f.redact,
                            "Örneklerde gizle: komut geçmişinde değeri ‹gizli› yazılır",
                            Some(msg(Event::Redact(!f.redact))),
                        ))
                        .push(
                            column![
                                words::secondary("Yansımasını ekle", Some(msg(Event::AddMirror))),
                                label::caption("Alanı formun başka bir yerinde bir kez daha, salt okunur gösterir.")
                                    .style(muted),
                            ]
                            .spacing(4),
                        )
                        // The item into the section library and its review comments (docs/adr/0441 §1, §5).
                        .push(self.section_save_row())
                        .push(self.comments_box(Some(&id)))
                        .push(self.field_links(f, fields));
                }
                return scrollable(col.padding(iced::Padding {
                    top: 0.0,
                    right: 16.0,
                    bottom: 0.0,
                    left: 4.0,
                }))
                .id(PROPERTIES_SCROLL)
                .on_scroll(|v| msg(Event::PropertiesScrolled(v.absolute_offset().y)))
                .height(Fill)
                .into();
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
        .on_scroll(|v| msg(Event::PropertiesScrolled(v.absolute_offset().y)))
        .height(Fill)
        .into()
    }

    /// Alt türlerde göster (docs/adr/0426): a chip for each of the layer's subtypes; the chosen ones are
    /// the item's visibility rule. A rule written otherwise leaves the chips off.
    pub(super) fn kinds_row<'a>(&self, item: &FormItem) -> Option<Element<'a, Message>> {
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
    pub(super) fn field_links<'a>(
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
            (&self.document, &self.gis.forms_window, self.chosen_form())
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
}

impl App {
    /// İlişki (docs/adr/0440 §6): the project relations whose parent is the form's layer; one picked fills
    /// the related item's layer and field.
    pub(super) fn relation_pick<'a>(&'a self, layer: &str, field: &str) -> Element<'a, Message> {
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

    /// A field's Seçenek kaynağı (docs/adr/0404 §1, 9322), each source by name: the layer field's own list (what it is
    /// said: a project list it is linked to, its own codes, none), a project list of its kind (linking the layer field
    /// to it), another layer (its key, Kimlik: the object's persistent id, its label, the chain's matches and the
    /// filter). Under it what the list holds and Liste oluştur ve bağla.
    pub(super) fn lookup_rows<'a>(
        &'a self,
        id: &str,
        f: &kentos_commands::FormField,
        error: Option<String>,
    ) -> Element<'a, Message> {
        use crate::forms::designer::lists::{FieldList, list_summary, list_words};
        let layers = self.form_layers();
        let field = self
            .chosen_form()
            .and_then(|form| self.layer_fields_of(&form.layer).into_iter().find(|x| x.key() == f.field));
        let lists = field.as_ref().map(|x| self.lists_for(x.kind)).unwrap_or_default();
        let now = field.as_ref().map(|x| self.field_list(x, &lists));
        let linked = match &now {
            Some(FieldList::Linked(d)) => Some((*d).clone()),
            _ => None,
        };
        let own = match &now {
            Some(FieldList::Linked(d)) => format!("Katman alanının listesi · {} (proje listesi)", d.name),
            Some(FieldList::Own(n)) => format!("Katman alanının listesi · kendi {n} kodu"),
            _ => "Katman alanının listesi · liste yok".to_owned(),
        };
        let mut names = vec![Choice::new(own)];
        names.extend(lists.iter().map(|d| Choice::new(format!("Proje listesi: {}", list_words(d)))));
        names.extend(layers.iter().map(|(_, n)| Choice::new(format!("Başka katmandan: {n}"))));
        let at = f
            .lookup
            .as_ref()
            .and_then(|l| layers.iter().position(|(lid, _)| *lid == l.layer))
            .map_or(0, |i| 1 + lists.len() + i);
        let list_ids: Vec<String> = lists.iter().map(|d| d.id.clone()).collect();
        let picks = layers.clone();
        let pick_source = Select::new(names, Some(at), move |i| match i {
            0 => msg(Event::LookupLayer(None)),
            i if i <= list_ids.len() => msg(Event::LinkDomain(list_ids[i - 1].clone())),
            i => msg(Event::LookupLayer(picks.get(i - 1 - list_ids.len()).map(|(l, _)| l.clone()))),
        });
        let mut col = Column::new().spacing(8).push(words::field("Seçenek kaynağı", pick_source, error));
        if f.lookup.is_none()
            && let Some(x) = &field
        {
            col = col.push(label::caption(list_summary(x, linked.as_ref())).style(muted));
        }
        col = col.push(words::ghost(
            "Liste oluştur ve bağla…",
            (field.is_some() && f.lookup.is_none()).then(|| msg(Event::NewDomainForField)),
        ));
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
    pub(super) fn lookup_extras<'a>(
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
    pub(super) fn forms_preview<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        if let Some(why) = &w.preview_error {
            return container(words::summary(vec![words::text_line(
                Kind::Warn,
                why.clone(),
            )]))
            .padding(8)
            .into();
        }
        let Some(e) = &self.gis.form_preview else {
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

/// Where a field's Zorunlu holds, said under its box: the layer's own is every way in (table, properties, Python); the
/// form's only this form's entries.
pub(crate) fn required_scope_words(form: bool, layer: bool) -> Option<&'static str> {
    if layer {
        Some("Katman alanı zorunlu: tabloda, özelliklerde ve Python'da da boş bırakılamaz.")
    } else if form {
        Some(
            "Yalnız bu formla girişte zorunlu; tabloda ve özelliklerde boş bırakılabilir. Her yerde zorunlu olması için Veri › Alanlar'da alanın Zorunlu kutusunu açın.",
        )
    } else {
        None
    }
}

#[cfg(test)]
mod required_tests {
    /// UR-069: the form's Zorunlu says it holds only in this form, the layer's that it holds everywhere.
    #[test]
    fn zorunlu_says_where_it_holds() {
        use super::required_scope_words as words;
        assert_eq!(words(false, false), None);
        assert!(
            words(true, false).is_some_and(|w| w.starts_with("Yalnız bu formla girişte zorunlu"))
        );
        assert!(words(true, true).is_some_and(|w| w.starts_with("Katman alanı zorunlu")));
        assert!(words(false, true).is_some_and(|w| w.starts_with("Katman alanı zorunlu")));
    }
}

#[cfg(test)]
mod required_screens {
    /// UR-069's pictures: a field made Zorunlu in the form only, its scope said under the box,
    /// `.run/gui-evidence/kullanim-069/`.
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        use crate::forms::designer as fd;
        crate::files_testing::usage_shots("069", "form-zorunlu-kapsami", || {
            let mut app = crate::files_testing::app_with_drawing();
            crate::form_scenes::buildings(&mut app, true);
            crate::tools_scenes::run(&mut app, "data.forms");
            crate::form_scenes::designer(&mut app, fd::Event::SelectItem("durum".into()));
            crate::form_scenes::designer(&mut app, fd::Event::Required(true));
            app
        });
    }
}
