//! What a table, a statistic or a join view offers when its source could not be read (docs/adr/0740 §2, plan/gis.md
//! CBS-40): the command's words and the one way out `sources::trouble::Trouble` says. Yeniden dene asks the same
//! question again (from its cursor: no row twice, none skipped); the layer's Yeniden bağla… (Adresi denetle for a
//! database) links it to its file's new place, after which what was not read is asked again; a question that reads too
//! much says to narrow it. The same words and the same button wherever a source is read.

use iced::Element;
use iced::widget::button;
use kentos_commands::{CommandResult, SourceLink, SourceNext};
use kentos_native_application::sources::trouble::Trouble;
use kentos_ui::{label, style};

use crate::app::Message;

/// Asks a question the source refused again, as it was.
pub(crate) const RETRY: &str = "Yeniden dene";
/// Asks a question that was stopped again, from where it stood.
pub(crate) const GO_ON: &str = "Sürdür";

/// A question a source refused: the command's code and words.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Refused {
    pub code: String,
    pub why: String,
}

impl Refused {
    /// What helps it.
    pub(crate) fn way(&self) -> Option<Trouble> {
        Trouble::of(&self.code)
    }

    /// A command's answer as an output or a refusal; a stopped question is the `stopped` words (no code).
    pub(crate) fn of<T: std::fmt::Debug>(
        answer: CommandResult<T>,
        stopped: &str,
    ) -> Result<T, Refused> {
        match answer {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err(Refused {
                code: error.code,
                why: error.message,
            }),
            CommandResult::Cancelled => Err(Refused {
                code: "cancelled".into(),
                why: stopped.to_owned(),
            }),
            CommandResult::Queued { .. } => Err(Refused {
                code: String::new(),
                why: "Kaynağın cevabı bir iş olarak geldi; burada beklenmez.".into(),
            }),
        }
    }
}

/// The one button that helps `refused`: none when only another question does (a filter that reads too much, a field
/// that is not there), or when the source is not a bound layer's and must be linked elsewhere (a join view's: its
/// window). `retry` asks the same question again.
pub(crate) fn way_out<'a>(
    refused: &Refused,
    link: Option<&SourceLink>,
    retry: Message,
) -> Option<Element<'a, Message>> {
    let (words, message) = match refused.way()? {
        Trouble::Retry | Trouble::Changed => (RETRY, retry),
        Trouble::Relink => {
            let link = link?;
            (
                crate::source_status::act_words(SourceNext::Relink, link.kind)?,
                Message::Bound(crate::bound::Event::Relink(Some(link.layer.clone()))),
            )
        }
        Trouble::Narrow => return None,
    };
    Some(
        button(label::caption(words))
            .on_press(message)
            .padding([2, 8])
            .style(style::button::secondary)
            .into(),
    )
}

impl crate::app::App {
    /// A bound layer linked to its file's new place (Yeniden bağla…): what its source did not give is asked again,
    /// Kaynağın tümü's page from its cursor and İstatistik's question.
    pub(crate) fn source_back(&mut self, layer: &str) -> iced::Task<Message> {
        let mut tasks = Vec::new();
        if self.features.whole.layer.as_deref() == Some(layer)
            && self.features.whole.missed.is_some()
        {
            tasks.push(self.whole_retry());
        }
        let refused = self
            .charts
            .whole
            .answer
            .as_ref()
            .is_some_and(Result::is_err);
        if refused && self.stats_bound().is_some_and(|l| l.layer == layer) {
            tasks.push(self.stats_whole_retry());
        }
        iced::Task::batch(tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::{CommandError, SourceKind};

    fn link(kind: SourceKind) -> SourceLink {
        SourceLink {
            id: "kaynak-1".into(),
            kind,
            path: "/veri/sira.gpkg".into(),
            table: "olay".into(),
            layer: "import-olay".into(),
            relative: None,
            revision: String::new(),
            digest: String::new(),
            srid: Some(5256),
            extent: None,
            key: None,
            parked: Vec::new(),
            checks: Vec::new(),
            uniques: Vec::new(),
            writable: true,
        }
    }

    fn refused(code: &str) -> Refused {
        Refused {
            code: code.into(),
            why: "neden".into(),
        }
    }

    #[test]
    fn a_refusal_offers_the_way_its_code_says() {
        let gpkg = link(SourceKind::Geopackage);
        let retry = Message::Run("test.retry");
        for code in ["source_unreachable", "import_unreadable", "source_changed"] {
            assert!(
                way_out(&refused(code), Some(&gpkg), retry.clone()).is_some(),
                "{code}"
            );
        }
        for code in ["file_not_found", "table_not_found"] {
            assert!(
                way_out(&refused(code), Some(&gpkg), retry.clone()).is_some(),
                "{code}"
            );
            // A join view's source is linked again in its own window.
            assert!(
                way_out(&refused(code), None, retry.clone()).is_none(),
                "{code}"
            );
        }
        for code in ["order_too_large", "field_not_found", "cancelled"] {
            assert!(
                way_out(&refused(code), Some(&gpkg), retry.clone()).is_none(),
                "{code}"
            );
        }
    }

    #[test]
    fn an_answer_reads_as_its_output_or_its_refusal() {
        let error = |code: &str| CommandError {
            code: code.into(),
            message: "neden".into(),
            ..CommandError::default()
        };
        assert_eq!(
            Refused::of(
                CommandResult::Completed {
                    output: 7,
                    warnings: Vec::new()
                },
                "durdu"
            ),
            Ok(7)
        );
        assert_eq!(
            Refused::of::<i32>(
                CommandResult::Failed {
                    error: error("file_not_found")
                },
                "durdu"
            )
            .map_err(|r| r.way()),
            Err(Some(Trouble::Relink))
        );
        assert_eq!(
            Refused::of::<i32>(
                CommandResult::Conflict {
                    error: error("source_changed")
                },
                "durdu"
            )
            .map_err(|r| r.way()),
            Err(Some(Trouble::Changed))
        );
        assert_eq!(
            Refused::of::<i32>(CommandResult::Cancelled, "durdu"),
            Err(Refused {
                code: "cancelled".into(),
                why: "durdu".into()
            })
        );
    }
}
