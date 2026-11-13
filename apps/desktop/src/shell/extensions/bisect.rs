//! Sorunlu eklentiyi bul (docs/adr/8057 §4; plan/eklenti.md ADIM 18, D6): when the window misbehaves and no extension
//! is named, KentOS holds the running extensions off round by round and the user says whether the problem is still
//! there: first with none (is it an extension's at all?), then halves. The user's choices never change: a round only
//! holds extensions off for the session ([`State::Held`]), the end, or leaving, brings every one back. The rounds are
//! kept in the registry's folder, so a problem seen at start-up is found across restarts.

use iced::Element;
use iced::widget::{Column, row};
use kentos_extension_host::State;
use kentos_extension_host::watchdog::{Bisection, Finding};
use kentos_ui::label;
use kentos_ui::widget::Banner;
use kentos_ui::widget::Dialog as Frame;

use super::{Event, msg};
use crate::app::{App, Dialog, Message};
use crate::exchange::words;

/// Where the window over the manager stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BisectStep {
    /// What it does, before it starts.
    Intro,
    /// A round going on: the registry's bisection says which.
    Asking,
    /// The extension the rounds named, and in how many rounds.
    Found { id: String, rounds: u32 },
    /// The problem was there with no extension running.
    NotAnExtension,
}

impl App {
    /// The running extensions' names, in the registry's order.
    fn running_names(&self) -> Vec<String> {
        self.extensions
            .registry
            .as_ref()
            .map(|r| {
                r.extensions()
                    .into_iter()
                    .filter(|e| e.state == State::Active)
                    .map(|e| e.name)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn names_of(&self, ids: &[String]) -> String {
        ids.iter()
            .map(|id| self.extension_name(id))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Sorunlu eklentiyi bul…: the window over the manager, at the round going on or at its start.
    pub(crate) fn open_bisection(&mut self) {
        self.open_extensions();
        let going = self
            .extensions
            .registry
            .as_ref()
            .is_some_and(|r| r.bisection().is_some());
        self.extensions.bisect = Some(if going {
            BisectStep::Asking
        } else {
            BisectStep::Intro
        });
    }

    pub(crate) fn bisect_start(&mut self) {
        let result = self
            .extensions
            .registry
            .as_mut()
            .map(|r| r.start_bisection());
        match result {
            Some(Ok(_)) => self.extensions.bisect = Some(BisectStep::Asking),
            Some(Err(problem)) => {
                self.extensions.bisect = None;
                self.extensions_say(true, problem.message);
            }
            None => {}
        }
        self.extensions_changed();
    }

    pub(crate) fn bisect_answer(&mut self, problem_seen: bool) {
        let rounds = self
            .extensions
            .registry
            .as_ref()
            .and_then(|r| r.bisection().map(|b| b.rounds() + 1))
            .unwrap_or(1);
        let result = self
            .extensions
            .registry
            .as_mut()
            .map(|r| r.answer_bisection(problem_seen));
        self.extensions.bisect = match result {
            Some(Ok(Finding::Testing(_))) => Some(BisectStep::Asking),
            Some(Ok(Finding::Found(id))) => {
                self.extensions.chosen = Some(id.clone());
                Some(BisectStep::Found { id, rounds })
            }
            Some(Ok(Finding::NotAnExtension)) => Some(BisectStep::NotAnExtension),
            Some(Err(problem)) => {
                self.extensions_say(true, problem.message);
                None
            }
            None => None,
        };
        self.extensions_changed();
    }

    pub(crate) fn bisect_stop(&mut self) {
        let result = self
            .extensions
            .registry
            .as_mut()
            .map(|r| r.stop_bisection());
        self.extensions.bisect = None;
        match result {
            Some(Ok(())) => self.extensions_say(
                false,
                "Sorunlu eklentiyi bulma bırakıldı; eklentiler eskisi gibi açık.".to_owned(),
            ),
            Some(Err(problem)) => self.extensions_say(true, problem.message),
            None => {}
        }
        self.extensions_changed();
    }

    /// The window over the manager while it is open.
    pub(crate) fn extension_bisect_window(&self) -> Option<Element<'_, Message>> {
        if self.dialog != Some(Dialog::Extensions) {
            return None;
        }
        let step = self.extensions.bisect.as_ref()?;
        let window = Frame::new("Sorunlu eklentiyi bul").key("eklenti-sorun-bul");
        let close = msg(Event::BisectClose);
        let mut body = Column::new().spacing(10);
        let window = match step {
            BisectStep::Intro => {
                let running = self.running_names();
                body = body.push(label::body(
                    "Program yavaşladıysa, bir pencere bozulduysa ya da bir şey ters gidiyorsa ve hangi eklentiden \
                     olduğu belli değilse: KentOS çalışan eklentileri tur tur kapatıp açar, siz her turda sorunun \
                     sürüp sürmediğini söylersiniz.",
                ));
                body = body.push(if running.is_empty() {
                    Banner::warning("Çalışan eklenti yok: aranacak bir eklenti de yok.")
                } else {
                    Banner::info(format!(
                        "Çalışan {} eklenti: {}. En çok {} tur.",
                        running.len(),
                        running.join(", "),
                        Bisection::most_rounds(running.len())
                    ))
                });
                body = body.push(label::caption(
                    "Seçimleriniz değişmez: bir tur eklentileri yalnız bu oturum için kapatır; sonunda ya da \
                     bıraktığınızda hepsi eskisi gibi açılır. Sorun açılışta görülüyorsa programı kapatıp açın: sorun \
                     bulma kaldığı yerden sürer.",
                ));
                window
                    .push(body)
                    .action(words::secondary("Vazgeç", Some(close.clone())))
                    .action(words::primary(
                        "Başla",
                        (!running.is_empty()).then(|| msg(Event::BisectStart)),
                    ))
            }
            BisectStep::Asking => {
                let b = self
                    .extensions
                    .registry
                    .as_ref()
                    .and_then(|r| r.bisection())?;
                let held: Vec<String> = self
                    .extensions
                    .registry
                    .as_ref()
                    .map(|r| {
                        r.extensions()
                            .into_iter()
                            .filter(|e| e.state == State::Held)
                            .map(|e| e.name)
                            .collect()
                    })
                    .unwrap_or_default();
                // The round with none counts once: after it, halves of the suspects left.
                let most = Bisection::most_rounds(b.suspects().len());
                let left = if b.rounds() == 0 { most } else { most - 1 };
                body = body.push(label::strong(format!(
                    "Tur {} — en çok {left} tur kaldı",
                    b.rounds() + 1
                )));
                body = body.push(Banner::info(if b.testing().is_empty() {
                    "Bu turda hiçbir eklenti açık değil.".to_owned()
                } else {
                    format!(
                        "Bu turda yalnız şunlar açık: {}.",
                        self.names_of(b.testing())
                    )
                }));
                if !held.is_empty() {
                    body = body.push(label::caption(format!(
                        "Bu turda kapalı: {}.",
                        held.join(", ")
                    )));
                }
                body = body.push(label::body(
                    "Sorunu yeniden deneyin (açılışta görülüyorsa programı kapatıp açın). Sorun hâlâ görülüyor mu?",
                ));
                window
                    .push(body)
                    .action(words::ghost("Bırak", Some(msg(Event::BisectStop))))
                    .action(words::secondary(
                        "Hayır, geçti",
                        Some(msg(Event::BisectAnswer(false))),
                    ))
                    .action(words::primary(
                        "Evet, sürüyor",
                        Some(msg(Event::BisectAnswer(true))),
                    ))
            }
            BisectStep::Found { id, rounds } => {
                let name = self.extension_name(id);
                body = body.push(Banner::error(format!(
                    "Sorun büyük olasılıkla “{name}” eklentisinden: yalnız o açıkken görüldü ({rounds} tur)."
                )));
                body = body.push(label::caption(
                    "Eklentiler eskisi gibi açıldı. Bulunan eklentinin günlüğüne yazıldı; raporunu yayımcısına \
                     gönderebilirsiniz.",
                ));
                window
                    .push(body)
                    .action(words::ghost(
                        "Raporu kopyala",
                        Some(msg(Event::CopyReport(id.clone()))),
                    ))
                    .action(words::secondary("Açık kalsın", Some(close.clone())))
                    .action(words::primary(
                        "Devre dışı bırak",
                        Some(msg(Event::BisectDisable(id.clone()))),
                    ))
            }
            BisectStep::NotAnExtension => {
                body = body.push(Banner::info(
                    "Sorun hiçbir eklenti açık değilken de görüldü: eklentilerden değil. Eklentiler eskisi gibi açıldı.",
                ));
                window
                    .push(body)
                    .action(words::primary("Kapat", Some(close.clone())))
            }
        };
        Some(window.width(620.0).modal(close))
    }

    /// The manager's line while a bisection goes on: which round, and the way back to the question.
    pub(crate) fn bisect_line(&self) -> Option<Element<'_, Message>> {
        let b = self.extensions.registry.as_ref()?.bisection()?;
        Some(
            row![
                Banner::warning(format!(
                    "Sorunlu eklentiyi bulma sürüyor (tur {}): bazı eklentiler bu turda kapalı.",
                    b.rounds() + 1
                ))
                .action("Soruya dön", msg(Event::BisectOpen)),
            ]
            .into(),
        )
    }
}
