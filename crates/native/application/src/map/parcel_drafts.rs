//! `cad.parcels.approve` v1 and `cad.parcels.withdraw` v1 (plan/map-cad.md MAP-08, docs/adr/5020): a parcel document's
//! draft and approval. `cad.parcels.record` with `draft` keeps the document as a draft: what it would make (its areas'
//! identity, stated area, rings and stated lengths as they were when drafted) and close, nothing current changed.
//! Approving writes the approval given outside (its number, date, who; KentOS never approves): a draft is then written
//! as a document with the next number, as drafted, and leaves the drafts; a recorded document keeps the approval beside
//! it. Withdrawing removes a draft. The shared cases are `fixtures/commands/v1/cad.parcels.approve.json` and
//! `cad.parcels.withdraw.json`.

use kentos_commands::{
    CommandResult, DraftParcel, DraftRing, MAX_APPROVAL_REFERENCE, MAX_PARCEL_LABEL, MAX_PARCEL_NAME, ParcelApproval,
    ParcelApprovalInput, ParcelDocument, ParcelDraft, ParcelRegister, ParcelsApprove, ParcelsApproved, ParcelsWithdraw,
    ParcelsWithdrawn, RecordedParcel, StatedLength, review_label_ok,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::parcel_register::{AreaRing, Made, apply, fail, free_of, next_document};

/// The undo steps' names.
pub const APPROVE_STEP: &str = "Belge onayı";
pub const WITHDRAW_STEP: &str = "Taslağı geri çek";

/// The draft `cad.parcels.record` keeps for a document: numbered after the drafts, its parcels as read now.
pub(crate) fn draft_of(reg: &ParcelRegister, document: &ParcelDocument, made: &[Made], closing: &[u32]) -> ParcelDraft {
    ParcelDraft {
        number: reg.drafts.iter().map(|d| d.number).max().unwrap_or(0) + 1,
        name: document.name.clone(),
        kind: document.kind.clone(),
        date: document.date.clone(),
        note: document.note.clone(),
        create: made
            .iter()
            .map(|m| DraftParcel {
                uid: m.parcel.uid.clone().unwrap_or_default(),
                il: m.parcel.il.clone(),
                ilce: m.parcel.ilce.clone(),
                mahalle: m.parcel.mahalle.clone(),
                ada: m.parcel.ada.clone(),
                parsel: m.parcel.parsel.clone(),
                stated_area: m.parcel.stated_area.clone(),
                rings: m
                    .rings
                    .iter()
                    .map(|(part, hole, pts, bulges)| DraftRing {
                        part: *part,
                        hole: *hole,
                        pts: pts.clone(),
                        bulges: if bulges.iter().all(|b| *b == 0.0) { Vec::new() } else { bulges.clone() },
                    })
                    .collect(),
                lengths: m
                    .lengths
                    .iter()
                    .map(|(ring, edge, length, _)| StatedLength { ring: *ring as u32, edge: *edge as u32, length: length.clone() })
                    .collect(),
            })
            .collect(),
        close: closing.to_vec(),
    }
}

/// A draft's parcel as the document will make it.
fn made_of_draft(p: &DraftParcel) -> Made {
    let rings: Vec<AreaRing> = p
        .rings
        .iter()
        .map(|r| {
            let bulges = if r.bulges.is_empty() { vec![0.0; r.pts.len()] } else { r.bulges.clone() };
            (r.part, r.hole, r.pts.clone(), bulges)
        })
        .collect();
    Made {
        parcel: RecordedParcel {
            number: 0,
            il: p.il.clone(),
            ilce: p.ilce.clone(),
            mahalle: p.mahalle.clone(),
            ada: p.ada.clone(),
            parsel: p.parsel.clone(),
            created_by: 0,
            closed_by: None,
            stated_area: p.stated_area.clone(),
            uid: Some(p.uid.clone()),
            rings: Vec::new(),
        },
        rings,
        lengths: p
            .lengths
            .iter()
            .map(|l| (l.ring as usize, l.edge as usize, l.length.clone(), "draft".to_owned()))
            .collect(),
    }
}

/// The approval as given, trimmed and checked.
fn approval_of(a: &ParcelApprovalInput) -> Result<ParcelApproval, Stop> {
    let reference = a.reference.trim().to_owned();
    if !review_label_ok(&reference, MAX_APPROVAL_REFERENCE) {
        return Err(fail(
            codes::INVALID_INPUT,
            format!(
                "Onay sayısı boş olmaz; en çok {MAX_APPROVAL_REFERENCE} karakterdir, denetim karakteri taşımaz. Onayı \
                 veren yerin yazdığı sayıyı yazın."
            ),
            "approval.reference",
        ));
    }
    let text = |t: &Option<String>| t.as_deref().map(str::trim).unwrap_or_default().to_owned();
    let (date, by) = (text(&a.date), text(&a.by));
    if !date.is_empty() && !review_label_ok(&date, MAX_PARCEL_LABEL) {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Onay tarihi en çok {MAX_PARCEL_LABEL} karakterdir, denetim karakteri taşımaz."),
            "approval.date",
        ));
    }
    if !by.is_empty() && !review_label_ok(&by, MAX_PARCEL_NAME) {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Onaylayan en çok {MAX_PARCEL_NAME} karakterdir, denetim karakteri taşımaz."),
            "approval.by",
        ));
    }
    Ok(ParcelApproval { reference, date, by })
}

/// The register after the approval and the answer, or why not.
fn approve(doc: &Document, input: &ParcelsApprove) -> Result<(ParcelRegister, ParcelsApproved), Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    if input.draft.is_some() == input.document.is_some() {
        return Err(fail(
            codes::INVALID_INPUT,
            "Onayı yazılacak taslağın (draft) ya da belgenin (document) numarasını verin; ikisinden yalnız biri.".into(),
            "draft",
        ));
    }
    let approval = approval_of(&input.approval)?;
    let mut reg = doc.parcel_register().cloned().unwrap_or_default();
    if let Some(n) = input.document {
        let Some(d) = reg.documents.iter_mut().find(|d| d.number == n) else {
            return Err(fail(codes::UNKNOWN_PARCEL_DOCUMENT, format!("Parsel kayıtlarında {n} numaralı belge yok."), "document"));
        };
        if let Some(a) = &d.approval {
            return Err(fail(
                codes::DOCUMENT_APPROVED,
                format!(
                    "“{}” belgesinin onayı yazılmış ({}); onay bir kez yazılır, değişirse yeni bir belgeyle kaydedin.",
                    d.name, a.reference
                ),
                "document",
            ));
        }
        d.approval = Some(approval);
        return Ok((reg, ParcelsApproved { document: n, recorded: None, revision: doc.revision().to_string() }));
    }
    let n = input.draft.unwrap_or_default();
    let Some(at) = reg.drafts.iter().position(|d| d.number == n) else {
        return Err(fail(codes::UNKNOWN_PARCEL_DRAFT, format!("Parsel kayıtlarında {n} numaralı belge taslağı yok."), "draft"));
    };
    let draft = reg.drafts.remove(at);
    // What the draft closes must still be current: a document recorded since may have closed it.
    for c in &draft.close {
        if let Some(p) = reg.parcel(*c)
            && let Some(by) = p.closed_by
        {
            let by = reg.document(by).map(|x| x.name.clone()).unwrap_or_default();
            return Err(fail(
                codes::PARCEL_CLOSED,
                format!(
                    "Taslağın kapatacağı {c} numaralı parsel ({}) o arada “{by}” belgesiyle kapatılmış; taslak bugün \
                     yazılamaz. Taslağı geri çekip bugünkü kayıtlara göre yeniden hazırlayın.",
                    p.label()
                ),
                "draft",
            ));
        }
    }
    let mut made: Vec<Made> = Vec::new();
    for p in &draft.create {
        let m = made_of_draft(p);
        free_of(&reg, &draft.close, &made, &m, "draft")?;
        made.push(m);
    }
    let document = ParcelDocument {
        number: next_document(&reg),
        name: draft.name,
        kind: draft.kind,
        date: draft.date,
        note: draft.note,
        approval: Some(approval),
    };
    let mut out = apply(&mut reg, document, made, draft.close, "draft")?;
    out.revision = doc.revision().to_string();
    Ok((reg, ParcelsApproved { document: out.document, recorded: Some(out), revision: doc.revision().to_string() }))
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ParcelsApprove) -> CommandResult<()> {
    match approve(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ParcelsApprove) -> CommandResult<ParcelsApproved> {
    match approve(cx.doc, input) {
        Ok((_, out)) => CommandResult::Completed { output: out, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

fn write(cx: &mut ExecutionContext<'_>, input: ParcelsApprove) -> CommandResult<ParcelsApproved> {
    let (reg, mut out) = match approve(cx.doc, &input) {
        Ok(done) => done,
        Err(stop) => return stop.into(),
    };
    cx.doc.set_parcel_register(Some(reg), APPROVE_STEP);
    out.revision = cx.doc.revision().to_string();
    if let Some(r) = out.recorded.as_mut() {
        r.revision = out.revision.clone();
    }
    CommandResult::Completed { output: out, warnings: Vec::new() }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ParcelsApprove) -> CommandResult<ParcelsApproved> {
    crate::call::recorded(cx, kentos_commands::CAD_PARCELS_APPROVE, kentos_commands::CAD_PARCELS_APPROVE_VERSION, input, write)
}

pub mod withdraw {
    //! `cad.parcels.withdraw` v1's handlers: the draft removed, nothing else.
    use super::*;

    fn run(doc: &Document, input: &ParcelsWithdraw) -> Result<(ParcelRegister, ParcelsWithdrawn), Stop> {
        checks::revision(doc, input.expected_revision.as_deref())?;
        let n = input.draft;
        let mut reg = doc.parcel_register().cloned().unwrap_or_default();
        let Some(at) = reg.drafts.iter().position(|d| d.number == n) else {
            return Err(fail(codes::UNKNOWN_PARCEL_DRAFT, format!("Parsel kayıtlarında {n} numaralı belge taslağı yok."), "draft"));
        };
        let name = reg.drafts.remove(at).name;
        Ok((reg, ParcelsWithdrawn { name, revision: doc.revision().to_string() }))
    }

    pub fn validate(cx: &ExecutionContext<'_>, input: &ParcelsWithdraw) -> CommandResult<()> {
        match run(cx.doc, input) {
            Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &ParcelsWithdraw) -> CommandResult<ParcelsWithdrawn> {
        match run(cx.doc, input) {
            Ok((_, out)) => CommandResult::Completed { output: out, warnings: Vec::new() },
            Err(stop) => stop.into(),
        }
    }

    fn write(cx: &mut ExecutionContext<'_>, input: ParcelsWithdraw) -> CommandResult<ParcelsWithdrawn> {
        let (reg, mut out) = match run(cx.doc, &input) {
            Ok(done) => done,
            Err(stop) => return stop.into(),
        };
        // A register left with nothing is none.
        cx.doc.set_parcel_register((!reg.is_empty()).then_some(reg), WITHDRAW_STEP);
        out.revision = cx.doc.revision().to_string();
        CommandResult::Completed { output: out, warnings: Vec::new() }
    }

    pub fn execute(cx: &mut ExecutionContext<'_>, input: ParcelsWithdraw) -> CommandResult<ParcelsWithdrawn> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_PARCELS_WITHDRAW,
            kentos_commands::CAD_PARCELS_WITHDRAW_VERSION,
            input,
            write,
        )
    }
}
