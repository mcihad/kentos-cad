//! An opening (docs/adr/0341 §5): its form, type, width, place along its wall, side, hinge, leaves, operation,
//! swing, frame, sill, head and number, each written through `cad.openings.edit`; its wall, or why it is in none.

use super::*;

/// An opening's own rows, under the general ones: what `entity_sections` shows for it.
pub(super) fn rows(k: &Kit, o: &kentos_commands::OpeningEntity) -> Vec<Row> {
    let doc = k.doc;
    let slot = k.slot;
    let f = k.f;
    let edit = |editor: Editor| k.edit(editor);
    let number = |field: Field| k.number(field);
    let metres = |label: &'static str, value: f64| k.metres(label, value);
    let mut geo = Vec::new();
    use kentos_commands::{OpeningChange, OpeningForm, OpeningHand, OpeningOperation, OpeningSide, OpeningsOperation};
    let x = &o.opening;
    let set = |c: OpeningChange| Message::Properties(Event::Opening(slot, OpeningsOperation::Set(c)));
    let pick = |label: String, chosen: bool, message: Message, icon: Option<&'static str>| Choice::Pick {
        label,
        swatch: None,
        icon,
        wide: false,
        chosen,
        enabled: true,
        message,
    };
    let select = |text: String, items: Vec<Choice>| {
        edit(Editor::Select {
            text,
            swatch: None,
            icon: None,
            items,
        })
    };
    let forms = OpeningForm::ALL
        .iter()
        .map(|&v| {
            let icon = match v {
                OpeningForm::Door => "door",
                OpeningForm::Window => "window",
                OpeningForm::Opening => "wallOpening",
                OpeningForm::Niche => "wallNiche",
                OpeningForm::Flue => "wallFlue",
            };
            pick(v.label().to_owned(), v == x.form, set(OpeningChange { form: Some(v), ..Default::default() }), Some(icon))
        })
        .collect();
    geo.push(Row::text("Biçim", x.form.label()).editor(select(x.form.label().to_owned(), forms)));
    // The project's types first (docs/adr/0772 §1), then KentOS's own, copied in.
    let project: Vec<(String, String)> = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.opening_types.iter().filter(|t| t.form == x.form).map(|t| (t.id.clone(), t.name.clone())).collect())
        .unwrap_or_default();
    let builtin: Vec<(String, String)> =
        kentos_commands::builtin_opening_types().into_iter().map(|t| (t.id, t.name)).collect();
    let type_name = x
        .style
        .as_deref()
        .and_then(|id| project.iter().chain(builtin.iter()).find(|t| t.0 == id))
        .map_or_else(|| "Yok".to_owned(), |t| t.1.clone());
    let project_count = project.len();
    let mut type_items: Vec<Choice> = project
        .iter()
        .chain(builtin.iter())
        .map(|t| {
            pick(
                t.1.clone(),
                x.style.as_deref() == Some(t.0.as_str()),
                Message::Properties(Event::Opening(
                    slot,
                    OpeningsOperation::ReplaceType { from: None, to: t.0.clone(), level: None },
                )),
                None,
            )
        })
        .collect();
    if project_count > 0 {
        type_items.insert(project_count, Choice::Separator);
    }
    type_items.push(Choice::Separator);
    type_items.push(Choice::Command {
        label: "Kapı ve pencere tipleri…",
        icon: crate::icons::from_web(Some("openingTypes")),
        id: "arch.openingTypes",
    });
    geo.push(Row::text("Tür", type_name.clone()).editor(select(type_name, type_items)));
    geo.push(metres("Genişlik", x.width).editor(number(Field::OpeningWidth(slot))));
    geo.push(metres("Konum", x.at).editor(number(Field::OpeningAt(slot))));
    // Its own phase (docs/adr/8039 §1): a new door cut into an existing wall, an existing window closed up.
    // Duvarınki: its wall's, which it says.
    let wall_phase = kentos_domain::Uuid::parse_str(&x.host)
        .ok()
        .and_then(|u| doc.model.slot_of(u))
        .and_then(|s| doc.model.get(s))
        .and_then(Entity::own_phase);
    let walls = wall_phase.map_or_else(
        || "Duvarınki".to_owned(),
        |p| format!("Duvarınki ({})", p.label()),
    );
    let phase_name = x
        .phase
        .map_or_else(|| walls.clone(), |p| p.label().to_owned());
    let mut phases = vec![pick(
        walls,
        x.phase.is_none(),
        set(OpeningChange {
            phase: Some(None),
            ..Default::default()
        }),
        None,
    )];
    phases.extend(kentos_commands::ArchPhase::ALL.iter().map(|&p| {
        let change = set(OpeningChange {
            phase: Some(Some(p)),
            ..Default::default()
        });
        pick(
            p.label().to_owned(),
            x.phase == Some(p),
            change,
            None,
        )
    }));
    geo.push(Row::text("Tadilat durumu", phase_name.clone()).editor(select(phase_name, phases)));
    // A niche's or flue's face, depth and a flue's distance from its face (docs/adr/0719).
    if x.form.is_recess() {
        let sides = [(OpeningSide::Left, "Sol", "openingSideLeft"), (OpeningSide::Right, "Sağ", "openingSideRight")]
            .into_iter()
            .map(|(v, l, i)| pick(l.to_owned(), v == x.side, set(OpeningChange { side: Some(v), ..Default::default() }), Some(i)))
            .collect();
        let side = if x.side == OpeningSide::Right { "Sağ" } else { "Sol" };
        geo.push(Row::text("Yüz", side).editor(select(side.to_owned(), sides)));
        match x.depth {
            Some(d) => geo.push(metres("Derinlik", d).editor(number(Field::OpeningDepth(slot)))),
            None => geo.push(Row::figure("Derinlik", "Yok").editor(number(Field::OpeningDepth(slot)))),
        }
        if x.form == OpeningForm::Flue {
            match x.inset {
                Some(v) => geo.push(metres("Yüzden uzaklık", v).editor(number(Field::OpeningInset(slot)))),
                None => geo.push(Row::text("Yüzden uzaklık", "Ortada").editor(number(Field::OpeningInset(slot)))),
            }
        }
    }
    if x.form == OpeningForm::Door {
        let sides = [(OpeningSide::Left, "Sol", "openingSideLeft"), (OpeningSide::Right, "Sağ", "openingSideRight")]
            .into_iter()
            .map(|(v, l, i)| pick(l.to_owned(), v == x.side, set(OpeningChange { side: Some(v), ..Default::default() }), Some(i)))
            .collect();
        let side = if x.side == OpeningSide::Right { "Sağ" } else { "Sol" };
        geo.push(Row::text("Yan", side).editor(select(side.to_owned(), sides)));
        let hands = [(OpeningHand::Start, "Baş"), (OpeningHand::End, "Son")]
            .into_iter()
            .map(|(v, l)| pick(l.to_owned(), v == x.hand, set(OpeningChange { hand: Some(v), ..Default::default() }), None))
            .collect();
        let hand = if x.hand == OpeningHand::End { "Son" } else { "Baş" };
        geo.push(Row::text("Menteşe", hand).editor(select(hand.to_owned(), hands)));
    }
    if matches!(x.form, OpeningForm::Door | OpeningForm::Window) {
        // Its leaves as its operation allows (docs/adr/0777): a revolving door's wings, a folding one's panels.
        let (least, most) = x.operation.leaf_range();
        let leaves = (least..=most)
            .map(|n| pick(n.to_string(), n == x.leaves, set(OpeningChange { leaves: Some(n), ..Default::default() }), None))
            .collect();
        let word = if x.operation == OpeningOperation::Folding { "Panel" } else { "Kanat" };
        geo.push(Row::figure(word, x.leaves.to_string()).editor(select(x.leaves.to_string(), leaves)));
        // A revolving, pocket or folding door's operation only on a door; choosing it takes its leaves into
        // its range (three wings, two panels).
        let ops = OpeningOperation::ALL
            .iter()
            .filter(|o| x.form == OpeningForm::Door || !o.is_special())
            .map(|&v| {
                let (least, most) = v.leaf_range();
                let leaves = (!(least..=most).contains(&x.leaves)).then_some(least);
                pick(v.label().to_owned(), v == x.operation, set(OpeningChange { operation: Some(v), leaves, ..Default::default() }), None)
            })
            .collect();
        geo.push(Row::text("Açılış", x.operation.label()).editor(select(x.operation.label().to_owned(), ops)));
        // A revolving door's drum, a pocket door's lip and pockets, a folding door's both sides, a sliding
        // door's overlap and track (docs/adr/0777 §5): edited by `cad.openings.edit`'s `set` (empty: none).
        {
            use kentos_commands::{OpeningBuild, OpeningTypeField};
            let b = x.build.clone().unwrap_or_default();
            let sizes = x.sizes();
            let length = |label: &'static str, field: OpeningTypeField, own: Option<f64>, otherwise: String| {
                let editor = number(Field::OpeningBuild(slot, field));
                match own {
                    Some(v) => metres(label, v).editor(editor),
                    None => Row::figure(label, otherwise).editor(editor),
                }
            };
            let switch = |label: &'static str, field: OpeningTypeField, on: bool| {
                let change = |v: bool| {
                    if v {
                        let mut b = OpeningBuild::default();
                        if field == OpeningTypeField::Track {
                            b.track = Some(true);
                        } else {
                            b.both_sides = Some(true);
                        }
                        OpeningChange { build: Some(Box::new(b)), ..Default::default() }
                    } else {
                        OpeningChange { unset: vec![field], ..Default::default() }
                    }
                };
                let picks = [(true, "Evet"), (false, "Hayır")]
                    .into_iter()
                    .map(|(v, l)| pick(l.to_owned(), v == on, set(change(v)), None))
                    .collect();
                let shown = if on { "Evet" } else { "Hayır" };
                Row::text(label, shown).editor(select(shown.to_owned(), picks))
            };
            match (x.form, x.operation) {
                (OpeningForm::Door, OpeningOperation::Revolving) => {
                    let light = format!("{} · ışık", f.length(sizes.leaf));
                    geo.push(length("Tambur çapı", OpeningTypeField::Drum, b.drum, light));
                }
                (OpeningForm::Door, OpeningOperation::Pocket) => {
                    let lip = format!("{} · öntanımlı", f.length(kentos_commands::POCKET_LIP));
                    geo.push(length("Cepten kalan pay", OpeningTypeField::Lip, b.lip, lip));
                    let words: Vec<String> = sizes
                        .pockets
                        .iter()
                        .map(|p| format!("{} {}", f.length(p.length), if p.at_end { "sonda" } else { "başta" }))
                        .collect();
                    if !words.is_empty() {
                        geo.push(Row::text("Cep", words.join(", ")));
                    }
                }
                (OpeningForm::Door, OpeningOperation::Folding) => {
                    geo.push(switch("İki yana toplanır", OpeningTypeField::BothSides, b.both_sides == Some(true)));
                }
                (OpeningForm::Door | OpeningForm::Window, OpeningOperation::Slide) => {
                    if x.leaves == 2 {
                        let lap = format!("{} · %5", f.length(sizes.leaf * 0.05));
                        geo.push(length("Kanat bindirmesi", OpeningTypeField::Overlap, b.overlap, lap));
                        // The first leaf's share (docs/adr/0807 §2): empty halves.
                        let editor = number(Field::OpeningBuild(slot, OpeningTypeField::Split));
                        let shown = b.split.map_or_else(
                            || "Eşit".to_owned(),
                            |v| format!("%{}", (v * 1000.0).round() / 10.0),
                        );
                        geo.push(Row::figure("Birinci kanadın payı", shown).editor(editor));
                        if !sizes.leaves.is_empty() {
                            let words: Vec<String> = sizes.leaves.iter().map(|l| f.length(*l)).collect();
                            geo.push(Row::text("Kanatlar", words.join(" + ")));
                        }
                    }
                    // Three and four leaves (docs/adr/0792 §1): the tracks, both jambs with four, the overlap
                    // at each meeting and the layout they make.
                    if x.leaves > 2 {
                        let tracks = b.tracks.unwrap_or(2);
                        let picks = [2u8, 3]
                            .into_iter()
                            .map(|n| {
                                let nb = OpeningBuild { tracks: Some(n), ..Default::default() };
                                let change = OpeningChange { build: Some(Box::new(nb)), ..Default::default() };
                                pick(n.to_string(), n == tracks, set(change), None)
                            })
                            .collect();
                        geo.push(Row::figure("Ray sayısı", tracks.to_string()).editor(select(tracks.to_string(), picks)));
                        if x.leaves == 4 {
                            geo.push(switch("İki yana açılır", OpeningTypeField::BothSides, b.both_sides == Some(true)));
                        } else {
                            // Three leaves: the outer two slide over a fixed middle (XOX, docs/adr/0807 §1).
                            geo.push(switch("Ortası sabit (XOX)", OpeningTypeField::BothSides, b.both_sides == Some(true)));
                        }
                        let share = sizes.leaf * kentos_commands::SLIDE_LAP / f64::from(x.leaves - 1);
                        let lap = format!("{} · öntanımlı", f.length(share));
                        geo.push(length("Kanat bindirmesi", OpeningTypeField::Overlap, b.overlap, lap));
                        if let Some(panels) = &sizes.panels {
                            geo.push(Row::text("Kanat düzeni", panels.clone()));
                        }
                    }
                    geo.push(switch("Ray çizilir", OpeningTypeField::Track, b.track == Some(true)));
                }
                _ => {}
            }
        }
    }
    if x.form == OpeningForm::Door && x.operation == OpeningOperation::Swing {
        geo.push(
            Row::figure("Açı", format!("{}", x.swing_angle()))
                .unit("°")
                .editor(number(Field::OpeningSwing(slot))),
        );
        let arcs = [(true, "Evet"), (false, "Hayır")]
            .into_iter()
            .map(|(v, l)| pick(l.to_owned(), v == x.show_swing, set(OpeningChange { show_swing: Some(v), ..Default::default() }), None))
            .collect();
        let arc = if x.show_swing { "Evet" } else { "Hayır" };
        geo.push(Row::text("Açılım yayı", arc).editor(select(arc.to_owned(), arcs)));
    }
    let optional = |label: &'static str, v: Option<f64>, field: Field| match v {
        Some(v) => metres(label, v).editor(number(field)),
        None => Row::figure(label, "Yok").editor(number(field)),
    };
    if matches!(x.form, OpeningForm::Door | OpeningForm::Window) {
        geo.push(optional("Kasa", x.frame, Field::OpeningFrame(slot)));
    }
    match x.form {
        OpeningForm::Window => geo.push(optional("Denizlik", x.sill, Field::OpeningSill(slot))),
        OpeningForm::Opening | OpeningForm::Niche | OpeningForm::Flue => {
            geo.push(optional("Alt kot", x.sill, Field::OpeningSill(slot)));
        }
        OpeningForm::Door => {}
    }
    if x.form.is_recess() {
        geo.push(optional("Üst kot", x.head, Field::OpeningHead(slot)));
    } else {
        geo.push(optional("Lento", x.head, Field::OpeningHead(slot)));
    }
    // Whether plan cuts it, at the project's cut height (docs/adr/0719 §2, 3012).
    if x.form.cut_by_height() {
        let cut = doc.settings().plan_cut();
        let words = if x.cut_in_plan(cut) {
            format!("Kesilir ({} kesme)", f.length(cut))
        } else {
            format!("Kesik çizgi ({} kesmenin dışında)", f.length(cut))
        };
        geo.push(Row::text("Planda", words));
    }
    geo.push(
        Row::text("Numara", x.number.clone().unwrap_or_default())
            .editor(edit(Editor::Text(Field::OpeningNumber(slot)))),
    );
    let wall = kentos_domain::Uuid::parse_str(&x.host)
        .ok()
        .and_then(|u| doc.model.slot_of(u))
        .and_then(|s| match doc.model.get(s) {
            Some(Entity::Wall(w)) => Some(w.base.label.clone().unwrap_or_else(|| format!("Duvar {}", s.0))),
            _ => None,
        })
        .unwrap_or_else(|| "Duvar yok".to_owned());
    geo.push(Row::text("Duvar", wall));
    if let Some(b) = x.broken {
        geo.push(Row::text("Durum", b.words()));
    }
    geo
}
