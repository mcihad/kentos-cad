//! The BIM permit package in Paftaları toplu yayınla, opened from Ruhsat takımı (docs/adr/8075; plan/bim-plan.md
//! BIM-14): a switch adds the building's IFC model and the model quality control form to the publication
//! (`PublishSheets::bim`). After Denetle the window shows the five results apart and, for each question KentOS leaves to
//! the author (EK-9 Tablo 9.1: the file size, the development level, the area tables), Evet or Hayır and a note: the
//! author's answers go with the publication. What the model or the form leaves is among the check's errors, and the
//! publication does not go out while one is left (the command's own rule).

use iced::widget::{button, column, row};
use iced::{Element, Fill};
use kentos_commands::{BimPackage, BimPackaged, FormAnswer};
use kentos_ui::{label, style, widget::Switch};

use super::sheet_publish::{Event, State};
use crate::app::{App, Message};

/// The switch's words.
pub(crate) const BIM_LABEL: &str = "BIM modeli ve model kalite kontrol formu (Md. 9/1, 15)";

fn event(e: Event) -> Message {
    super::message(super::Event::SheetPublish(e))
}

/// The package switched on or off: on, the publication is PDF/A in separate files, as the package asks.
pub(crate) fn switch(s: &mut State, on: bool) {
    s.input.bim = on.then(BimPackage::default);
    if on {
        s.input.pdfa = true;
        s.input.mode = kentos_commands::SheetPublishMode::Separate;
    }
}

/// The author's answer to question `no`: Evet, Hayır, or none (taken back).
pub(crate) fn answer(s: &mut State, no: u8, yes: Option<bool>) {
    // The evidence answered: the question's KentOS note in the check's report (docs/adr/8077 §7).
    let basis = s
        .report
        .as_ref()
        .and_then(|r| r.bim.as_ref())
        .and_then(|b| b.rows.iter().find(|r| r.no == no))
        .map(|r| r.basis.clone())
        .unwrap_or_default();
    let Some(bim) = s.input.bim.as_mut() else { return };
    let note = bim.answers.iter().find(|a| a.no == no).map(|a| a.note.clone()).unwrap_or_default();
    bim.answers.retain(|a| a.no != no);
    if let Some(yes) = yes {
        bim.answers.push(FormAnswer { no, yes, note, basis });
        bim.answers.sort_by_key(|a| a.no);
    }
}

/// The author's note on question `no` (kept with an answer only).
pub(crate) fn note(s: &mut State, no: u8, text: String) {
    if let Some(a) = s.input.bim.as_mut().and_then(|b| b.answers.iter_mut().find(|a| a.no == no)) {
        a.note = text;
    }
}

impl App {
    /// Writes the window's answers to the drawing (`cad.form.answer`, docs/adr/8077) where they differ from it: the
    /// questions given and the ones taken back. Whether the check or the publication may go on (a refusal is said).
    pub(crate) fn permit_bim_keep(&mut self) -> bool {
        let (Some(crate::exchange::Window::SheetPublish(s)), Some(doc)) = (&self.exchange, self.document.as_mut()) else {
            return true;
        };
        let Some(bim) = s.input.bim.as_ref() else {
            return true;
        };
        let held = doc.model.form_answers().to_vec();
        let mut changes: Vec<serde_json::Value> = bim
            .answers
            .iter()
            .filter(|a| !held.contains(a))
            .map(|a| serde_json::json!({"no": a.no, "yes": a.yes, "note": a.note, "basis": a.basis}))
            .collect();
        changes.extend(
            held.iter()
                .filter(|h| !bim.answers.iter().any(|a| a.no == h.no))
                .map(|h| serde_json::json!({"no": h.no, "yes": null})),
        );
        if changes.is_empty() {
            return true;
        }
        let answer = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            kentos_commands::CAD_FORM_ANSWER,
            kentos_commands::CallOp::Execute,
            serde_json::json!({"answers": changes}),
        );
        match answer {
            Ok(v) if v["status"] == "completed" => true,
            Ok(v) => {
                let words = v["error"]["message"].as_str().unwrap_or("komut reddetti").to_owned();
                self.say(kentos_interaction::Level::Error, format!("Kontrol formunun cevapları yazılamadı: {words}"));
                false
            }
            Err(unrun) => {
                self.say(kentos_interaction::Level::Error, format!("Kontrol formunun cevapları yazılamadı: {}", unrun.message));
                false
            }
        }
    }

    /// The package's part of the window: the switch, then after Denetle the five results and the author's questions.
    pub(crate) fn permit_bim_view<'a>(&'a self, s: &'a State, busy: bool) -> Element<'a, Message> {
        let on = s.input.bim.is_some();
        let mut out = column![if busy {
            Element::from(Switch::<Message>::disabled(on).label(BIM_LABEL))
        } else {
            Element::from(Switch::new(on, |v| event(Event::Bim(v))).label(BIM_LABEL))
        }]
        .spacing(6);
        if !on {
            return out.into();
        }
        let Some(bim) = s.report.as_ref().and_then(|r| r.bim.as_ref()) else {
            return out
                .push(label::caption(
                    "Denetle binanın IFC modelini yazıp geri okur, kontrol formunu cevaplar; incelemeye kalan soruları burada \
                     cevaplarsınız.",
                ))
                .into();
        };
        out = out.push(results(bim));
        let open: Vec<_> = bim.rows.iter().filter(|r| r.evidence == "inceleme").collect();
        if !open.is_empty() {
            out = out.push(label::body("Müellifin cevabı (EK-9)").font(kentos_ui::theme::typography::ui_strong()));
        }
        for r in open {
            let given = s.input.bim.as_ref().and_then(|b| b.answers.iter().find(|a| a.no == r.no));
            let choice = |yes: bool, words: &'static str| {
                let chosen = given.is_some_and(|a| a.yes == yes);
                // The chosen answer's words on the accent, as a primary button writes them.
                let words = label::caption(words).style(if chosen { style::text::on_accent } else { style::text::default });
                button(words)
                    .padding(kentos_ui::theme::metrics::action())
                    .style(if chosen { style::button::primary } else { style::button::secondary })
                    .on_press_maybe((!busy).then(|| event(Event::BimAnswer(r.no, (!chosen).then_some(yes)))))
            };
            let mut note = kentos_ui::widget::text_field("Not", given.map_or("", |a| a.note.as_str()));
            if !busy && given.is_some() {
                let no = r.no;
                note = note.on_input(move |v| event(Event::BimNote(no, v)));
            }
            out = out.push(
                column![
                    label::caption(format!("{}. {} — {}", r.no, r.title, r.note)),
                    row![choice(true, "Evet"), choice(false, "Hayır"), note.width(Fill)]
                        .spacing(6)
                        .align_y(iced::Alignment::Center),
                ]
                .spacing(4),
            );
        }
        out.into()
    }
}

/// The five results, each on its line: holds, does not, or not KentOS's to say.
fn results<'a>(bim: &'a BimPackaged) -> Element<'a, Message> {
    let name = |key: &str| match key {
        "okunuyor" => "Dosya okunuyor",
        "sema" => "Şema",
        "profil" => "Türkiye profili (EK-5, 6, 7, 9)",
        "model_pafta" => "Model ile pafta uyumu (Md. 9/1)",
        "idare" => "İdare kabulü",
        _ => "",
    };
    let mut out = column![label::body(format!("BIM paketi · {}", bim.model)).font(kentos_ui::theme::typography::ui_strong())]
        .spacing(2);
    for r in &bim.results {
        let (mark, text): (&str, fn(&iced::Theme) -> iced::widget::text::Style) = match r.holds {
            Some(true) => ("Tutuyor", style::text::default),
            Some(false) => ("Tutmuyor", style::text::danger),
            None => ("Bekleniyor", style::text::muted),
        };
        out = out.push(label::caption(format!("{} · {mark} · {}", name(&r.key), r.words)).style(text));
    }
    out.into()
}
