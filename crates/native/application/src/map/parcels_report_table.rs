//! `cad.parcels.table` v1 (docs/adr/9306): Parsel alan çizelgesi's words written as one table at `at`, one undo step
//! “Parsel alan çizelgesi tablosu”. The words, the merged ranges and the bold rows are the report's
//! (parcels_report.rs); this part makes them a table: its alignments and styles, its sizes fitted to its words (a merged
//! range's words widen its last column), its layer and height, and writes it.

use super::*;

/// Widens the last column of each merged range whose words are wider than its columns (the layout's padding each
/// side), so the title, the headings and the area rows' words fit.
fn fit_merged(doc: &Document, t: &mut TableEntity) {
    let font = drawing_font(doc.settings().drawing_font);
    let entity = Entity::Table(t.clone());
    let s = shape(&entity);
    let Some(g) = table_geom(&s) else { return };
    let typeface = g.face.font_or(font);
    let mut columns = t.columns.clone();
    for r in &t.merges {
        let (row, col, cols) = (r.row as usize, r.col as usize, r.cols as usize);
        let Some(words) = t.cells.get(row).and_then(|w| w.get(col)) else {
            continue;
        };
        let bold = (t.header && row == 0)
            || t.styles
                .iter()
                .any(|st| st.range.holds(row, col) && st.bold == Some(true));
        let need = width_em_in(words, typeface, bold) * t.height + 2.0 * PAD * t.height;
        let have: f64 = columns.iter().skip(col).take(cols).sum();
        if need > have
            && let Some(last) = columns.get_mut(col + cols - 1)
        {
            *last += need - have;
        }
    }
    t.columns = columns;
}

fn checked(
    doc: &Document,
    input: &ParcelsTable,
) -> Result<(TableEntity, Worked, Vec<CommandWarning>), Stop> {
    checks::point(input.at, "Tablonun köşesinin", "at")?;
    if let Some(h) = input.height
        && !(h.is_finite() && h > 0.0 && h <= MAX_TABLE_SIZE)
    {
        return Err(fail(
            codes::INVALID_TABLE,
            format!("Tablonun yazı yüksekliği {h}; sıfırdan büyük ve sonlu olmalı."),
            "height",
        ));
    }
    let report = input.report();
    let parcels = check(doc, &report)?;
    if parcels.parcels.is_empty() {
        return Err(fail(
            codes::NO_OBJECTS,
            "Çizelgeye yazılacak parsel yok: parsel katmanında alan yok ya da hiçbir alan verilmedi. Parselleri seçin ya da parsel katmanına alan çizin.".into(),
            "uids",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let layer = input
        .layer_id
        .clone()
        .unwrap_or_else(|| doc.layers().active().to_owned());
    let warnings = checks::layer(doc, &layer)?;
    let worked = work(doc, &report, parcels);
    let height = input.height.unwrap_or_else(|| {
        doc.settings()
            .annotation_height(kentos_commands::AnnotationKind::Table)
    });
    let m = worked.columns;
    let mut aligns = vec![TableAlign::Right; m];
    aligns[0] = TableAlign::Left;
    let mut t = TableEntity {
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: layer,
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        p: input.at,
        rotation: 0.0,
        height,
        rows: Vec::new(),
        columns: Vec::new(),
        cells: Vec::new(),
        merges: worked.merges.clone(),
        aligns: Some(aligns),
        header: true,
        grid: None,
        frame: None,
        face: Default::default(),
        source: None,
        styles: worked
            .bold
            .iter()
            .map(|&row| CellStyle {
                range: CellRange {
                    row,
                    col: 0,
                    rows: 1,
                    cols: count(m),
                },
                align: (row == 1).then_some(TableAlign::Center),
                bold: Some(true),
                color: None,
                fill: None,
                decimals: None,
            })
            .collect(),
        formulas: Vec::new(),
        fields: Vec::new(),
        breaks: None,
        table_style: None,
    };
    doc.take_rows(&mut t, worked.report.cells.clone());
    fit_merged(doc, &mut t);
    if let Some((field, words)) = t.problem() {
        return Err(fail(codes::INVALID_TABLE, words, field));
    }
    Ok((t, worked, warnings))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ParcelsTable) -> CommandResult<()> {
    match checked(cx.doc, input) {
        Ok((_, _, warnings)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &ParcelsTable) -> CommandResult<ParcelsTablePlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over a document: the table the placement hangs from the cursor.
pub fn plan_in(doc: &Document, input: &ParcelsTable) -> CommandResult<ParcelsTablePlan> {
    match checked(doc, input) {
        Ok((t, worked, warnings)) => CommandResult::Completed {
            output: ParcelsTablePlan {
                parcels: count(worked.report.parcels.len()),
                rows: count(t.cells.len()),
                numbered: worked.report.numbered,
                table: Entity::Table(t),
                revision: doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ParcelsTable) -> CommandResult<ParcelsTabled> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_PARCELS_TABLE,
        kentos_commands::CAD_PARCELS_TABLE_VERSION,
        input,
        write,
    )
}

fn write(cx: &mut ExecutionContext<'_>, input: ParcelsTable) -> CommandResult<ParcelsTabled> {
    let (t, worked, warnings) = match checked(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let rows = count(t.cells.len());
    let slot = match cx.doc.add_many(vec![Entity::Table(t)], STEP) {
        Ok(slots) => slots.first().copied(),
        Err(full) => {
            return Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None)).into();
        }
    };
    let Some(slot) = slot else {
        return Stop::failed(error(codes::SLOTS_EXHAUSTED, "Yuva yok.".into(), None)).into();
    };
    CommandResult::Completed {
        output: ParcelsTabled {
            uid: cx.doc.uid(slot).map(|u| u.to_string()).unwrap_or_default(),
            parcels: count(worked.report.parcels.len()),
            rows,
            numbered: worked.report.numbered,
            revision: cx.doc.revision().to_string(),
        },
        warnings,
    }
}
