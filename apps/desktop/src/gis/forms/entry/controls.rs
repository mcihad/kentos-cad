//! A field's control by its widget (docs/adr/0401 §9): text, numbers, choices, colours, dates and times
//! with their offsets.

use super::*;

pub(super) fn entry_control<'a>(
    e: &'a Entry,
    item: &'a kentos_commands::FormField,
    field: &'a LayerField,
    read_only: bool,
) -> Element<'a, Message> {
    let key = field.key().to_owned();
    // The record's date and time or time of day as people write it (16.10.2026 14:30, 08:00;
    // docs/adr/0412 §2); what is typed stays as typed.
    let value: String = e
        .values
        .get(&key)
        .cloned()
        .or_else(|| {
            e.after.get(&field.name).map(|v| match field.kind {
                LayerFieldKind::DateTime | LayerFieldKind::Time => {
                    kentos_commands::display_value(field, v)
                }
                _ => v.clone(),
            })
        })
        .unwrap_or_default();
    let set = {
        let key = key.clone();
        move |v: String| e.msg(Event::Value(key.clone(), v))
    };
    let placeholder = if e.mixed.contains(&field.name) && !e.values.contains_key(&key) {
        "Çeşitli değerler".to_owned()
    } else {
        e.said(&item.id)
            .and_then(|w| w.placeholder.clone())
            .or_else(|| item.placeholder.clone())
            // A new record's value its table works out (docs/adr/0753 §2), and any record's the table always makes
            // (docs/adr/0754 §3): left empty, said.
            .or_else(|| {
                (e.new_record || field.made_always())
                    .then(|| crate::layer_fields::made_said(field.column.as_ref()))
                    .flatten()
            })
            .unwrap_or_default()
    };
    // What the item's choice filter offers for the record; every listed value without one.
    let offered = e.state(&item.id).and_then(|s| s.choices.as_ref());
    // A list from another layer offers its values for the record (docs/adr/0404 §2).
    // A paged list's whole list where it is searched; its page elsewhere (docs/adr/0439 §1).
    let more = e.state(&item.id).and_then(|s| s.more);
    let paged = match kentos_forms::widget_of(item, field) {
        FormWidget::Search => true,
        FormWidget::List => more.is_some(),
        _ => false,
    };
    let options = e
        .whole
        .get(&item.id)
        .filter(|_| paged)
        .or_else(|| e.state(&item.id).and_then(|s| s.options.as_ref()));
    let looked: Option<Vec<(String, String)>> = item.lookup.as_ref().map(|_| {
        options
            .map(|o| {
                o.iter()
                    .map(|c| (c.code.clone(), c.label.clone()))
                    .collect()
            })
            .unwrap_or_default()
    });
    // A value's columns from its layer, said beside its label (docs/adr/0439 §2).
    let details: BTreeMap<&str, &str> = options
        .map(|o| {
            o.iter()
                .filter_map(|c| Some((c.code.as_str(), c.description.as_deref()?)))
                .collect()
        })
        .unwrap_or_default();
    // A retired value, or one outside its days (docs/adr/0438 §5), only for the record holding it
    // (docs/adr/0418).
    let listed = field.offered_values_on(
        e.after.get(&field.name).map_or("", String::as_str),
        e.moment.as_deref(),
    );
    let choices: Vec<(String, String)> = match &field.values {
        _ if looked.is_some() => looked.clone().unwrap_or_default(),
        Some(list) if !list.is_empty() => listed
            .iter()
            .filter(|c| offered.is_none_or(|o| o.contains(&c.code)))
            // A label in the chosen language; the code is the list's own (docs/adr/0443 §5).
            .map(|c| {
                let said = e
                    .said(&item.id)
                    .and_then(|w| w.choices.get(&c.code))
                    .cloned();
                (c.code.clone(), said.unwrap_or_else(|| c.label.clone()))
            })
            .collect(),
        _ if field.kind == LayerFieldKind::Boolean => vec![
            ("true".into(), "Evet".into()),
            ("false".into(), "Hayır".into()),
        ],
        _ => Vec::new(),
    };
    // The value's code: as typed, a label's code, or the record's.
    let code = kentos_commands::check_value(
        &LayerField {
            required: false,
            ..field.clone()
        },
        &value,
    )
    .unwrap_or_else(|_| value.clone());
    // A list from another layer: a label typed is its value's (as the core writes it).
    let at = choices.iter().position(|(c, _)| *c == code).or_else(|| {
        looked
            .as_ref()
            .and(choices.iter().position(|(_, l)| *l == value))
    });
    // A list from another layer is not known before the record: its segmented choice is a list. A
    // long list is searched (docs/adr/0439 §6): its values are not all laid out.
    let widget = match kentos_forms::widget_of(item, field) {
        FormWidget::Segmented if looked.is_some() => FormWidget::List,
        FormWidget::List if choices.len() > LONG_LIST || more.is_some() => FormWidget::Search,
        w => w,
    };
    match widget {
        FormWidget::Check | FormWidget::Switch if read_only => label::body(if code == "true" {
            "Evet"
        } else if code == "false" {
            "Hayır"
        } else {
            "—"
        })
        .into(),
        // A check box for each listed value the record may hold (docs/adr/0408 §2).
        FormWidget::CheckList => {
            let held: Vec<String> = field
                .codes_of(&code)
                .into_iter()
                .map(str::to_owned)
                .collect();
            let mut col = Column::new().spacing(4);
            for (c, words) in &choices {
                let on = held.contains(c);
                let next: Vec<&str> = choices
                    .iter()
                    .map(|(x, _)| x.as_str())
                    .filter(|x| {
                        if x == c {
                            !on
                        } else {
                            held.iter().any(|h| h == x)
                        }
                    })
                    .collect();
                let toggle = set(next.join(kentos_commands::MULTIPLE_JOIN));
                col = col.push(
                    row![
                        check_box(
                            if on { Check::Checked } else { Check::Unchecked },
                            (!read_only).then_some(toggle),
                        ),
                        label::body(words.clone()),
                    ]
                    .spacing(8)
                    .align_y(Center),
                );
            }
            // A held code no longer offered still shows, so it can be taken off.
            for h in held
                .iter()
                .filter(|h| !choices.iter().any(|(c, _)| c == *h))
            {
                let next: Vec<&str> = held
                    .iter()
                    .filter(|x| *x != h)
                    .map(String::as_str)
                    .collect();
                col = col.push(
                    row![
                        check_box(
                            Check::Checked,
                            (!read_only).then(|| set(next.join(kentos_commands::MULTIPLE_JOIN))),
                        ),
                        label::body(kentos_commands::display_value(field, h)).style(muted),
                    ]
                    .spacing(8)
                    .align_y(Center),
                );
            }
            col.into()
        }
        // The held codes as chips, one more added from the list (docs/adr/0431).
        FormWidget::Chips => {
            let held: Vec<String> = field
                .codes_of(&code)
                .into_iter()
                .map(str::to_owned)
                .collect();
            // The list's order, a held code no longer offered after them.
            let ordered = |codes: &[&str]| -> String {
                let mut out: Vec<&str> = choices
                    .iter()
                    .map(|(c, _)| c.as_str())
                    .filter(|c| codes.contains(c))
                    .collect();
                out.extend(
                    codes
                        .iter()
                        .filter(|c| !choices.iter().any(|(x, _)| x == *c)),
                );
                out.join(kentos_commands::MULTIPLE_JOIN)
            };
            let mut chips = Row::new().spacing(4);
            for h in &held {
                let words = choices.iter().find(|(c, _)| c == h).map_or_else(
                    || kentos_commands::display_value(field, h),
                    |(_, l)| l.clone(),
                );
                let rest: Vec<&str> = held
                    .iter()
                    .filter(|x| *x != h)
                    .map(String::as_str)
                    .collect();
                let face = row![label::caption(words)].spacing(4).align_y(Center);
                let face = if read_only {
                    face
                } else {
                    face.push(icon(crate::icons::from_web(Some("close"))).size(10.0))
                };
                chips = chips.push(
                    button(face)
                        .padding([2, 8])
                        .style(style::button::chip(true))
                        .on_press_maybe((!read_only).then(|| set(ordered(&rest)))),
                );
            }
            let mut col = Column::new().spacing(6).push(chips.wrap());
            if !read_only {
                let free: Vec<(String, String)> = choices
                    .iter()
                    .filter(|(c, _)| !held.contains(c))
                    .cloned()
                    .collect();
                if !free.is_empty() {
                    let held_now = held.clone();
                    let set_add = set.clone();
                    let codes: Vec<String> = free.iter().map(|(c, _)| c.clone()).collect();
                    let order: Vec<String> = choices.iter().map(|(c, _)| c.clone()).collect();
                    let add = Select::new(
                        free.iter().map(|(_, l)| Choice::new(l.clone())),
                        None,
                        move |i| {
                            let mut next: Vec<&str> = held_now.iter().map(String::as_str).collect();
                            next.push(codes[i].as_str());
                            set_add(ordered_codes(&next, &order))
                        },
                    )
                    .placeholder("Ekle…");
                    col = col.push(container(add).width(220));
                }
            }
            col.into()
        }
        // The list as a tree (docs/adr/0437): what a label holds before its last “ / ” is its branch.
        FormWidget::Tree if !read_only => {
            let mut col = Column::new().spacing(2);
            let mut last: Option<String> = None;
            for (c, words) in &choices {
                let (branch, leaf) = match words.rsplit_once(" / ") {
                    Some((b, l)) => (Some(b.to_owned()), l.to_owned()),
                    None => (None, words.clone()),
                };
                if branch != last {
                    if let Some(b) = &branch {
                        col = col.push(container(label::caption(b.clone()).style(muted)).padding(
                            iced::Padding {
                                top: 4.0,
                                ..iced::Padding::ZERO
                            },
                        ));
                    }
                    last = branch.clone();
                }
                let indent = if branch.is_some() { 14.0 } else { 0.0 };
                col = col.push(row![
                    space::horizontal().width(indent),
                    button(label::body(leaf))
                        .padding([3, 8])
                        .width(Fill)
                        .style(style::button::row(*c == code))
                        .on_press(set(c.clone())),
                ]);
            }
            container(col)
                .padding(4)
                .width(Fill)
                .style(style::container::field_box)
                .into()
        }
        // A long list searched by typing (docs/adr/0431): the labels holding what is typed.
        FormWidget::Search if !read_only => {
            let query = e.queries.get(&key).cloned();
            let shown = query
                .clone()
                .unwrap_or_else(|| at.map_or_else(|| value.clone(), |i| choices[i].1.clone()));
            // What was searched once typing paused, a page at a time; a value's columns searched
            // with its label (docs/adr/0439 §2, §6).
            let searched = e.searched.get(&key).filter(|_| query.is_some());
            let least = item.min_search.unwrap_or(1).max(1) as usize;
            let short = searched.is_some_and(|q| !q.is_empty() && q.chars().count() < least);
            let page = SEARCH_PAGE * e.pages.get(&key).copied().unwrap_or(1);
            let texts: Vec<String> = choices
                .iter()
                .map(|(c, l)| match details.get(c.as_str()) {
                    Some(d) => format!("{l} {d}"),
                    None => l.clone(),
                })
                .collect();
            // The labels holding what is typed first, then the values whose columns hold it.
            let labels: Vec<String> = choices.iter().map(|(_, l)| l.clone()).collect();
            let found: Vec<usize> = match searched {
                _ if short => Vec::new(),
                Some(q) if !q.is_empty() => {
                    let mut out: Vec<usize> = kentos_forms::suggestions_for(&labels, q, page + 1)
                        .into_iter()
                        .filter_map(|s| labels.iter().position(|x| std::ptr::eq(x, s)))
                        .collect();
                    if out.len() <= page {
                        for s in kentos_forms::suggestions_for(&texts, q, page + 1) {
                            if let Some(i) = texts.iter().position(|x| std::ptr::eq(x, s))
                                && !out.contains(&i)
                            {
                                out.push(i);
                            }
                            if out.len() > page {
                                break;
                            }
                        }
                    }
                    out
                }
                _ => (0..choices.len().min(page + 1)).collect(),
            };
            let more = found.len() > page;
            let found: Vec<usize> = found.into_iter().take(page).collect();
            let picked: Vec<String> = found.iter().map(|&i| choices[i].0.clone()).collect();
            let shown_count = picked.len();
            let k = key.clone();
            let km = key.clone();
            let on_pick = set.clone();
            let mut rows: Vec<kentos_ui::widget::suggest::Suggestion<'a>> = found
                .iter()
                .map(|&i| {
                    let s = kentos_ui::widget::suggest::Suggestion::new(choices[i].1.clone());
                    match details.get(choices[i].0.as_str()) {
                        Some(d) => s.detail((*d).to_owned()),
                        None => s,
                    }
                })
                .collect();
            if more {
                rows.push(kentos_ui::widget::suggest::Suggestion::offer(
                    "Daha fazla göster",
                ));
            }
            let mut suggest = kentos_ui::widget::Suggest::new(shown.as_str(), placeholder.as_str())
                .id(input_id(&key))
                .on_input(move |t| e.msg(Event::Query(k.clone(), t)))
                .suggestions(rows)
                .on_pick(move |i| {
                    if i >= shown_count {
                        e.msg(Event::More(km.clone()))
                    } else {
                        on_pick(picked.get(i).cloned().unwrap_or_default())
                    }
                });
            if short {
                suggest = suggest.note(format!("Aramak için en az {least} harf yazın."));
            } else if query.is_some() && e.pending.contains_key(&key) {
                suggest = suggest.note("Aranıyor…");
            }
            suggest.into()
        }
        FormWidget::Check => {
            let on = code == "true";
            let set = set.clone();
            check_box(
                if on { Check::Checked } else { Check::Unchecked },
                Some(set(if on { "false" } else { "true" }.to_owned())),
            )
        }
        FormWidget::Switch => {
            let set = set.clone();
            Switch::new(code == "true", move |on| {
                set(if on { "true" } else { "false" }.to_owned())
            })
            .into()
        }
        FormWidget::List if !read_only => {
            let picks = choices.clone();
            let set_pick = set.clone();
            let mut select = Select::new(
                choices.iter().map(|(c, l)| match details.get(c.as_str()) {
                    Some(d) => Choice::new(l.clone()).detail((*d).to_owned()),
                    None => Choice::new(l.clone()),
                }),
                at,
                move |i| set_pick(picks[i].0.clone()),
            );
            if !field.required && !item.required {
                select = select.clear(set(String::new()));
            }
            select
                .placeholder(if placeholder.is_empty() {
                    "Seçin".to_owned()
                } else {
                    placeholder
                })
                .into()
        }
        FormWidget::Radio if !read_only => {
            let picks = choices.clone();
            let mut group =
                RadioGroup::new(at, move |i: usize| set(picks[i].0.clone())).horizontal();
            for (i, (_, l)) in choices.iter().enumerate() {
                group = group.option(i, l.clone(), String::new());
            }
            group.into()
        }
        FormWidget::Segmented if !read_only => {
            let held = field.retired_in(e.after.get(&field.name).map_or("", String::as_str));
            let opts: Vec<Opt<'a>> = field
                .values
                .as_ref()
                .filter(|v| !v.is_empty())
                .map(|v| {
                    v.iter()
                        .filter(|c| !c.retired || held.contains(&c.code))
                        .filter(|c| offered.is_none_or(|o| o.contains(&c.code)))
                        .enumerate()
                        .map(|(i, c)| Opt(i, c.label.as_str()))
                        .collect()
                })
                .unwrap_or_else(|| vec![Opt(0, "Evet"), Opt(1, "Hayır")]);
            let picks = choices.clone();
            Segmented::optional(opts.clone(), at.map(|i| opts[i]), move |o: Opt<'_>| {
                set(picks[o.0].0.clone())
            })
            .into()
        }
        // A date and time, and a time of day, by their pickers (docs/adr/0412 §2).
        // An instant (docs/adr/0629 §2): its date and time in the picker, its offset from UTC kept; an empty one
        // takes the device's (docs/adr/0751).
        FormWidget::Text | FormWidget::Date
            if !read_only && field.kind == LayerFieldKind::Instant =>
        {
            let (at, offset) = instant_parts(&code);
            let set_at = set.clone();
            let written = offset.clone();
            DatePicker::date_time(at, move |d: Option<DateTime>| {
                set_at(d.map_or_else(String::new, |d| instant_text(&d, &written)))
            })
            .anchor(
                kentos_ui::widget::text_field(
                    or_shape(&placeholder, "YYYY-AA-GGTSS:DD+03:00"),
                    &value,
                )
                .id(input_id(&key))
                .on_input(set),
            )
            .clearable(!field.required)
            .now(now_at(&offset))
            .into()
        }
        FormWidget::Date if !read_only && field.kind == LayerFieldKind::DateTime => {
            let at = code
                .split_once('T')
                .and_then(|(d, t)| Some(DateTime::new(parse_date(d)?, Time::parse(t)?)));
            let set_at = set.clone();
            DatePicker::date_time(at, move |d: Option<DateTime>| {
                set_at(d.map_or_else(String::new, |d| {
                    format!(
                        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                        d.date.year(),
                        d.date.month(),
                        d.date.day(),
                        d.time.hour(),
                        d.time.minute(),
                        d.time.second()
                    )
                }))
            })
            .anchor(
                kentos_ui::widget::text_field(or_shape(&placeholder, "GG.AA.YYYY SS:DD"), &value)
                    .id(input_id(&key))
                    .on_input(set),
            )
            .clearable(!field.required)
            .now(device_now())
            .into()
        }
        FormWidget::Date if !read_only && field.kind == LayerFieldKind::Time => {
            let set_at = set.clone();
            TimePicker::new(Time::parse(&code), move |t: Option<Time>| {
                set_at(t.map_or_else(String::new, |t| {
                    format!("{:02}:{:02}:{:02}", t.hour(), t.minute(), t.second())
                }))
            })
            .anchor(
                kentos_ui::widget::text_field(or_shape(&placeholder, "SS:DD"), &value)
                    .id(input_id(&key))
                    .on_input(set),
            )
            .clearable(!field.required)
            .now(device_now())
            .into()
        }
        FormWidget::Date if !read_only => {
            let date = parse_date(&code);
            let set_date = set.clone();
            DatePicker::date(date, move |d: Option<Date>| {
                set_date(d.map_or_else(String::new, |d| {
                    format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
                }))
            })
            .anchor(
                kentos_ui::widget::text_field(or_shape(&placeholder, "YYYY-AA-GG"), &value)
                    .id(input_id(&key))
                    .on_input(set),
            )
            .clearable(!field.required)
            .now(device_now())
            .into()
        }
        // A slider between the field's least and most (docs/adr/0406 §2); the value beside it.
        FormWidget::Slider if !read_only => {
            let (lo, hi) = (
                field
                    .min
                    .as_deref()
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(0.0),
                field
                    .max
                    .as_deref()
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(100.0),
            );
            let places = if field.kind == LayerFieldKind::Integer {
                0
            } else {
                field.scale.unwrap_or(2).min(6) as usize
            };
            let step = 10f64.powi(-(places as i32));
            let now = value.trim().parse::<f64>().unwrap_or(lo).clamp(lo, hi);
            let set_value = set.clone();
            row![
                iced::widget::slider(lo..=hi.max(lo), now, move |v| {
                    set_value(format!("{v:.places$}"))
                })
                .step(step)
                .style(style::field::slider)
                .width(Fill),
                container(label::body(if value.trim().is_empty() {
                    "—".to_owned()
                } else {
                    value.clone()
                }))
                .width(typography::scaled(56.0))
                .align_x(iced::alignment::Horizontal::Right),
            ]
            .spacing(10)
            .align_y(Center)
            .into()
        }
        FormWidget::Color if !read_only => colour_control(&value, placeholder, &key, set),
        // A read-only text of several lines shows them all (a workflow's log, docs/adr/0429).
        FormWidget::TextArea if read_only => container(label::body(value.clone()))
            .padding([5, 8])
            .width(Fill)
            .style(style::container::field_box)
            .into(),
        FormWidget::TextArea if !read_only => {
            let k = key.clone();
            match e.notes.get(&key) {
                Some(content) => text_editor(content)
                    .id(input_id(&key))
                    .size(typography::body())
                    .on_action(move |a| e.msg(Event::Note(k.clone(), a)))
                    .placeholder(placeholder)
                    .height(typography::scaled(72.0))
                    .padding([5, 8])
                    .into(),
                None => space::horizontal().into(),
            }
        }
        _ => {
            // Shown as the field displays it when read-only; typed as is otherwise.
            let shown = if read_only {
                at.map_or_else(|| value.clone(), |i| choices[i].1.clone())
            } else {
                value.clone()
            };
            // The layer's values offered while typing (docs/adr/0407 §3): those holding what is
            // typed, the value itself left out.
            if item.suggest && !read_only {
                let offered: Vec<&String> = e
                    .suggestions
                    .get(&item.id)
                    .map(|all| kentos_forms::suggestions_for(all, &value, 8))
                    .unwrap_or_default();
                let picked: Vec<String> = offered.iter().map(|s| (*s).clone()).collect();
                let on_pick = set.clone();
                return kentos_ui::widget::Suggest::new(value.as_str(), placeholder.as_str())
                    .id(input_id(&key))
                    .on_input(set)
                    .suggestions(
                        offered
                            .into_iter()
                            .map(|s| kentos_ui::widget::suggest::Suggestion::new(s.as_str())),
                    )
                    .on_pick(move |i| on_pick(picked.get(i).cloned().unwrap_or_default()))
                    .on_submit(e.msg(Event::Save))
                    .into();
            }
            let input = kentos_ui::widget::text_field(&placeholder, &shown).id(input_id(&key));
            if read_only {
                input.into()
            } else {
                input.on_input(set).on_submit(e.msg(Event::Save)).into()
            }
        }
    }
}

/// The record's slot by its persistent id.
pub(super) fn record_slot(doc: &Document, uid: &str) -> Option<Slot> {
    doc.slot_of(kentos_domain::Uuid::parse_str(uid).ok()?)
}

/// A colour field (docs/adr/0407 §2): the colour's swatch opens the picker, the text takes
/// `#RRGGBB` as typed.
pub(super) fn colour_control<'a>(
    value: &str,
    placeholder: String,
    key: &str,
    set: impl Fn(String) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    use kentos_ui::widget::color::{ColorPicker, parse_hex, to_hex};
    let shown = parse_hex(value).filter(|_| kentos_forms::is_colour(value));
    let size = typography::scaled(24.0);
    let face: Element<'a, Message> = container(space())
        .width(size)
        .height(size)
        .style(move |t: &iced::Theme| container::Style {
            background: shown.map(iced::Background::Color),
            border: iced::Border {
                color: Tokens::of(t).muted,
                width: 1.0,
                radius: kentos_ui::theme::shape::radius(3.0).into(),
            },
            ..container::Style::default()
        })
        .into();
    let pick = set.clone();
    let picker = ColorPicker::new(shown.unwrap_or(iced::Color::BLACK), move |c| {
        pick(to_hex(c))
    })
    .alpha()
    .anchor(face);
    let input = kentos_ui::widget::text_field(
        if placeholder.is_empty() {
            "#RRGGBB"
        } else {
            &placeholder
        },
        value,
    )
    .id(input_id(key))
    .on_input(set);
    row![picker, input].spacing(8).align_y(Center).into()
}

/// An instant's date and time of day and its offset from UTC (`Z`, `+03:00`): the offset an empty one takes is the
/// device's (docs/adr/0751).
pub(super) fn instant_parts(code: &str) -> (Option<DateTime>, String) {
    let local = || {
        let east = kentos_interaction::time_version::LOCAL_OFFSET
            .load(std::sync::atomic::Ordering::Relaxed);
        offset_text(east)
    };
    let Some((date, rest)) = code.split_once('T') else {
        return (None, local());
    };
    // The time of day, a fraction dropped, then the offset.
    let clock: String = rest.chars().take(8).collect();
    let mut tail = &rest[clock.len().min(rest.len())..];
    if let Some(after) = tail.strip_prefix('.') {
        tail = after.trim_start_matches(|c: char| c.is_ascii_digit());
    }
    let offset = if tail.is_empty() {
        local()
    } else {
        tail.to_owned()
    };
    let at = parse_date(date).and_then(|d| Some(DateTime::new(d, Time::parse(&clock)?)));
    (at, offset)
}

/// Now on the device's clock: a date's or a time's Şimdi and today, they have no zone (docs/adr/0751; the picker's
/// own is UTC's).
pub(super) fn device_now() -> DateTime {
    let east =
        kentos_interaction::time_version::LOCAL_OFFSET.load(std::sync::atomic::Ordering::Relaxed);
    DateTime::now(east / 60)
}

/// Now at an instant's offset (`Z`, `+03:00`): the moment Şimdi writes is the present one.
pub(super) fn now_at(offset: &str) -> DateTime {
    DateTime::now(offset_minutes(offset).unwrap_or(0))
}

/// An instant's offset (`Z`, `±SS:DD`) in minutes east of UTC.
pub(super) fn offset_minutes(offset: &str) -> Option<i32> {
    if offset == "Z" {
        return Some(0);
    }
    let sign = match offset.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let (h, m) = offset[1..].split_once(':')?;
    Some(sign * (h.parse::<i32>().ok()? * 60 + m.parse::<i32>().ok()?))
}

/// Seconds east of UTC as an instant's offset: `Z` at none, else `±SS:DD`.
pub(super) fn offset_text(east: i32) -> String {
    if east == 0 {
        return "Z".into();
    }
    let sign = if east < 0 { '-' } else { '+' };
    let m = east.unsigned_abs() / 60;
    format!("{sign}{:02}:{:02}", m / 60, m % 60)
}

/// An instant's text from a picked date and time and `offset`.
pub(super) fn instant_text(d: &DateTime, offset: &str) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{offset}",
        d.date.year(),
        d.date.month(),
        d.date.day(),
        d.time.hour(),
        d.time.minute(),
        d.time.second()
    )
}

pub(super) fn parse_date(text: &str) -> Option<Date> {
    let mut parts = text.split('-');
    let y = parts.next()?.parse().ok()?;
    let m = parts.next()?.parse().ok()?;
    let d = parts.next()?.parse().ok()?;
    Date::new(y, m, d)
}
