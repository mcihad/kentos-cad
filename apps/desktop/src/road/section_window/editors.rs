//! Enkesit's window on screen: the set's and the template's editors, the drawing row and the controls
//! found by their words.

use super::*;
use iced::widget::column;

impl App {
    pub(crate) fn road_section_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.road.section_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style {
            color: Some(Tokens::of(t).muted),
        };
        let (alignments, _, _) = self.section_names();
        let revisions: Vec<(String, u32)> = self.document.as_ref().map_or_else(Vec::new, |d| {
            d.model.road_alignments().iter().map(|a| (a.id.clone(), a.revision)).collect()
        });
        let stale_of = |r: &SetRow| {
            w.kept_sets.iter().any(|k| k.id == r.id)
                && revisions.iter().find(|(a, _)| *a == r.alignment).is_some_and(|(_, v)| *v != r.alignment_revision)
        };
        let item = |title: String, kind: String, on: bool, event: Event| -> Element<'_, Message> {
            let face = column![
                Elided::new(title).size(typography::body()).font(typography::ui()).width(Fill),
                label::caption(kind).style(muted),
            ]
            .spacing(1);
            iced::widget::button(face).on_press(msg(event)).padding([4, 8]).width(Fill).style(style::button::row(on)).into()
        };
        let mut sets = Column::new().spacing(1).padding(2);
        for (i, r) in w.sets.iter().enumerate() {
            let on = alignments.iter().find(|(id, _)| *id == r.alignment).map_or(r.alignment.clone(), |(_, n)| n.clone());
            let kind = if stale_of(r) { format!("{on} · eski") } else { on };
            sets = sets.push(item(r.name.clone(), kind, w.chosen == Some(Chosen::Set(i)), Event::Choose(Chosen::Set(i))));
        }
        if w.sets.is_empty() {
            sets = sets.push(label::caption("Takım yok. Yeni takım ile bir aksın km'lerinde kesit hatları kurun.").style(muted));
        }
        let mut templates = Column::new().spacing(1).padding(2);
        for (i, t) in w.templates.iter().enumerate() {
            let n = t.left.parts.len();
            let kind = format!("{n} bileşen{}", if t.mirror { " · simetrik" } else { " · asimetrik" });
            templates = templates.push(item(t.name.clone(), kind, w.chosen == Some(Chosen::Template(i)), Event::Choose(Chosen::Template(i))));
        }
        if w.templates.is_empty() {
            templates = templates.push(label::caption("Tip kesit yok.").style(muted));
        }
        let left = column![
            label::strong("Kesit takımları"),
            container(scrollable(sets).height(typography::scaled(96.0))).style(style::container::field_box).width(Fill),
            words::secondary(NEW_SET, Some(msg(Event::NewSet))),
            label::strong("Tip kesitler"),
            container(scrollable(templates).height(typography::scaled(84.0))).style(style::container::field_box).width(Fill),
            words::secondary(NEW_TEMPLATE, Some(msg(Event::NewTemplate))),
        ]
        .spacing(8);
        // The editors are not scrolled on their own: the window's body scrolls as one and, resized or full screen,
        // grows while the buttons stay on the bottom edge (a footer, DESIGN.md §7.19).
        let right: Element<'_, Message> = match w.chosen {
            Some(Chosen::Set(i)) if i < w.sets.len() => self.section_set_editor(w, &w.sets[i], stale_of(&w.sets[i])),
            Some(Chosen::Template(i)) if i < w.templates.len() => section_template_editor(w, &w.templates[i]),
            _ => container(label::caption("Bir kesit takımı ya da tip kesit seçin ya da yeni birini oluşturun.")).padding(12).into(),
        };
        let saved_set = match w.chosen {
            Some(Chosen::Set(i)) => w.sets.get(i).and_then(|r| r.def().ok()).is_some_and(|d| w.kept_sets.contains(&d)),
            _ => false,
        };
        let stale = matches!(w.chosen, Some(Chosen::Set(i)) if w.sets.get(i).is_some_and(stale_of));
        // The drawing row and the rebind are a set's: a template has neither (DESIGN.md §7.19, fewer dead controls).
        let on_set = matches!(w.chosen, Some(Chosen::Set(_)));
        let main = row![container(left).width(220), container(right).width(Fill)].spacing(16);
        let mut frame = Frame::new(TITLE).scroll(main);
        if on_set {
            frame = frame.push(self.section_draw_row(w, saved_set));
        }
        frame
            .aside(words::ghost(REMOVE, w.chosen.map(|_| msg(Event::Remove))))
            .aside(words::secondary(REBIND, (saved_set && stale).then(|| msg(Event::Rebind))))
            .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
            .action(words::primary(SAVE, Some(msg(Event::Save))))
            .width(1080.0)
            .modal(msg(Event::Cancel))
    }

    pub(super) fn section_draw_row<'a>(
        &'a self,
        w: &'a Window,
        saved: bool,
    ) -> Element<'a, Message> {
        let ids: Vec<String> = self
            .relation_layers()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        let names: Vec<String> = self.relation_layers().into_iter().map(|(_, n)| n).collect();
        let layer = Select::new(
            names.iter().map(|n| Choice::new(n.clone())),
            w.layer.as_ref().and_then(|l| ids.iter().position(|id| id == l)),
            move |i| msg(Event::DrawLayer(ids[i].clone())),
        )
        .placeholder("Katman");
        row![
            label::strong("Çizime"),
            words::field("Katman", container(layer).width(220), None),
            words::field("Yer (Y; X)", kentos_ui::widget::text_field("Çizimin altı", &w.place).on_input(|t| msg(Event::Place(t))).width(200), None),
            words::field("Satırda", kentos_ui::widget::text_field("5", &w.columns).on_input(|t| msg(Event::Columns(t))).width(60), None),
            words::secondary(DRAW, saved.then(|| msg(Event::Draw))),
            words::field("Pafta 1:", kentos_ui::widget::text_field("500", &w.sheet_scale).on_input(|t| msg(Event::SheetScale(t))).width(70), None),
            words::secondary(SHEETS, saved.then(|| msg(Event::Sheets))),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom)
        .into()
    }

    pub(super) fn section_set_editor<'a>(
        &'a self,
        w: &'a Window,
        r: &'a SetRow,
        stale: bool,
    ) -> Element<'a, Message> {
        let muted = |t: &Theme| iced::widget::text::Style {
            color: Some(Tokens::of(t).muted),
        };
        let (alignments, profiles, surfaces) = self.section_names();
        let pick = |list: Vec<(String, String)>, now: Option<&String>, none: bool, make: fn(Option<String>) -> Event| {
            let ids: Vec<String> = list.iter().map(|(id, _)| id.clone()).collect();
            let at = now.and_then(|n| ids.iter().position(|id| id == n));
            let choices: Vec<Choice> = if none {
                std::iter::once(Choice::new("Yok".to_owned())).chain(list.iter().map(|(_, n)| Choice::new(n.clone()))).collect()
            } else {
                list.iter().map(|(_, n)| Choice::new(n.clone())).collect()
            };
            let shown = if none { Some(at.map_or(0, |i| i + 1)) } else { at };
            Select::new(choices, shown, move |i| {
                msg(make(if none { (i > 0).then(|| ids[i - 1].clone()) } else { Some(ids[i].clone()) }))
            })
        };
        let on_alignment = pick(alignments, Some(&r.alignment), false, |id| Event::Alignment(id.unwrap_or_default()));
        let on_profile = pick(
            profiles.into_iter().filter(|p| p.2 == r.alignment).map(|p| (p.0, p.1)).collect(),
            r.profile.as_ref(),
            true,
            Event::Profile,
        );
        let on_template = pick(
            w.templates.iter().map(|t| (t.id.clone(), t.name.clone())).collect(),
            r.template.as_ref(),
            true,
            Event::Template,
        );
        let on_surface = pick(surfaces, r.surface.as_ref(), true, Event::Surface);
        let schedules = self.document.as_ref().map_or_else(Vec::new, |d| {
            d.model
                .road_superelevations()
                .iter()
                .filter(|s| s.alignment == r.alignment)
                .map(|s| (s.id.clone(), s.name.clone()))
                .collect()
        });
        let on_superelevation = pick(schedules, r.superelevation.as_ref(), true, Event::Superelevation);
        let field = |hint: &'a str, value: &'a str, k: usize, width: f32| {
            kentos_ui::widget::text_field(hint, value).on_input(move |t| msg(Event::SetField(k, t))).width(width)
        };
        // The section and its plan take what a resized or full screen window has over (docs/adr/0937 §2).
        let mut col = kentos_ui::widget::grow::Grows::new().spacing(10.0);
        col = col.push(
            row![
                words::field("Ad", kentos_ui::widget::text_field("Takımın adı", &r.name).on_input(|t| msg(Event::Name(t))), None),
                words::field("Aks", on_alignment, None),
                words::field("Boykesit", on_profile, None),
                words::field("Tip kesit", on_template, None),
                words::field("Yüzey", on_surface, None),
            ]
            .spacing(10),
        );
        let method = Select::new(
            METHODS.iter().map(|m| Choice::new((*m).to_owned())),
            Some(r.method),
            |m| msg(Event::Method(m)),
        );
        // The swath and the method on one row, the method's own fields on the next: at 1100 px one row squeezed the
        // method's select to nothing.
        col = col.push(
            row![
                words::field("Sol (m)", field("25", &r.left, 0, 70.0), None),
                words::field("Sağ (m)", field("25", &r.right, 1, 70.0), None),
                words::field("Dever", container(on_superelevation).width(150), None),
                words::field("Km", container(method).width(170), None),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Bottom),
        );
        let mut stations = row![].spacing(10).align_y(iced::alignment::Vertical::Bottom);
        stations = match r.method {
            0 => stations
                .push(words::field("Baş (m)", field("0", &r.from, 2, 80.0), None))
                .push(words::field("Son (m)", field("", &r.to, 3, 80.0), None))
                .push(words::field("Her (m)", field("20", &r.every, 4, 60.0), None))
                .push(words::field("Kurpta (m)", field("yok", &r.curve, 6, 60.0), None))
                .push(words::check(r.geometry, "Kurp noktaları ve PVI'ler", Some(msg(Event::Geometry)))),
            3 => stations
                .push(words::field("Baş (m)", field("0", &r.from, 2, 80.0), None))
                .push(words::field("Son (m)", field("", &r.to, 3, 80.0), None))
                .push(words::field("Tolerans (m)", field("0,1", &r.tolerance, 7, 70.0), None))
                .push(words::field("En çok (m)", field("yok", &r.most, 8, 60.0), None))
                .push(words::check(r.geometry, "Kurp noktaları ve PVI'ler", Some(msg(Event::Geometry)))),
            1 => stations.push(words::field("Km'ler (m; ile)", field("100; 150", &r.stations, 5, 300.0), None)),
            _ => stations.push(label::caption(format!("{} çizginin aksı kestiği yerler", r.uids.len())).style(muted)),
        };
        col = col.push(stations);
        let hint = match r.method {
            0 => "Kurpta: kurpta ve klotoidde bu aralıkla da kesit alınır.",
            3 => "Aksın yüzeyin üçgen kenarlarını kestiği kırıklar; iki kesit arasında doğrusal okunan eksen arazisi en çok tolerans kadar şaşar. Yan arazinin kırıklarını garanti etmez.",
            _ => "",
        };
        // Why to clip, next to the box: the crossing said under the charts was below the fold.
        let why: Element<'_, Message> = match &w.preview.crossing {
            Some(c) => label::caption(c.clone()).style(|t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) }).into(),
            None => label::caption(hint).style(muted).into(),
        };
        col = col.push(row![words::check(r.clip, CLIP, Some(msg(Event::Clip))), why].spacing(16).align_y(Center));
        let shown = w.preview.rows.get(w.at);
        let exaggeration = number(&w.exaggeration).ok().flatten().filter(|v| *v > 0.0);
        let name = shown.map_or_else(|| "Enkesit".to_owned(), |s| format!("Enkesit — km {}", s.km_text));
        col = col.push_grow(
            row![
                crate::picture::picture(
                    iced::widget::canvas(Chart { row: shown, sample: None, exaggeration, layers: None }).width(Fill).height(Fill),
                    name,
                ),
                container(crate::picture::picture(
                    iced::widget::canvas(Plan { rows: &w.preview.rows, at: w.at }).width(Fill).height(Fill),
                    "Enkesit hatları",
                ))
                .width(typography::scaled(260.0))
                .height(Fill),
            ]
            .spacing(10)
            .height(Fill),
            typography::scaled(CHART_HEIGHT),
        );
        let n = w.preview.rows.len();
        let place = shown.map_or_else(|| "Kesit yok".to_owned(), |s| format!("km {} · {} / {n}", s.km_text, w.at + 1));
        let areas = shown.and_then(|s| Some((s.cut?, s.fill?))).map_or_else(String::new, |(c, f)| {
            format!("Yarma {} m² · Dolgu {} m²", comma(c, 2), comma(f, 2))
        });
        col = col.push(
            row![
                words::secondary(PREVIOUS, (w.at > 0).then(|| msg(Event::Previous))),
                label::body(place),
                words::secondary(NEXT, (w.at + 1 < n).then(|| msg(Event::Next))),
                space::horizontal().width(16),
                label::body(areas),
                space::horizontal().width(Fill),
                words::field(
                    "Düşey abartma",
                    kentos_ui::widget::text_field("oto", &w.exaggeration).on_input(|t| msg(Event::Exaggeration(t))).width(60),
                    None,
                ),
            ]
            .spacing(8)
            .align_y(Center),
        );
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        if let Some(why) = &w.preview.why {
            col = col.push(label::caption(why.clone()).style(danger));
        }
        if let Some(open) = &w.preview.open {
            col = col.push(label::caption(open.clone()).style(warn));
        }
        if let Some(missing) = &w.preview.missing {
            col = col.push(label::caption(missing.clone()).style(warn));
        }
        if let Some(sources) = &w.preview.sources {
            col = col.push(label::caption(sources.clone()).style(muted));
        }
        if stale {
            col = col.push(label::caption("Aks değişti: km'leri denetleyip aksa yeniden bağlayın.").style(warn));
        }
        if r.profile.is_none() || r.template.is_none() || r.surface.is_none() {
            col = col.push(
                label::caption("Platform için takımın boykesiti, tip kesiti ve yüzeyi olmalı; eksikken yalnız arazi çizilir.")
                    .style(muted),
            );
        }
        container(col).padding(iced::Padding::default().right(14)).into()
    }

    pub(crate) fn section_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.road.section_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CLIP) => Some(msg(Event::Clip)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW_SET) => Some(msg(Event::NewSet)),
            Control::Press(NEW_TEMPLATE) => Some(msg(Event::NewTemplate)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(REBIND) => Some(msg(Event::Rebind)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(REMOVE_PART) => Some(msg(Event::RemovePart)),
            Control::Press(PREVIOUS) => Some(msg(Event::Previous)),
            Control::Press(NEXT) => Some(msg(Event::Next)),
            Control::Press(DRAW) => Some(msg(Event::Draw)),
            Control::Press(SHEETS) => Some(msg(Event::Sheets)),
            Control::Press(PAVEMENT) => Some(msg(Event::Pavement)),
            Control::Press(ADD_LAYER) => Some(msg(Event::AddLayer)),
            Control::Press(REMOVE_LAYER) => Some(msg(Event::RemoveLayer)),
            Control::Press("Sol yan") => Some(msg(Event::Side(false))),
            Control::Press("Sağ yan") => Some(msg(Event::Side(true))),
            Control::Press("Dik") => Some(msg(Event::PavementSloped(false))),
            Control::Press("Şevli") => Some(msg(Event::PavementSloped(true))),
            Control::Fill(EDGE_SLOPE, text) => Some(msg(Event::PavementSlope(text.to_owned()))),
            Control::Press(words) => {
                if let Some(i) = w.sets.iter().position(|r| r.name == words) {
                    Some(msg(Event::Choose(Chosen::Set(i))))
                } else if let Some(i) = w.templates.iter().position(|r| r.name == words) {
                    Some(msg(Event::Choose(Chosen::Template(i))))
                } else {
                    return Err(format!("“{TITLE}” penceresinde “{words}” düğmesi yok"));
                }
            }
            Control::Fill("Ad", text) => Some(msg(Event::Name(text.to_owned()))),
            Control::Fill("Sol (m)", text) => Some(msg(Event::SetField(0, text.to_owned()))),
            Control::Fill("Sağ (m)", text) => Some(msg(Event::SetField(1, text.to_owned()))),
            Control::Fill("Kurpta (m)", text) => Some(msg(Event::SetField(6, text.to_owned()))),
            Control::Fill("Tolerans (m)", text) => Some(msg(Event::SetField(7, text.to_owned()))),
            Control::Fill("En çok (m)", text) => Some(msg(Event::SetField(8, text.to_owned()))),
            Control::Fill("Düşey abartma", text) => Some(msg(Event::Exaggeration(text.to_owned()))),
            Control::Fill("Yer (Y; X)", text) => Some(msg(Event::Place(text.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The template editor (docs/adr/0975 §3, DESIGN.md §7.19): the template on a sample ground over two grouped columns:
/// the template and its end condition on the left; the side's components and the pavement on the right.
pub(super) fn section_template_editor<'a>(
    w: &'a Window,
    t: &'a TemplateRow,
) -> Element<'a, Message> {
    use kentos_ui::widget::{Form, Segmented};
    let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
    let editing_right = w.right && !t.mirror;
    let side = if editing_right { &t.right } else { &t.left };
    // The look on a sloping ground: a cut on the left, a fill on the right; the pavement under it.
    let def = t.def();
    let benched = |s: &TemplateSideDef| s.bench.is_some() || s.fill_bench.is_some();
    let benched = def.as_ref().is_ok_and(|d| benched(&d.left) || d.right.as_ref().is_some_and(benched));
    let ground = if benched { sample_ground_benched() } else { sample_ground_near() };
    let built = def.as_ref().ok().and_then(|d| {
        let core = road_sections::core_template(d);
        let sec = tpl::build(&core, 100.0, &ground).ok()?;
        let pav = d.pavement.as_ref().map(|p| kentos_road::section::pavement::build(&core, &sec, &road_volumes_core_pavement(p), &ground));
        Some((sec, pav))
    });
    let sample = built.as_ref().map(|(s, _)| (ground.clone(), s.clone()));
    let layers = built.as_ref().and_then(|(_, p)| p.as_ref()).and_then(|p| p.as_ref().ok()).map(|p| p.bottoms.clone());
    let exaggeration = number(&w.exaggeration).ok().flatten().filter(|v| *v > 0.0);
    let picture = crate::picture::picture(
        iced::widget::canvas(Chart { row: None, sample, exaggeration, layers }).width(Fill).height(Fill),
        format!("Tip kesit — {}", def.as_ref().map_or("", |d| d.name.as_str())),
    );
    let summary: Element<'a, Message> = match (&built, &def) {
        (_, Err(e)) => label::caption(e.clone()).style(muted).into(),
        (Some((s, pav)), _) => {
            let mut words = format!(
                "Örnek arazide: sol {} · sağ {} · yarma {} m² · dolgu {} m²",
                kind_words(s.left.kind),
                kind_words(s.right.kind),
                comma(s.cut, 2),
                comma(s.fill, 2)
            );
            match pav {
                Some(Ok(p)) => words.push_str(&format!(
                    " · üstyapı {} m² · formasyondan yarma {} m², dolgu {} m²",
                    comma(p.area(), 3),
                    comma(s.cut + p.below, 2),
                    comma(s.fill - p.above, 2)
                )),
                Some(Err(_)) => words.push_str(" · üstyapı kurulmuyor: kenar şevi banketten yatık"),
                None => {}
            }
            label::caption(words).into()
        }
        (None, Ok(_)) => label::caption("Tip kesitin değerleri kurallara uymuyor.").style(muted).into(),
    };

    // Left: the template and its pavement (both sides alike).
    let end = |hint: &'a str, value: &'a str, k: usize| kentos_ui::widget::text_field(hint, value).on_input(move |v| msg(Event::End(k, v)));
    let shown = if editing_right { SideChoice::Right } else { SideChoice::Left };
    let sides = Segmented::new_with([SideChoice::Left, SideChoice::Right], shown, |c| msg(Event::Side(c == SideChoice::Right)), |c| {
        c == SideChoice::Left || !t.mirror
    });
    let pv = &t.pavement;
    let mut form = Form::new()
        .label_width(110.0)
        .section("Tip kesit")
        .field("Ad", kentos_ui::widget::text_field("Tip kesitin adı", &t.name).on_input(|v| msg(Event::Name(v))))
        .field("Yan", sides)
        .row(words::check(t.mirror, "Sağ yan solun aynısı", Some(msg(Event::Mirror))))
        .section("Üstyapı (iki yanda aynı)")
        .row(words::check(pv.on, PAVEMENT, Some(msg(Event::Pavement))));
    if pv.on {
        let edge = Segmented::new(
            [EdgeChoice::Vertical, EdgeChoice::Slope],
            if pv.sloped { EdgeChoice::Slope } else { EdgeChoice::Vertical },
            |c| msg(Event::PavementSloped(c == EdgeChoice::Slope)),
        );
        form = form.field("Kenar", edge);
        if pv.sloped {
            form = form
                .field(EDGE_SLOPE, kentos_ui::widget::text_field("1,5", &pv.h).on_input(|v| msg(Event::PavementSlope(v))))
                .help("Dolgu şevi verilirse kenar dolgunun yüzü olur.");
        }
        let layers = &pv.layers;
        let table = road_dialog::grid_table(
            vec![
                road_dialog::Col { title: "Tabaka", width: None, number: false },
                road_dialog::col("Kalınlık (m)", 100.0),
                road_dialog::col("Malzeme kodu", 130.0),
            ],
            layers.len(),
            move |j, k| road_dialog::edit(&layers[j][k], k == 1, move |v| msg(Event::Layer(j, k, v))),
            "Tabaka yok",
        );
        let total: f64 = pv.layers.iter().filter_map(|l| number(&l[1]).ok().flatten()).sum();
        form = form.wide(table).wide(
            row![
                words::secondary(ADD_LAYER, Some(msg(Event::AddLayer))),
                words::ghost(REMOVE_LAYER, (pv.layers.len() > 1).then(|| msg(Event::RemoveLayer))),
                space::horizontal().width(Fill),
                label::caption(format!("Toplam {} m", comma(total, 2))).style(muted),
            ]
            .spacing(8)
            .align_y(Center),
        );
    }

    // Right: the side's components and its end condition.
    let side_parts = &side.parts;
    let parts = road_dialog::grid_table(
        vec![
            road_dialog::Col { title: "Bileşen", width: None, number: false },
            road_dialog::col("Genişlik (m)", 110.0),
            road_dialog::col("Enine eğim (%)", 120.0),
        ],
        side_parts.len(),
        move |j, k| {
            let p = &side_parts[j];
            match k {
                0 => road_dialog::pick(KINDS, Some(p.kind), move |c| msg(Event::PartKind(j, c))),
                1 => road_dialog::edit(&p.width, true, move |v| msg(Event::Part(j, 0, v))),
                _ => road_dialog::edit(&p.slope, true, move |v| msg(Event::Part(j, 1, v))),
            }
        },
        "Bileşen yok: şerit ya da banket ekleyin",
    );
    let mut right = Form::new()
        .label_width(110.0)
        .section(if editing_right { "Sağ yanın bileşenleri (eksenden dışa)" } else { "Sol yanın bileşenleri (eksenden dışa)" })
        .wide(parts)
        .wide(
            row![
                words::secondary(ADD, Some(msg(Event::Add))),
                words::ghost(REMOVE_PART, (!side.parts.is_empty()).then(|| msg(Event::RemovePart))),
            ]
            .spacing(8),
        )
        .section(if editing_right { "Sağ yanın uç koşulu" } else { "Sol yanın uç koşulu" })
        .field("Yarma şevi (y/d)", end("1", &side.cut, 0))
        .field("Dolgu şevi (y/d)", end("1,5", &side.fill, 1))
        .row(words::check(side.ditch, "Yarmada hendek", Some(msg(Event::Ditch))));
    if side.ditch {
        right = right
            .field("İç şev (y/d)", end("2", &side.fore, 2))
            .field("Derinlik (m)", end("0,5", &side.depth, 3))
            .field("Taban (m)", end("0", &side.bottom, 4))
            .help("Boş taban: V hendek.");
    }
    // The cut slope's benches (docs/adr/0958): from the geotechnical report; KentOS gives no number.
    right = right.row(words::check(side.bench, "Yarma şevinde basamak (berme)", Some(msg(Event::Bench))));
    if side.bench {
        right = right
            .field("Basamak yüksekliği (m)", end("6", &side.bench_fields[0], 5))
            .field("Basamak genişliği (m)", end("3", &side.bench_fields[1], 6))
            .field("Basamak eğimi (%)", end("0", &side.bench_fields[2], 7))
            .help("Şev her basamak yüksekliğinde kesilir, araya dışa doğru düşen basamak girer. Basamaklı şevin ortalama eğimi h/(h·m + w): arazi ondan dik yükseliyorsa şev araziye varmaz.");
    }
    right = right.row(words::check(side.fill_bench, "Dolgu şevinde basamak (berme)", Some(msg(Event::FillBench))));
    if side.fill_bench {
        right = right
            .field("Dolgu basamağı yüksekliği (m)", end("6", &side.fill_bench_fields[0], 8))
            .field("Dolgu basamağı genişliği (m)", end("3", &side.fill_bench_fields[1], 9))
            .field("Dolgu basamağı eğimi (%)", end("0", &side.fill_bench_fields[2], 10))
            .help("Dolgu şevi her basamak yüksekliğinde iner, araya dışa doğru düşen basamak girer. Arazi basamaklı şevin ortalama eğiminden dik düşüyorsa şev araziye varmaz.");
    }
    // The picture takes what a resized or full screen window has over (docs/adr/0937 §2).
    let col = kentos_ui::widget::grow::Grows::new()
        .spacing(10.0)
        .push_grow(picture, typography::scaled(170.0))
        .push(summary)
        .push(row![container(form).width(typography::scaled(400.0)), container(right).width(Fill)].spacing(typography::scaled(20.0)))
        .push(label::caption("Değerler KGM'nin standardından ya da projeden yazılır; KentOS öntanımlı sayı vermez.").style(muted));
    container(col).padding(iced::Padding::default().right(14)).into()
}
