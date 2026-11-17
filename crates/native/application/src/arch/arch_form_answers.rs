//! `cad.form.answer` v1 (“Kontrol formunun cevapları”, docs/adr/8077; plan/bim-plan.md BIM-14): the author's answers
//! to the model quality control form's questions KentOS leaves to them, kept in the drawing (`settings.arch.formAnswers`)
//! as one undo step. The printed form (the standards package's `form.tablo`) and the BIM package's publication read
//! them; the shared cases are `fixtures/commands/v1/cad.form.answer.json` (`scripts/fixtures/form_answers_cases.py`).
//!
//! The checks, in order (the first that fails answers): a change at all (`nothing_to_set`); each change's number in
//! 1–21, no question twice, no note with null, a note a form's cell holds (`invalid_input`); the expected revision.

use kentos_commands::{ArchFormAnswers, ArchFormAnswersDone, CommandResult, FormAnswer};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const STEP: &str = "Kontrol formunun cevapları";

fn fail(code: &str, message: String, path: String) -> Stop {
    Stop::failed(error(code, message, Some(path)))
}

/// The drawing's answers now.
pub fn answers_of(doc: &Document) -> Vec<FormAnswer> {
    doc.form_answers().to_vec()
}

/// The answers the changes leave, by question; refused with the first change that does not hold.
fn check(doc: &Document, input: &ArchFormAnswers) -> Result<Vec<FormAnswer>, Stop> {
    if input.answers.is_empty() {
        return Err(fail(
            codes::NOTHING_TO_SET,
            "Değiştirilecek bir cevap verilmedi. Sorunun numarasını ve cevabını (Evet: true, Hayır: false, kaldırmak için null) verin.".into(),
            "answers".into(),
        ));
    }
    let mut next = answers_of(doc);
    for (i, c) in input.answers.iter().enumerate() {
        if input.answers[..i].iter().any(|d| d.no == c.no) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("{}. soru iki kez verildi; her soruyu bir kez verin.", c.no),
                format!("answers[{i}].no"),
            ));
        }
        match c.yes {
            None => {
                if c.note.is_some() {
                    return Err(fail(
                        codes::INVALID_INPUT,
                        format!("{}. sorunun cevabı kaldırılırken not verilemez; notu bir cevapla verin.", c.no),
                        format!("answers[{i}].note"),
                    ));
                }
                if !(1..=21).contains(&c.no) {
                    return Err(fail(
                        codes::INVALID_INPUT,
                        format!("Sorunun numarası 1 ile 21 arasında olmalı (EK-9 Tablo 9.1); {} verildi.", c.no),
                        format!("answers[{i}].no"),
                    ));
                }
                next.retain(|a| a.no != c.no);
            }
            Some(yes) => {
                let note = match &c.note {
                    Some(n) => n.trim().to_owned(),
                    None => next.iter().find(|a| a.no == c.no).map(|a| a.note.clone()).unwrap_or_default(),
                };
                let answer = FormAnswer { no: c.no, yes, note };
                if let Some((field, why)) = answer.problem() {
                    return Err(fail(codes::INVALID_INPUT, why, format!("answers[{i}].{field}")));
                }
                next.retain(|a| a.no != c.no);
                next.push(answer);
            }
        }
    }
    next.sort_by_key(|a| a.no);
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(next)
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ArchFormAnswers) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What it would write, nothing written.
pub fn plan(cx: &ExecutionContext<'_>, input: &ArchFormAnswers) -> CommandResult<ArchFormAnswersDone> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document.
pub fn plan_in(doc: &Document, input: &ArchFormAnswers) -> CommandResult<ArchFormAnswersDone> {
    match check(doc, input) {
        Ok(answers) => CommandResult::Completed {
            output: ArchFormAnswersDone { answers, revision: doc.revision().to_string() },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ArchFormAnswers) -> CommandResult<ArchFormAnswersDone> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_FORM_ANSWER,
        kentos_commands::CAD_FORM_ANSWER_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: ArchFormAnswers) -> CommandResult<ArchFormAnswersDone> {
    let answers = match check(cx.doc, &input) {
        Ok(next) => next,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    if let Err(e) = doc.set_form_answers(answers.clone(), STEP) {
        return CommandResult::Failed { error: error(codes::INVALID_INPUT, e.0, Some("answers".into())) };
    }
    CommandResult::Completed {
        output: ArchFormAnswersDone { answers, revision: doc.revision().to_string() },
        warnings: Vec::new(),
    }
}
