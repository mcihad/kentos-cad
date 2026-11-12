//! A row's edit written: a typed value or a choice turned into the command that writes it (casework,
//! dimension sets, elevations and every other field).

use super::*;

/// A kitchen run changed through `cad.casework.edit` (docs/adr/0727 §6): its refusal or warnings, said.
pub(super) fn casework_edit(
    model: &mut kentos_domain::Document,
    slot: Slot,
    edit: serde_json::Value,
) -> Vec<String> {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let Some(uid) = model.uid(slot) else {
        return Vec::new();
    };
    let input = serde_json::json!({
        "run": uid.to_string(),
        "edit": edit,
        "expectedRevision": model.revision().to_string(),
    });
    said(registry::run(
        &mut ExecutionContext::new(model),
        "cad.casework.edit",
        CallOp::Execute,
        input,
    ))
}

/// A command's answer as what the panel says: its warnings, or why it wrote nothing.
pub(super) fn said(
    answer: Result<serde_json::Value, kentos_native_application::registry::Unrun>,
) -> Vec<String> {
    match answer {
        Ok(value) if value["status"] == "completed" => value["warnings"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|w| w["message"].as_str().map(str::to_owned))
            .collect(),
        Ok(value) => vec![
            value["error"]["message"]
                .as_str()
                .unwrap_or("Mutfak dizisi değiştirilemedi; katman kilitlerini kontrol edin.")
                .into(),
        ],
        Err(error) => vec![error.message],
    }
}

/// Diziyi bırak and Dizinin tamamını sil.
pub(super) fn casework_action(
    model: &mut kentos_domain::Document,
    slot: Slot,
    action: &str,
) -> Vec<String> {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    match action {
        "release" => casework_edit(model, slot, serde_json::json!({"kind": "release"})),
        "delete" => {
            let Some((run, _)) = model.uid(slot).and_then(|uid| model.casework_member(uid)) else {
                return Vec::new();
            };
            let uids: Vec<String> = run.members.iter().map(|m| m.uid.to_text()).collect();
            let input =
                serde_json::json!({"uids": uids, "expectedRevision": model.revision().to_string()});
            said(registry::run(
                &mut ExecutionContext::new(model),
                "cad.entities.delete",
                CallOp::Execute,
                input,
            ))
        }
        _ => Vec::new(),
    }
}

/// A run's value picked: the object's module's width, door or kind (its other own value kept), or the recipe's.
pub(super) fn casework_value(
    model: &mut kentos_domain::Document,
    slot: Slot,
    field: &str,
    value: &str,
) -> Vec<String> {
    use serde_json::json;
    let Some((run, member)) = model.uid(slot).and_then(|uid| model.casework_member(uid)) else {
        return Vec::new();
    };
    let (recipe, module) = (run.recipe.clone(), member.index);
    let (own_width, own_hand) = match module {
        Some(k) => (
            rows::casework::own_width(&recipe, k),
            rows::casework::own_hand(&recipe, k),
        ),
        None => (None, None),
    };
    let edit = match (field, module) {
        ("width", Some(k)) => {
            let Ok(width) = value.parse::<f64>() else {
                return Vec::new();
            };
            // A cabinet wider than 0.60 m takes a double door: its hinge goes.
            let hand = own_hand.filter(|_| width <= kentos_commands::CASEWORK_APPLIANCE + 1e-9);
            json!({"kind": "module", "module": k, "width": width, "hand": hand})
        }
        ("hand", Some(k)) => {
            json!({"kind": "module", "module": k, "hand": value, "width": own_width})
        }
        ("holds", Some(k)) => {
            // An appliance (a dishwasher, an oven's cooktop) has no width or door of its own.
            let appliance =
                value == "dishwasher" || (value == "cooktop" && recipe.oven == Some(true));
            if appliance {
                json!({"kind": "module", "module": k, "holds": value})
            } else {
                json!({"kind": "module", "module": k, "holds": value, "width": own_width, "hand": own_hand})
            }
        }
        ("corner" | "fridge", _) => {
            let mut r = serde_json::to_value(&recipe).unwrap_or_default();
            r[field] = if value == "none" {
                serde_json::Value::Null
            } else {
                json!(value)
            };
            json!({"kind": "recipe", "recipe": r})
        }
        ("upper" | "oven", _) => {
            let mut r = serde_json::to_value(&recipe).unwrap_or_default();
            r[field] = json!(value == "true");
            json!({"kind": "recipe", "recipe": r})
        }
        _ => return Vec::new(),
    };
    casework_edit(model, slot, edit)
}

/// A member points at its recipe; the shared command owns all edits and undo.
pub(super) fn dimension_set_edit(
    model: &mut kentos_domain::Document,
    slot: Slot,
    operation: serde_json::Value,
) -> Vec<String> {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let Some((set, _)) = model
        .uid(slot)
        .and_then(|uid| model.dimension_set_of_item(uid))
    else {
        return Vec::new();
    };
    let input = serde_json::json!({"id":set.id.to_string(),"operation":operation,"expectedRevision":model.revision().to_string()});
    match registry::run(
        &mut ExecutionContext::new(model),
        "cad.dimensionsets.edit",
        CallOp::Execute,
        input,
    ) {
        Ok(value) if value["status"] == "completed" => value["warnings"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|w| w["message"].as_str().map(str::to_owned))
            .collect(),
        Ok(value) => vec![
            value["error"]["message"]
                .as_str()
                .unwrap_or(
                    "Ölçü dizisi değiştirilemedi; kaynağı ve katman kilitlerini kontrol edin.",
                )
                .into(),
        ],
        Err(error) => vec![error.message],
    }
}

pub(super) fn dimension_set_action(
    model: &mut kentos_domain::Document,
    slot: Slot,
    action: &str,
) -> Vec<String> {
    use serde_json::json;
    let found = model
        .uid(slot)
        .and_then(|uid| model.dimension_set_of_item(uid));
    let row = found.map(|(set, index)| set.members[index].row.name());
    // A façade's or a space's row on or off, the rows kept in their order (docs/adr/0729 §7).
    let toggled = action.strip_prefix("toggle:").and_then(|name| {
        let (set, _) = found?;
        let order = ["openings", "walls", "grids", "overall"];
        let mut rows: Vec<&str> = order
            .iter()
            .copied()
            .filter(|r| set.rows.iter().any(|x| x.name() == *r))
            .collect();
        if rows.contains(&name) {
            rows.retain(|r| *r != name);
        } else {
            rows.push(name);
            rows.sort_by_key(|r| order.iter().position(|o| o == r));
        }
        Some(rows)
    });
    let operation = match action {
        _ if toggled.is_some() => json!({"kind":"set","rows":toggled}),
        "faces" | "core" => json!({"kind":"set","reference":action}),
        "left" | "right" => json!({"kind":"set","face":action}),
        "ends" | "centers" => json!({"kind":"set","openings":action}),
        "both" => json!({"kind":"set","rows":["openings","overall"]}),
        "openings" | "overall" => json!({"kind":"set","rows":[action]}),
        "rowText" => json!({"kind":"resetText","row":row}),
        "resetRows" | "restoreHidden" | "resetText" | "detach" | "delete" => json!({"kind":action}),
        _ => return Vec::new(),
    };
    dimension_set_edit(model, slot, operation)
}

/// `cad.entities.set`'s input for the objects in `slots`, nothing set yet.
pub(super) fn set_input(
    model: &kentos_domain::Document,
    slots: &[Slot],
    operation: PropertiesOperation,
) -> EntitiesSetProperties {
    EntitiesSetProperties {
        line_pattern: None,
        line_type: None,
        line_type_scale: None,
        transparency: None,
        wipeout: None,
        uids: properties::uids_of(model, slots),
        layer_id: None,
        color: None,
        line_weight: None,
        symbol: None,
        attrs: None,
        label: None,
        operation,
        expected_revision: None,
        unlink: false,
        attachments: None,
    }
}

/// A Kot cell's text into the drawing (docs/adr/0142): a number sets the
/// elevation of the vertices it stands for, an empty text clears it, and
/// anything else changes nothing (the web's `parseElevation`). What to say:
/// the command's refusal.
pub(super) fn commit_elevation(
    model: &mut kentos_domain::Document,
    slots: &[Slot],
    spot: Spot,
    text: &str,
) -> Vec<String> {
    // Typed in the project's unit (docs/adr/0165 §2).
    let f = kentos_interaction::Format::of(model.settings());
    let Some(value) = elevation::parse_typed_in(text, f.length_unit_label()) else {
        return Vec::new();
    };
    let value = value.map(|z| f.to_metres(z));
    let change = match spot {
        Spot::Start => Change::End(0, value),
        Spot::End => Change::End(1, value),
        Spot::All => Change::Set(value),
    };
    properties::set_elevations(model, slots, change)
}

/// A cell's text into the drawing, as the web's editors take it; what they
/// do not take changes nothing. What to say: the command's refusal.
pub(super) fn commit(
    model: &mut kentos_domain::Document,
    field: &Field,
    text: &str,
) -> Vec<String> {
    // Lengths and coordinates are typed in the project's unit (docs/adr/0165 §2).
    let f = kentos_interaction::Format::of(model.settings());
    // Only physical architectural length cells opt in. Angles, paper sizes,
    // coordinates and the legacy GIS readers keep their existing grammar.
    let cad = model.settings().workspace == Some(kentos_commands::Workspace::Cad);
    let length = |text: &str| {
        if cad {
            kentos_geometry_core::tools::point_text::parse_length(text)
                .map(|value| value.metres(|v| f.to_metres(v)))
        } else {
            Some(f.to_metres(web_number(text)))
        }
    };
    if cad && let Field::TextHeight(slot) = field {
        let Some(height) = length(text).filter(|h| h.is_finite() && *h > 0.0) else {
            return vec!["Yazı yüksekliği sıfırdan büyük bir uzunluk olmalı (örnek: 2.5 mm).".into()];
        };
        return properties::set_text_heights(model, &[*slot], height);
    }
    if let Field::TextHeights(slots) = field {
        let Some(height) = length(text).filter(|h| h.is_finite() && *h > 0.0) else {
            return vec!["Yazı yüksekliği sıfırdan büyük bir uzunluk olmalı (örnek: 2.5 mm).".into()];
        };
        return properties::set_text_heights(model, slots, height);
    }
    if let Field::CaseworkDepth(slot) = field {
        let Some(recipe) = model
            .uid(*slot)
            .and_then(|uid| model.casework_member(uid))
            .map(|(run, _)| run.recipe.clone())
        else {
            return Vec::new();
        };
        return match length(text) {
            Some(depth) if depth.is_finite() => {
                let mut r = serde_json::to_value(&recipe).unwrap_or_default();
                r["depth"] = serde_json::json!(depth);
                casework_edit(
                    model,
                    *slot,
                    serde_json::json!({"kind": "recipe", "recipe": r}),
                )
            }
            _ => vec!["Tezgâhın derinliğini girin; örneğin 65 cm.".into()],
        };
    }
    if let Field::DimensionSetScales(slot) = field {
        return match kentos_interaction::wall_dimensions::sheet_scales(text) {
            Some(scales) => dimension_set_edit(model, *slot, serde_json::json!({"kind":"set","scales":scales})),
            None => vec!["Pafta ölçeklerini 1:100, 1:200 ya da 100 200 gibi yazın (1 ile 1000000 arası, en çok 32); kaldırmak için boş bırakın ya da Yok yazın.".into()],
        };
    }
    if let Field::DimensionSetGap(slot) = field {
        let typed = text.trim().strip_suffix("mm").unwrap_or(text.trim()).trim();
        return match kentos_geometry_core::tools::point_text::parse_number(typed) {
            Some(mm) if mm.is_finite() && mm > 0.0 && mm <= 1000.0 =>
                dimension_set_edit(model, *slot, serde_json::json!({"kind":"set","gapMm":mm})),
            _ => vec!["Kağıt üzerinde 0'dan büyük, en çok 1000 mm bir sıra aralığı girin; örneğin 7.5 mm.".into()],
        };
    }
    if let Field::DimensionOffset(slot) = field
        && let Some((set, index)) = model
            .uid(*slot)
            .and_then(|uid| model.dimension_set_of_item(uid))
    {
        let row = set.members[index].row.name();
        return match length(text) {
            Some(offset) if offset.is_finite() => dimension_set_edit(
                model,
                *slot,
                serde_json::json!({"kind":"rowOffset","row":row,"offset":offset}),
            ),
            _ => vec!["Bu sıranın duvar yüzüne uzaklığını girin; örneğin 40 cm.".into()],
        };
    }
    match field {
        Field::Elevation(slot, spot) => return commit_elevation(model, &[*slot], *spot, text),
        // Typed in the project's unit; emptied or 0, none (docs/adr/0258 §2).
        Field::PolylineWidth(slot) => {
            let t = kentos_interaction::js_trim(text);
            let n = if t.is_empty() { 0.0 } else { web_number(t) };
            return if n.is_finite() {
                properties::set_polyline_width(model, &[*slot], f.to_metres(n))
            } else {
                Vec::new()
            };
        }
        Field::Elevations(slots) => return commit_elevation(model, slots, Spot::All, text),
        // Any finite number: the command refuses one out of range and says why.
        Field::TextWidth(slots) => {
            let n = web_number(text);
            return if n.is_finite() {
                properties::set_text_width(model, slots, n)
            } else {
                Vec::new()
            };
        }
        // A box: emptied, none; a number, the command refuses one not over 0 (docs/adr/0182 §4).
        Field::AnnotativeScales(slots) => {
            return match kentos_interaction::annotative::parse_scales(text) {
                Ok(wanted) => kentos_interaction::annotative::set_scales(model, slots, &wanted),
                Err(words) => vec![words],
            };
        }
        Field::TextBox(slots) => {
            let t = kentos_interaction::js_trim(text);
            if t.is_empty() || t == "Kutusuz" {
                return properties::set_text_box(model, slots, None);
            }
            let n = web_number(t);
            return if n.is_finite() {
                properties::set_text_box(model, slots, Some(n))
            } else {
                Vec::new()
            };
        }
        Field::TextSpacing(slots) => {
            let n = web_number(text);
            return if n.is_finite() {
                properties::set_text_spacing(model, slots, n)
            } else {
                Vec::new()
            };
        }
        // The layout's numbers (docs/adr/0324 §3): the command refuses one out of range and says why.
        Field::TextIndent(slots)
        | Field::TextLeftIndent(slots)
        | Field::TextColumns(slots)
        | Field::TextColumnHeight(slots)
        | Field::TextColumnGap(slots) => {
            use kentos_interaction::text_layout as layout;
            let n = web_number(text);
            if !n.is_finite() {
                return Vec::new();
            }
            return match field {
                Field::TextIndent(_) => layout::set_indent(model, slots, n),
                Field::TextLeftIndent(_) => layout::set_left_indent(model, slots, n),
                Field::TextColumns(_) => {
                    layout::set_columns(model, slots, n.max(0.0).min(f64::from(u32::MAX)) as u32)
                }
                Field::TextColumnHeight(_) => layout::set_column_height(model, slots, n),
                _ => layout::set_column_gap(model, slots, n),
            };
        }
        Field::TextTabs(slots) => {
            return match kentos_interaction::text_layout::parse_tabs(text) {
                Ok(stops) => kentos_interaction::text_layout::set_tabs(model, slots, &stops),
                Err(words) => vec![words],
            };
        }
        Field::LeaderNote(slots) => {
            let note = kentos_interaction::js_trim(text);
            let to = (!note.is_empty()).then(|| note.to_owned());
            return properties::change_leaders(model, slots, |l| {
                (l.text != to).then(|| LeaderEntity {
                    text: to.clone(),
                    ..l.clone()
                })
            });
        }
        Field::LeaderHeight(slots) => {
            let n = f.to_metres(web_number(text));
            if !(n.is_finite() && n > 0.0) {
                return Vec::new();
            }
            return properties::change_leaders(model, slots, |l| {
                (l.height != n).then(|| LeaderEntity {
                    height: n,
                    ..l.clone()
                })
            });
        }
        Field::DimensionRound(slots) | Field::DimensionTolerance(slots, _) => {
            let n = f.to_metres(web_number(text));
            let empty = kentos_interaction::js_trim(text).is_empty();
            if !(empty || n.is_finite()) {
                return Vec::new();
            }
            let base = model
                .get(slots[0])
                .and_then(|e| match e {
                    Entity::Dimension(d) => d.look.options.as_deref().cloned(),
                    _ => None,
                })
                .unwrap_or_default();
            let mut options = base;
            match field {
                Field::DimensionRound(_) => options.round = (!empty && n > 0.0).then_some(n),
                Field::DimensionTolerance(_, upper) => {
                    let Some(t) = options.tolerance.as_mut() else {
                        return Vec::new();
                    };
                    let v = if empty { 0.0 } else { n };
                    if *upper {
                        t.upper = v;
                    } else {
                        t.lower = Some(v);
                    }
                }
                _ => {}
            }
            let options = (!options.is_empty()).then_some(options);
            return properties::edit_dimensions(model, slots, |uids| {
                kentos_commands::DimensionEdit::Options { uids, options }
            });
        }
        // Fark (docs/adr/0721 §2): the Kots' offset from the level they share; emptied or 0, none.
        Field::DimensionLevelOffset(slots) => {
            let id = slots.iter().find_map(|s| match model.get(*s) {
                Some(Entity::Dimension(d)) => d.level.as_ref().map(|l| l.id.clone()),
                _ => None,
            });
            let Some(id) = id else {
                return Vec::new();
            };
            // A length with its unit kept (−2 cm), else in the drawing's unit (docs/adr/0646).
            let typed = kentos_interaction::js_trim(text);
            let n = if typed.is_empty() {
                Some(0.0)
            } else {
                length(typed)
            };
            let Some(n) =
                n.filter(|n| n.is_finite() && n.abs() <= kentos_commands::MAX_LEVEL_OFFSET)
            else {
                return vec![
                    "Farkı metre olarak bir sayıyla yazın (−0.02 gibi); en çok 1000 m.".into(),
                ];
            };
            let level = kentos_commands::DimensionLevel {
                id,
                offset: (n != 0.0).then_some(n),
            };
            return properties::edit_dimensions(model, slots, |uids| {
                kentos_commands::DimensionEdit::Level {
                    uids,
                    level: Some(level),
                }
            });
        }
        // A slope's and a Kot's elevations: a length with its unit kept in CAD (docs/adr/0646, 0721 §2).
        Field::DimensionZa(slots) | Field::DimensionZb(slots) => {
            let Some(n) = length(kentos_interaction::js_trim(text)).filter(|n| n.is_finite())
            else {
                return Vec::new();
            };
            let second = matches!(field, Field::DimensionZb(_));
            return properties::change_dimensions(model, slots, |d| {
                let z = if second { d.zb } else { d.za };
                (z != Some(n)).then(|| {
                    let mut d = d.clone();
                    *(if second { &mut d.zb } else { &mut d.za }) = Some(n);
                    d
                })
            });
        }
        Field::LeaderArrowSize(slots) => {
            let n = web_number(text);
            if !kentos_commands::leader_arrow_holds(n) {
                return Vec::new();
            }
            let size = (n != 1.0).then_some(n);
            return properties::change_leaders(model, slots, |l| {
                (l.arrow_size.unwrap_or(1.0) != n).then(|| LeaderEntity {
                    arrow_size: size,
                    ..l.clone()
                })
            });
        }
        Field::OpeningBuild(slot, which) => {
            use kentos_commands::{OpeningChange, OpeningsOperation};
            let change = if text.trim().is_empty() {
                OpeningChange {
                    unset: vec![*which],
                    ..Default::default()
                }
            } else {
                // A two-leaf slide's share (docs/adr/0807 §2): a fraction or a percentage, not a length.
                let n = if *which == kentos_commands::OpeningTypeField::Split {
                    let t = text.trim().trim_start_matches('%').replace(',', ".");
                    match t.parse::<f64>() {
                        Ok(v) if v > 1.0 || text.trim().starts_with('%') => Some(v / 100.0),
                        Ok(v) => Some(v),
                        Err(_) => None,
                    }
                } else {
                    length(text)
                };
                let Some(n) = n else {
                    return vec![if *which == kentos_commands::OpeningTypeField::Split {
                        "Bir pay girin; örneğin %60 veya 0.6.".into()
                    } else {
                        "Bir uzunluk girin; örneğin 10 cm veya 0.10 m.".into()
                    }];
                };
                if !n.is_finite() {
                    return Vec::new();
                }
                let Ok(b) = serde_json::from_value::<kentos_commands::OpeningBuild>(
                    serde_json::json!({ which.name(): n }),
                ) else {
                    return Vec::new();
                };
                OpeningChange {
                    build: Some(Box::new(b)),
                    ..Default::default()
                }
            };
            return properties::edit_opening(model, *slot, OpeningsOperation::Set(change));
        }
        Field::OpeningWidth(slot)
        | Field::OpeningAt(slot)
        | Field::OpeningSwing(slot)
        | Field::OpeningFrame(slot)
        | Field::OpeningSill(slot)
        | Field::OpeningHead(slot)
        | Field::OpeningDepth(slot)
        | Field::OpeningInset(slot)
        | Field::OpeningNumber(slot) => {
            use kentos_commands::{OpeningChange, OpeningsOperation};
            let blank = text.trim().is_empty();
            let op = match field {
                Field::OpeningNumber(_) => {
                    let n = kentos_interaction::js_trim(text);
                    OpeningsOperation::Set(OpeningChange {
                        number: Some((!n.is_empty()).then(|| n.to_owned())),
                        ..Default::default()
                    })
                }
                Field::OpeningFrame(_)
                | Field::OpeningSill(_)
                | Field::OpeningHead(_)
                | Field::OpeningSwing(_)
                | Field::OpeningInset(_)
                    if blank =>
                {
                    OpeningsOperation::Set(match field {
                        Field::OpeningFrame(_) => OpeningChange {
                            frame: Some(None),
                            ..Default::default()
                        },
                        Field::OpeningSill(_) => OpeningChange {
                            sill: Some(None),
                            ..Default::default()
                        },
                        Field::OpeningHead(_) => OpeningChange {
                            head: Some(None),
                            ..Default::default()
                        },
                        Field::OpeningInset(_) => OpeningChange {
                            inset: Some(None),
                            ..Default::default()
                        },
                        _ => OpeningChange {
                            swing: Some(None),
                            ..Default::default()
                        },
                    })
                }
                _ => {
                    let n = if matches!(field, Field::OpeningSwing(_)) {
                        web_number(text)
                    } else {
                        let Some(n) = length(text) else {
                            return vec![
                                "Açıklık ölçüsü için bir uzunluk girin; örneğin 900 mm veya 90 cm."
                                    .into(),
                            ];
                        };
                        n
                    };
                    if !n.is_finite() {
                        return Vec::new();
                    }
                    match field {
                        Field::OpeningWidth(_) => OpeningsOperation::Set(OpeningChange {
                            width: Some(n),
                            ..Default::default()
                        }),
                        Field::OpeningAt(_) => OpeningsOperation::Move {
                            by: None,
                            at: Some(n),
                        },
                        Field::OpeningSwing(_) => OpeningsOperation::Set(OpeningChange {
                            swing: Some(Some(n)),
                            ..Default::default()
                        }),
                        Field::OpeningFrame(_) => OpeningsOperation::Set(OpeningChange {
                            frame: Some(Some(n)),
                            ..Default::default()
                        }),
                        Field::OpeningSill(_) => OpeningsOperation::Set(OpeningChange {
                            sill: Some(Some(n)),
                            ..Default::default()
                        }),
                        Field::OpeningDepth(_) => OpeningsOperation::Set(OpeningChange {
                            depth: Some(Some(n)),
                            ..Default::default()
                        }),
                        Field::OpeningInset(_) => OpeningsOperation::Set(OpeningChange {
                            inset: Some(Some(n)),
                            ..Default::default()
                        }),
                        _ => OpeningsOperation::Set(OpeningChange {
                            head: Some(Some(n)),
                            ..Default::default()
                        }),
                    }
                }
            };
            return properties::edit_opening(model, *slot, op);
        }
        Field::Arch(slot, which @ (ArchField::MarkNumber | ArchField::MarkSize)) => {
            use kentos_commands::{ViewMarkChange, ViewMarksOperation};
            let t = kentos_interaction::js_trim(text);
            let c = if *which == ArchField::MarkNumber {
                ViewMarkChange {
                    mark: Some(t.to_owned()),
                    ..Default::default()
                }
            } else {
                let n = web_number(text);
                if !t.is_empty() && !n.is_finite() {
                    return Vec::new();
                }
                ViewMarkChange {
                    size: Some((!t.is_empty()).then_some(n)),
                    ..Default::default()
                }
            };
            return properties::edit_viewmark(model, *slot, ViewMarksOperation::Set(Box::new(c)));
        }
        Field::Arch(slot, which @ (ArchField::TagContent | ArchField::TagSize)) => {
            use kentos_commands::TagChange;
            let t = kentos_interaction::js_trim(text);
            let c = if *which == ArchField::TagContent {
                TagChange {
                    content: Some((!t.is_empty()).then(|| t.replace("\\n", "\n"))),
                    ..Default::default()
                }
            } else {
                let n = web_number(text);
                if !t.is_empty() && !n.is_finite() {
                    return Vec::new();
                }
                TagChange {
                    size: Some((!t.is_empty()).then_some(n)),
                    ..Default::default()
                }
            };
            return properties::edit_tag(model, *slot, c);
        }
        Field::Arch(slot, which) => {
            use kentos_commands::{ArchChange, ArchOperation};
            let t = kentos_interaction::js_trim(text);
            let words = (!t.is_empty()).then(|| t.to_owned());
            let physical = matches!(
                which,
                ArchField::Width
                    | ArchField::Depth
                    | ArchField::RiserHeight
                    | ArchField::Tread
                    | ArchField::Landing
                    | ArchField::Rise
                    | ArchField::Base
                    | ArchField::BaseOffset
                    | ArchField::Post
                    | ArchField::LegA
                    | ArchField::LegB
                    | ArchField::BeamHeight
                    | ArchField::BeamOffset
                    | ArchField::BeamBase
                    | ArchField::RailingGap
            );
            let n = if physical && !t.is_empty() {
                let Some(n) = length(text) else {
                    return vec![
                        "Mimari ölçü için bir uzunluk girin; örneğin 400 mm veya 40 cm.".into(),
                    ];
                };
                n
            } else {
                web_number(text)
            };
            if !t.is_empty()
                && !n.is_finite()
                && !matches!(
                    which,
                    ArchField::Label | ArchField::Tag | ArchField::Type | ArchField::Material
                )
            {
                return Vec::new();
            }
            let c = match which {
                ArchField::Label => match words {
                    Some(w) => ArchChange {
                        label: Some(w),
                        ..Default::default()
                    },
                    None => return Vec::new(),
                },
                ArchField::Tag => ArchChange {
                    tag: Some(words),
                    ..Default::default()
                },
                ArchField::Type => ArchChange {
                    kind: Some(words),
                    ..Default::default()
                },
                ArchField::Material => ArchChange {
                    material: Some(words),
                    ..Default::default()
                },
                ArchField::BeamOffset => ArchChange {
                    offset: Some(words.map_or(0.0, |_| n)),
                    ..Default::default()
                },
                ArchField::BeamBase => ArchChange {
                    base_offset: Some(words.map(|_| n)),
                    ..Default::default()
                },
                ArchField::RailingGap => ArchChange {
                    gap: Some(words.map(|_| n)),
                    ..Default::default()
                },
                ArchField::BubbleSize => ArchChange {
                    bubble_size: Some(words.map(|_| n)),
                    ..Default::default()
                },
                ArchField::Depth => ArchChange {
                    depth: Some(words.map(|_| n)),
                    ..Default::default()
                },
                ArchField::Landing => ArchChange {
                    landing: Some(words.map(|_| n)),
                    ..Default::default()
                },
                ArchField::Slope => ArchChange {
                    slope: Some(words.map(|_| n)),
                    ..Default::default()
                },
                ArchField::Base => ArchChange {
                    base: Some(words.map(|_| n)),
                    ..Default::default()
                },
                ArchField::BaseOffset => ArchChange {
                    base_offset: Some(words.map(|_| n)),
                    ..Default::default()
                },
                ArchField::Post => ArchChange {
                    post: Some(words.map(|_| n)),
                    ..Default::default()
                },
                ArchField::Winders if words.is_none() => ArchChange {
                    winders: Some(None),
                    ..Default::default()
                },
                ArchField::Winders if n >= 1.0 && n.fract() == 0.0 => ArchChange {
                    winders: Some(Some(n as u32)),
                    ..Default::default()
                },
                ArchField::Winders => {
                    return vec![
                        "Balansman basamağı 1 ya da daha büyük bir tam sayıdır; boş: sahanlık."
                            .into(),
                    ];
                }
                ArchField::Ends => ArchChange {
                    ends: Some(words.map(|_| n).filter(|v| *v != 0.0)),
                    ..Default::default()
                },
                _ if words.is_none() => return Vec::new(),
                // One leg typed, the other kept (docs/adr/0725 §3).
                ArchField::LegA | ArchField::LegB => {
                    let Some(kentos_commands::Entity::Column(c)) = model.get(*slot) else {
                        return Vec::new();
                    };
                    let [a, b] = c
                        .column
                        .legs
                        .unwrap_or([c.column.depth() / 2.0, c.column.width / 2.0]);
                    ArchChange {
                        legs: Some(if *which == ArchField::LegA {
                            [n, b]
                        } else {
                            [a, n]
                        }),
                        ..Default::default()
                    }
                }
                ArchField::Width => ArchChange {
                    width: Some(n),
                    ..Default::default()
                },
                ArchField::Turn => ArchChange {
                    rotation: Some(n.to_radians()),
                    ..Default::default()
                },
                ArchField::Risers if n >= 0.0 && n.fract() == 0.0 => ArchChange {
                    risers: Some(n as u32),
                    ..Default::default()
                },
                ArchField::Risers => return Vec::new(),
                ArchField::RiserHeight => ArchChange {
                    riser_height: Some(n),
                    ..Default::default()
                },
                ArchField::Tread => ArchChange {
                    tread: Some(n),
                    ..Default::default()
                },
                ArchField::Rise => ArchChange {
                    rise: Some(n),
                    ..Default::default()
                },
                ArchField::BeamHeight => ArchChange {
                    height: Some(n),
                    ..Default::default()
                },
                ArchField::TagContent
                | ArchField::TagSize
                | ArchField::MarkNumber
                | ArchField::MarkSize => {
                    return Vec::new();
                }
            };
            return properties::edit_arch(model, *slot, ArchOperation::Set(Box::new(c)));
        }
        Field::SpaceNumber(slot)
        | Field::SpaceName(slot)
        | Field::SpaceUsage(slot)
        | Field::SpaceHeight(slot)
        | Field::SpaceFloorOffset(slot)
        | Field::SpaceSuspended(slot)
        | Field::SpaceFloor(slot)
        | Field::SpaceWall(slot)
        | Field::SpaceCeiling(slot)
        | Field::SpaceBase(slot)
        | Field::SpaceSill(slot)
        | Field::SpaceStair(slot)
        | Field::SpaceRailing(slot) => {
            use kentos_commands::{SpaceChange, SpacesOperation};
            let t = kentos_interaction::js_trim(text);
            let words = (!t.is_empty()).then(|| t.to_owned());
            // With an asma tavan the inside height is its height: what is typed there moves it (docs/adr/8040 §3).
            let suspended = matches!(model.get(*slot), Some(Entity::Space(s)) if s.space.suspended_ceiling.is_some());
            let change = match field {
                Field::SpaceNumber(_) => SpaceChange {
                    number: Some(words),
                    ..Default::default()
                },
                Field::SpaceName(_) => SpaceChange {
                    name: Some(words),
                    ..Default::default()
                },
                Field::SpaceUsage(_) => SpaceChange {
                    usage: Some(words),
                    ..Default::default()
                },
                Field::SpaceFloor(_) => SpaceChange {
                    floor: Some(words),
                    ..Default::default()
                },
                Field::SpaceWall(_) => SpaceChange {
                    wall: Some(words),
                    ..Default::default()
                },
                Field::SpaceCeiling(_) => SpaceChange {
                    ceiling: Some(words),
                    ..Default::default()
                },
                Field::SpaceBase(_) => SpaceChange {
                    base: Some(words),
                    ..Default::default()
                },
                Field::SpaceSill(_) => SpaceChange {
                    sill: Some(words),
                    ..Default::default()
                },
                Field::SpaceStair(_) => SpaceChange {
                    stair: Some(words),
                    ..Default::default()
                },
                Field::SpaceRailing(_) => SpaceChange {
                    railing: Some(words),
                    ..Default::default()
                },
                Field::SpaceFloorOffset(_) if t.is_empty() => SpaceChange {
                    floor_offset: Some(None),
                    ..Default::default()
                },
                Field::SpaceSuspended(_)
                    if t.is_empty() || kentos_domain::room_names::fold(t) == "YOK" =>
                {
                    SpaceChange {
                        suspended_ceiling: Some(None),
                        ..Default::default()
                    }
                }
                _ if t.is_empty() => SpaceChange {
                    height: Some(None),
                    ..Default::default()
                },
                _ => {
                    let Some(n) = length(text) else {
                        return vec![match field {
                            Field::SpaceFloorOffset(_) => {
                                "Döşeme kot farkı için bir uzunluk girin; örneğin -2 cm.".into()
                            }
                            Field::SpaceSuspended(_) => {
                                "Asma tavan için döşemeden bir yükseklik girin; örneğin 2.70 m. Kaldırmak için boş \
                                 bırakın."
                                    .into()
                            }
                            _ => "Mahal yüksekliği için bir uzunluk girin; örneğin 3000 mm veya 3 m.".into(),
                        }];
                    };
                    if !n.is_finite() {
                        return Vec::new();
                    }
                    match field {
                        Field::SpaceFloorOffset(_) => SpaceChange {
                            floor_offset: Some(Some(n)),
                            ..Default::default()
                        },
                        Field::SpaceSuspended(_) => SpaceChange {
                            suspended_ceiling: Some(Some(n)),
                            ..Default::default()
                        },
                        _ if suspended => SpaceChange {
                            suspended_ceiling: Some(Some(n)),
                            ..Default::default()
                        },
                        _ => SpaceChange {
                            height: Some(Some(n)),
                            ..Default::default()
                        },
                    }
                }
            };
            return properties::edit_space(model, *slot, SpacesOperation::Set(change));
        }
        Field::SlabThickness(slot)
        | Field::SlabOffset(slot)
        | Field::SlabEdgeWidth(slot)
        | Field::SlabLayer(slot, ..)
        | Field::SlabMaterial(slot) => {
            use kentos_commands::{SlabChange, SlabsOperation};
            let change = if let Field::SlabMaterial(_) = field {
                let t = kentos_interaction::js_trim(text);
                SlabChange {
                    material: Some((!t.is_empty()).then(|| t.to_owned())),
                    ..Default::default()
                }
            } else {
                let Some(n) = length(text) else {
                    return vec!["Bir uzunluk girin; örneğin 15 cm ya da 0.15 m.".into()];
                };
                if !n.is_finite() {
                    return Vec::new();
                }
                match field {
                    Field::SlabThickness(_) => SlabChange {
                        thickness: Some(n),
                        ..Default::default()
                    },
                    Field::SlabOffset(_) => SlabChange {
                        offset: Some(n),
                        ..Default::default()
                    },
                    // A layer's thickness (docs/adr/8040 §2); 0 takes it away.
                    Field::SlabLayer(_, over, k) => {
                        let Some(Entity::Slab(s)) = model.get(*slot) else {
                            return Vec::new();
                        };
                        let mut layers = s.slab.layers.clone();
                        let list = if *over {
                            &mut layers.over
                        } else {
                            &mut layers.under
                        };
                        if *k >= list.len() {
                            return Vec::new();
                        }
                        if n.abs() < 1e-9 {
                            list.remove(*k);
                        } else {
                            list[*k].thickness = n;
                        }
                        SlabChange {
                            layers: Some(layers),
                            ..Default::default()
                        }
                    }
                    _ => {
                        let Some(Entity::Slab(s)) = model.get(*slot) else {
                            return Vec::new();
                        };
                        let Some(e) = &s.slab.edge else {
                            return Vec::new();
                        };
                        SlabChange {
                            edge: Some(Some(kentos_commands::NewSlabEdge {
                                kind: e.kind,
                                width: Some(n),
                            })),
                            ..Default::default()
                        }
                    }
                }
            };
            return properties::edit_slab(model, *slot, SlabsOperation::Set(change));
        }
        Field::WallWidth(slot) | Field::WallOffset(slot) | Field::WallHeight(slot) => {
            let height = matches!(field, Field::WallHeight(_));
            if height && text.trim().is_empty() {
                return properties::edit_wall(
                    model,
                    *slot,
                    kentos_commands::WallChange {
                        height: Some(None),
                        ..Default::default()
                    },
                );
            }
            let Some(n) = length(text) else {
                return vec![
                    "Duvar ölçüsü için bir uzunluk girin; örneğin 250 mm veya 25 cm.".into(),
                ];
            };
            if !n.is_finite() {
                return Vec::new();
            }
            let change = match field {
                Field::WallWidth(_) => {
                    let change = kentos_commands::WallChange { width: Some(n), ..Default::default() };
                    return properties::edit_wall_kept(model, *slot, change, wall_parts::keep());
                }
                Field::WallOffset(_) => kentos_commands::WallChange {
                    offset: Some(n),
                    ..Default::default()
                },
                _ => kentos_commands::WallChange {
                    height: Some(Some(n)),
                    ..Default::default()
                },
            };
            return properties::edit_wall(model, *slot, change);
        }
        Field::LeaderTurn(slots) => {
            let n = web_number(text);
            if !n.is_finite() {
                return Vec::new();
            }
            let turn = ((n % 360.0) + 360.0) % 360.0;
            return properties::change_leaders(model, slots, |l| {
                (l.rotation != turn).then(|| LeaderEntity {
                    rotation: turn,
                    ..l.clone()
                })
            });
        }
        _ => {}
    }
    if let Field::Attributes(slots, key) = field {
        let mut input = set_input(model, slots, PropertiesOperation::Attributes);
        input.attrs = Some(BTreeMap::from([(key.clone(), Some(text.to_owned()))]));
        return properties::set_properties(model, input);
    }
    if let Field::Parameter(slot, name) = field {
        let Some(Entity::Insert(i)) = model.get(*slot) else {
            return Vec::new();
        };
        let Some(block) = model.block(i.block) else {
            return Vec::new();
        };
        let Some(p) = block
            .dynamic
            .as_ref()
            .and_then(|d| d.parameters.iter().find(|p| p.name() == name))
        else {
            return Vec::new();
        };
        let factor = kentos_native_application::blocks_insert::unit_factor(model, block);
        let Some(value) = rows::params::typed(p, text, f, factor) else {
            return vec![format!(
                "“{name}” için {} yazın.",
                match p {
                    kentos_commands::BlockParameter::Angle { .. } => {
                        format!("{} biriminde bir açı", f.angle_unit_label())
                    }
                    kentos_commands::BlockParameter::Linear { .. } => format!(
                        "{} biriminde bir uzunluk (ör. 900 mm)", f.length_unit_label()
                    ),
                    _ => format!(
                        "yerel X, Y olarak iki uzunluk ({}; ör. 900 mm, 250 mm)",
                        f.length_unit_label()
                    ),
                }
            )];
        };
        return properties::set_parameters(
            model,
            *slot,
            Some(BTreeMap::from([(name.clone(), value)])),
            None,
        );
    }
    if let Field::Attribute(slot, key) = field {
        let Some(base) = model.get(*slot).map(Entity::base) else {
            return Vec::new();
        };
        // Keep the drawn number in step with the cadastral attribute, in the same step.
        let follows = (key == "Parsel" || key == "Ada")
            && base.label.is_some()
            && base.label.as_ref() == base.attrs.get(key);
        let mut input = set_input(model, &[*slot], PropertiesOperation::Attributes);
        input.attrs = Some(BTreeMap::from([(key.clone(), Some(text.to_owned()))]));
        input.label = follows.then(|| Some(text.to_owned()));
        return properties::set_properties(model, input);
    }
    let n = web_number(text);
    let finite = n.is_finite();
    let slot = match field {
        Field::PointX(s)
        | Field::PointY(s)
        | Field::DimensionOffset(s)
        | Field::DimensionHeight(s)
        | Field::DimensionText(s)
        | Field::HatchAngle(s)
        | Field::HatchSpacing(s)
        | Field::HatchScale(s)
        | Field::Text(s)
        | Field::TextHeight(s)
        | Field::TextAngle(s)
        | Field::InsertX(s)
        | Field::InsertY(s)
        | Field::InsertScale(s)
        | Field::InsertYScale(s)
        | Field::InsertTurn(s)
        | Field::ImageX(s)
        | Field::ImageY(s)
        | Field::ImageWidth(s)
        | Field::ImageHeight(s)
        | Field::ImageTurn(s)
        | Field::ImageClear(s)
        | Field::RasterClear(s)
        | Field::CloudClear(s)
        | Field::Attribute(s, _) => *s,
        // Taken above.
        Field::WallWidth(_)
        | Field::WallOffset(_)
        | Field::WallHeight(_)
        | Field::OpeningWidth(_)
        | Field::OpeningAt(_)
        | Field::OpeningSwing(_)
        | Field::OpeningFrame(_)
        | Field::OpeningSill(_)
        | Field::OpeningHead(_)
        | Field::OpeningDepth(_)
        | Field::OpeningInset(_)
        | Field::OpeningNumber(_)
        | Field::OpeningBuild(..)
        | Field::SpaceNumber(_)
        | Field::SpaceName(_)
        | Field::SpaceUsage(_)
        | Field::SpaceHeight(_)
        | Field::SpaceFloorOffset(_)
        | Field::SpaceSuspended(_)
        | Field::SpaceFloor(_)
        | Field::SpaceWall(_)
        | Field::SpaceCeiling(_)
        | Field::SpaceBase(_)
        | Field::SpaceSill(_)
        | Field::SpaceStair(_)
        | Field::SpaceRailing(_)
        | Field::SlabThickness(_)
        | Field::SlabOffset(_)
        | Field::SlabEdgeWidth(_)
        | Field::SlabLayer(..)
        | Field::SlabMaterial(_)
        | Field::Arch(..)
        | Field::Elevation(..)
        | Field::PolylineWidth(_)
        | Field::Elevations(_)
        | Field::TextWidth(_)
        | Field::TextHeights(_)
        | Field::TextBox(_)
        | Field::AnnotativeScales(_)
        | Field::TextSpacing(_)
        | Field::TextIndent(_)
        | Field::TextLeftIndent(_)
        | Field::TextTabs(_)
        | Field::TextColumns(_)
        | Field::TextColumnHeight(_)
        | Field::TextColumnGap(_)
        | Field::LeaderNote(_)
        | Field::LeaderHeight(_)
        | Field::LeaderTurn(_)
        | Field::LeaderArrowSize(_)
        | Field::Attributes(..)
        | Field::Parameter(..)
        | Field::DimensionZa(_)
        | Field::DimensionZb(_)
        | Field::DimensionRound(_)
        | Field::DimensionTolerance(..)
        | Field::DimensionLevelOffset(_)
        // Gerekçe is the Topoloji tab's own words, taken before any drawing commit (docs/adr/4600).
        | Field::TopologyReason => return Vec::new(),
        Field::DimensionSetGap(_) | Field::DimensionSetScales(_) | Field::CaseworkDepth(_) => {
            return Vec::new();
        }
    };
    let Some(mut e) = model.get(slot).cloned() else {
        return Vec::new();
    };
    let taken = match (field, &mut e) {
        (Field::PointX(_), Entity::Point(p)) if finite => {
            p.p.x = f.to_metres(n);
            true
        }
        (Field::PointY(_), Entity::Point(p)) if finite => {
            p.p.y = f.to_metres(n);
            true
        }
        (Field::DimensionOffset(_), Entity::Dimension(d)) if finite => {
            d.offset = f.to_metres(n);
            true
        }
        (Field::DimensionHeight(_), Entity::Dimension(d)) if finite && n > 0.0 => {
            d.height = f.to_metres(n);
            true
        }
        (Field::DimensionText(_), Entity::Dimension(d)) => {
            let own = kentos_interaction::js_trim(text);
            d.text = (!own.is_empty()).then(|| own.to_owned());
            true
        }
        (Field::HatchAngle(_), Entity::Hatch(h)) if finite => {
            h.pattern.angle = n;
            true
        }
        (Field::HatchSpacing(_), Entity::Hatch(h)) if finite && n > 0.0 => {
            h.pattern.spacing = f.to_metres(n);
            true
        }
        // The pattern's paper scale: its millimetres n × N / 1000 metres each (docs/adr/0186 §7).
        (Field::HatchScale(_), Entity::Hatch(h))
            if finite && n > 0.0 && h.pattern.scale.is_some() =>
        {
            h.pattern.scale = Some(n * model.settings().plot_scale / 1000.0);
            true
        }

        // Trimmed, as the in-place editor stores it; an empty text is not taken.
        (Field::Text(_), Entity::Text(t)) => {
            let body = kentos_interaction::js_trim(text);
            if body.is_empty() {
                false
            } else {
                t.text = body.to_owned();
                true
            }
        }
        (Field::TextHeight(_), Entity::Text(t)) if finite && n > 0.0 => {
            t.height = f.to_metres(n);
            true
        }
        (Field::TextAngle(_), Entity::Text(t)) if finite => {
            t.rotation = ((n % 360.0) + 360.0) % 360.0;
            true
        }
        // A table's text height and turn (docs/adr/0184 §6): its cells keep their sizes.
        (Field::TextHeight(_), Entity::Table(t)) if finite && n > 0.0 => {
            t.height = f.to_metres(n);
            true
        }
        (Field::TextAngle(_), Entity::Table(t)) if finite => {
            t.rotation = ((n % 360.0) + 360.0) % 360.0;
            true
        }
        (Field::InsertX(_), Entity::Insert(i)) if finite => {
            i.p.x = f.to_metres(n);
            true
        }
        (Field::InsertY(_), Entity::Insert(i)) if finite => {
            i.p.y = f.to_metres(n);
            true
        }
        // Ölçek sets both factors of a uniform insert; X ölçeği the x one alone (docs/adr/0315 §3).
        (Field::InsertScale(_), Entity::Insert(i)) if finite && n > 0.0 => {
            i.scale = n;
            i.y_scale = i.y_scale.filter(|y| *y != n);
            true
        }
        (Field::InsertYScale(_), Entity::Insert(i)) if finite && n > 0.0 => {
            i.y_scale = (n != i.scale).then_some(n);
            true
        }
        (Field::InsertTurn(_), Entity::Insert(i)) if finite => {
            i.rotation = kentos_commands::blocks::turn_of(n);
            true
        }
        // A picture keeps its shape: its width sets its height and the other way round (docs/adr/0192 §4).
        (Field::ImageX(_), Entity::Image(i)) if finite => {
            i.image.p.x = f.to_metres(n);
            true
        }
        (Field::ImageY(_), Entity::Image(i)) if finite => {
            i.image.p.y = f.to_metres(n);
            true
        }
        (Field::ImageWidth(_), Entity::Image(i)) if finite && n > 0.0 => {
            let k = f.to_metres(n) / i.image.width;
            i.image.width *= k;
            i.image.height *= k;
            true
        }
        (Field::ImageHeight(_), Entity::Image(i)) if finite && n > 0.0 => {
            let k = f.to_metres(n) / i.image.height;
            i.image.width *= k;
            i.image.height *= k;
            true
        }
        (Field::ImageTurn(_), Entity::Image(i)) if finite => {
            i.image.rotation = kentos_commands::blocks::turn_of(n);
            true
        }
        (Field::ImageClear(_), Entity::Image(i)) if finite && (0.0..=90.0).contains(&n) => {
            i.image.opacity = (n > 0.0).then(|| 1.0 - n / 100.0);
            true
        }
        (Field::RasterClear(_), Entity::Raster(r)) if finite && (0.0..=90.0).contains(&n) => {
            r.raster.opacity = (n > 0.0).then(|| 1.0 - n / 100.0);
            true
        }
        (Field::CloudClear(_), Entity::PointCloud(c)) if finite && (0.0..=90.0).contains(&n) => {
            c.cloud.opacity = (n > 0.0).then(|| 1.0 - n / 100.0);
            true
        }
        _ => false,
    };
    if !taken {
        return Vec::new();
    }
    properties::set_geometry(model, slot, &e)
}
