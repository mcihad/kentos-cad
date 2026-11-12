//! Formla düzenle's lists' sources (docs/adr/0763): each layer the form lists from asked off the interface's thread
//! (`cad.forms.sources`'s own rule, `forms_sources::probe`), each source that is not ready said in its own line with
//! its way out and Yeniden dene. The lists stay offered from the drawing's working copy meanwhile.

use iced::widget::{Column, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{FormSource, FormSourceRetry, FormsSources};
use kentos_native_application::forms_sources;

use super::{Entry, Event};
use crate::app::{App, Message};
use crate::exchange::words::{self, Kind};

const AGAIN: &str = "Yeniden dene";
const RELINK: &str = "Yeniden bağla…";
const READ: &str = "Kaynaktan oku";
const CREDENTIALS: &str = "Bağlantıyı düzenle…";
const ASKING: &str = "Listelerin kaynakları soruluyor…";

impl App {
    /// Asks the sources of the open record's lists again, off the interface's thread; an answer to an older question
    /// is dropped.
    pub(crate) fn ask_entry_sources(&mut self) -> Task<Message> {
        let (Some(e), Some(doc)) = (self.form_entry.as_mut(), self.document.as_ref()) else {
            return Task::none();
        };
        if e.preview {
            return Task::none();
        }
        let input = FormsSources {
            form: e.form.clone(),
            draft: e.draft,
            wait_ms: None,
        };
        let Ok((targets, wait)) = forms_sources::targets(&doc.model, &input) else {
            e.sources.clear();
            return Task::none();
        };
        // A form whose every list is the drawing's own asks nothing outside.
        if targets.iter().all(|t| t.link.is_none()) {
            e.sources = targets
                .iter()
                .map(|t| forms_sources::probe(t, wait))
                .collect();
            e.sources_busy = false;
            return Task::none();
        }
        e.sources_asked += 1;
        e.sources_busy = true;
        let asked = e.sources_asked;
        let (done, answer) = iced::futures::channel::oneshot::channel();
        std::thread::Builder::new()
            .name("kentos-form-kaynak".into())
            .spawn(move || {
                let _ = done.send(
                    targets
                        .iter()
                        .map(|t| forms_sources::probe(t, wait))
                        .collect::<Vec<_>>(),
                );
            })
            .ok();
        Task::perform(answer, move |a| {
            Message::FormEntry(Event::SourcesAnswered(asked, a.unwrap_or_default()))
        })
    }

    /// The sources asked on this thread, waiting `wait` for a database (the pictures: no task runs there).
    #[cfg(test)]
    pub(crate) fn entry_sources_now(&mut self, wait: std::time::Duration) {
        let (Some(e), Some(doc)) = (self.form_entry.as_mut(), self.document.as_ref()) else {
            return;
        };
        let input = FormsSources {
            form: e.form.clone(),
            draft: e.draft,
            wait_ms: None,
        };
        if let Ok((targets, _)) = forms_sources::targets(&doc.model, &input) {
            e.sources = targets
                .iter()
                .map(|t| forms_sources::probe(t, wait))
                .collect();
            e.sources_busy = false;
        }
    }

    /// A source's way out: the binding's own command (Yeniden bağla…, Kaynaktan oku) or the database's connection.
    pub(crate) fn entry_source_way(
        &mut self,
        layer: String,
        way: FormSourceRetry,
    ) -> Task<Message> {
        match way {
            FormSourceRetry::Retry => self.ask_entry_sources(),
            FormSourceRetry::Relink => {
                Task::done(Message::Bound(crate::bound::Event::Relink(Some(layer))))
            }
            FormSourceRetry::Read => {
                Task::done(Message::Bound(crate::bound::Event::Read(Some(layer))))
            }
            FormSourceRetry::Credentials => {
                let address = self.document.as_ref().and_then(|d| {
                    d.model
                        .settings()
                        .sources
                        .iter()
                        .find(|l| l.layer == layer)
                        .map(|l| l.path.clone())
                });
                let saved = address.and_then(|a| saved_connection(&a));
                use crate::db_connections::{Event as Db, Then};
                Task::done(Message::Db(match saved {
                    Some(name) => Db::Edit(name, Then::Browse),
                    None => Db::New(Then::Browse),
                }))
            }
        }
    }
}

/// The device's saved connection to the address's server, database and user (docs/adr/0623 §1).
fn saved_connection(address: &str) -> Option<String> {
    use kentos_postgis::connections::{self, Key};
    let wanted = Key::of(&connections::parse(address).ok()?.connection);
    connections::store()
        .list()
        .into_iter()
        .find(|c| Key::of(c) == wanted)
        .map(|c| c.name)
}

/// The lines of the sources that are not ready, each with its way out; the question while it runs.
pub(super) fn sources_lines<'a>(
    e: &'a Entry,
    mut body: Column<'a, Message>,
) -> Column<'a, Message> {
    if e.preview {
        return body;
    }
    if e.sources_busy {
        body = body.push(words::text_line(Kind::Info, ASKING));
    }
    for s in e.sources.iter().filter(|s| s.code.is_some()) {
        body = body.push(source_line(e, s));
    }
    body
}

fn source_line<'a>(e: &'a Entry, s: &'a FormSource) -> Element<'a, Message> {
    let mut line = row![
        container(words::text_line(
            Kind::Warn,
            s.message.clone().unwrap_or_default()
        ))
        .width(Fill)
    ]
    .spacing(6)
    .align_y(Center);
    let way = match s.retry {
        Some(FormSourceRetry::Relink) => Some(RELINK),
        Some(FormSourceRetry::Read) => Some(READ),
        Some(FormSourceRetry::Credentials) => Some(CREDENTIALS),
        Some(FormSourceRetry::Retry) | None => None,
    };
    if let (Some(caption), Some(retry)) = (way, s.retry) {
        line = line.push(words::secondary(
            caption,
            Some(e.msg(Event::SourceWay(s.layer.clone(), retry))),
        ));
    }
    line.push(words::ghost(
        AGAIN,
        (!e.sources_busy).then(|| e.msg(Event::SourcesAgain)),
    ))
    .into()
}

#[cfg(test)]
mod tests {
    use kentos_commands::FormSourceState as S;

    use crate::app::Message;
    use crate::forms::entry::Event;

    fn states(app: &crate::app::App) -> Vec<(String, S)> {
        app.form_entry
            .as_ref()
            .expect("the form")
            .sources
            .iter()
            .map(|s| (s.layer.clone(), s.state))
            .collect()
    }

    // Each source is said apart, in the form's order, the local list too (docs/adr/0763 §1, §4).
    #[test]
    fn each_source_stands_its_own_way() {
        let mut app = crate::files_testing::app_with_drawing();
        crate::form_scenes::sources_states(&mut app);
        assert_eq!(
            states(&app),
            vec![
                ("mahalle".into(), S::Local),
                ("malik".into(), S::Changed),
                ("parsel".into(), S::Missing),
                ("tapu".into(), S::Offline),
                ("ruhsat".into(), S::Timeout),
            ]
        );
        let e = app.form_entry.as_ref().expect("the form");
        assert_eq!(e.sources[4].items, vec!["ruhsat".to_owned()]);
        assert!(!e.sources_busy);
        // Yeniden dene asks again off the thread; an older question's answer is dropped.
        let _ = app.update(Message::FormEntry(Event::SourcesAgain));
        let e = app.form_entry.as_ref().expect("the form");
        assert!(e.sources_busy);
        let asked = e.sources_asked;
        let _ = app.update(Message::FormEntry(Event::SourcesAnswered(
            asked - 1,
            Vec::new(),
        )));
        assert_eq!(states(&app).len(), 5, "an old answer is dropped");
        let _ = app.update(Message::FormEntry(Event::SourcesAnswered(
            asked,
            Vec::new(),
        )));
        let e = app.form_entry.as_ref().expect("the form");
        assert!(e.sources.is_empty() && !e.sources_busy);
    }

    // A form whose lists are all the drawing's own asks nothing off the thread.
    #[test]
    fn local_lists_ask_nothing_outside() {
        let mut app = crate::files_testing::app_with_drawing();
        crate::form_scenes::owner_actions(&mut app, serde_json::Value::Null, false);
        let e = app.form_entry.as_ref().expect("the form");
        assert!(!e.sources_busy);
        assert!(e.sources.iter().all(|s| s.state == S::Local));
    }
}
