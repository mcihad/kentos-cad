//! Others' changes to an open database project (docs/adr/0041, 0040, 0044):
//! while it is open and has not ended, its committed events are waited for
//! on the server (a long poll: the answer comes as soon as someone commits,
//! or empty after 25 s) and asked for again at once. The objects they name
//! are fetched and go into the drawing as changes from outside (no undo
//! step, nothing to save); an object with changes here becomes a conflict.
//! Without an answer the next try waits 1 s, doubling up to 30 s. A changed
//! role is asked for; a cursor the server no longer continues from opens
//! the project again. After others' changes the server's side goes to the
//! local copy first, then the draft is written (docs/adr/0043). The
//! conflict window's two choices end here too: keep mine, or take theirs.
//!
//! An object on a layer the drawing lacks waits for it (docs/adr/0081): once
//! a tree brings the layer, or an undo here, it is fetched and taken in
//! (`fetch_arrived`). When the server refuses our tree because a layer it
//! drops still holds objects, the missed events come in at once
//! (`catch_up`), then the server's tree, and the layers others drew on are
//! given back (`give_back_to`).

use std::time::Instant;

use iced::Task;
use kentos_cloud::follow::{self, EVENTS_PAGE};
use kentos_cloud::{ApiFailure, Incoming, Remote, SaveState, Taken, given_back_text};
use kentos_contracts::{ConflictReason, EventPage, ProjectInfo};

use crate::app::{App, Dialog, Message, Then};
use crate::cloud::live::conflict_text;
use crate::cloud::{Event, access_of};

impl App {
    /// Waits on the server for the events after the project's cursor.
    pub(crate) fn poll(&mut self) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let (Some(client), Some(live)) = (client, self.cloud.live.as_mut()) else {
            return Task::none();
        };
        if live.polling.is_some() || live.sync.state().ended() {
            return Task::none();
        }
        let session = live.session;
        let (task, handle) = Task::perform(
            follow::wait(&client, live.tenant, live.project, live.sync.cursor()),
            move |result| crate::cloud::msg(Event::Events { session, result }),
        )
        .abortable();
        live.polling = Some(handle.abort_on_drop());
        live.long_poll = true;
        task
    }

    /// The events after the cursor at once, without waiting on the server:
    /// a refused tree waits for them (docs/adr/0081). The long poll gives
    /// way; a fetch already on its way is taken in first and does the same.
    pub(crate) fn catch_up(&mut self) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let (Some(client), Some(live)) = (client, self.cloud.live.as_mut()) else {
            return Task::none();
        };
        if live.taking.is_some() || (live.polling.is_some() && !live.long_poll) {
            return Task::none();
        }
        let session = live.session;
        let (task, handle) = Task::perform(
            follow::events(&client, live.tenant, live.project, live.sync.cursor()),
            move |result| crate::cloud::msg(Event::Events { session, result }),
        )
        .abortable();
        live.polling = Some(handle.abort_on_drop());
        live.long_poll = false;
        task
    }

    /// Others' objects that waited for their layer, now that the drawing has
    /// it (docs/adr/0081): fetched at their current versions and taken in as
    /// others' changes. The long poll gives way and starts again after; a
    /// fetch already on its way looks again once it is taken in. None when
    /// nothing arrived.
    pub(crate) fn fetch_arrived(&mut self) -> Option<Task<Message>> {
        let client = self.cloud.signed_in().cloned()?;
        let (live, doc) = (self.cloud.live.as_mut()?, self.document.as_ref()?);
        if live.taking.is_some()
            || live.sync.state().ended()
            || (live.polling.is_some() && !live.long_poll)
        {
            return None;
        }
        let fetch = live.sync.arrived(&doc.model);
        if fetch.is_empty() {
            return None;
        }
        let incoming = Incoming {
            fetch,
            cursor: live.sync.cursor().to_owned(),
            ..Incoming::default()
        };
        let session = live.session;
        let request = follow::fetch(&client, live.tenant, live.project, &incoming);
        let (task, handle) = Task::perform(request, move |result| {
            crate::cloud::msg(Event::Fetched {
                session,
                incoming,
                full: false,
                result,
            })
        })
        .abortable();
        live.polling = Some(handle.abort_on_drop());
        live.long_poll = false;
        Some(task)
    }

    pub(crate) fn follow_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Events { session, result } => return self.events(session, result),
            Event::Fetched {
                session,
                incoming,
                full,
                result,
            } => {
                let Some(live) = self.cloud.live.as_mut().filter(|l| l.session == session) else {
                    return Task::none();
                };
                live.polling = None;
                return match result {
                    Ok(remote) => {
                        let link = self.heard(None);
                        Task::batch([link, self.take(incoming, remote, full)])
                    }
                    // The cursor did not move: the same events come again.
                    Err(failure) => self.poll_failed(&failure),
                };
            }
            Event::Access { session, result } => return self.access(session, result),
            Event::ServerTree { session, result } => return self.server_tree_came(session, result),
            Event::KeepMine => {
                let Some(live) = self.cloud.live.as_mut() else {
                    return Task::none();
                };
                // The user chose: a give-back on its way has nothing left to answer.
                live.give_back = false;
                live.giving = None;
                live.sync.keep_mine();
                live.send_soon();
                self.close_conflicts();
                self.output("Sizin değişiklikleriniz kaydediliyor.");
                return Task::batch([self.after_server_step(), self.live_tick(Instant::now())]);
            }
            Event::TakeTheirs => {
                let client = self.cloud.signed_in().cloned();
                let Some(live) = self.cloud.live.as_mut() else {
                    return Task::none();
                };
                live.give_back = false;
                live.giving = None;
                let meta = live
                    .sync
                    .conflicts()
                    .iter()
                    .any(|c| c.reason == ConflictReason::Project);
                if !meta {
                    return self.take_theirs(None);
                }
                // The metadata's conflict takes the project's info as it is now.
                let Some(client) = client else {
                    self.warn("Bulut oturumu açık değil; sunucudaki proje bilgileri alınamadı. Yeniden giriş yapın.");
                    return Task::none();
                };
                let session = live.session;
                return Task::perform(client.project(live.tenant, live.project), move |result| {
                    crate::cloud::msg(Event::TheirInfo { session, result })
                });
            }
            Event::TheirInfo { session, result } => {
                if self
                    .cloud
                    .live
                    .as_ref()
                    .is_none_or(|l| l.session != session)
                {
                    return Task::none();
                }
                match result {
                    Ok(info) => return self.take_theirs(Some(info)),
                    Err(failure) => self.warn(format!(
                        "Sunucudaki proje bilgileri alınamadı: {}",
                        failure.message
                    )),
                }
            }
            _ => {}
        }
        Task::none()
    }

    fn events(&mut self, session: u64, result: Result<EventPage, ApiFailure>) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let Some(live) = self.cloud.live.as_mut().filter(|l| l.session == session) else {
            return Task::none();
        };
        live.polling = None;
        let page = match result {
            Ok(page) => page,
            Err(failure) => return self.poll_failed(&failure),
        };
        live.poll_tries = 0;
        let full = page.events.len() >= EVENTS_PAGE;
        let incoming = live.sync.incoming(&page);
        live.long_poll = false;
        let mut tasks = vec![self.heard(None)];
        let Some(live) = self.cloud.live.as_mut() else {
            return Task::batch(tasks);
        };
        if incoming.access
            && let Some(client) = &client
        {
            let session = live.session;
            tasks.push(Task::perform(
                client.project(live.tenant, live.project),
                move |result| crate::cloud::msg(Event::Access { session, result }),
            ));
        }
        if live.sync.state() == SaveState::Deleted {
            tasks.push(self.ended());
            return Task::batch(tasks);
        }
        if incoming.needs_fetch()
            && let Some(client) = client
        {
            let (tenant, project) = (live.tenant, live.project);
            let fetch = follow::fetch(&client, tenant, project, &incoming);
            // Still following: the next wait starts once these are in.
            let (task, handle) = Task::perform(fetch, move |result| {
                crate::cloud::msg(Event::Fetched {
                    session,
                    incoming,
                    full,
                    result,
                })
            })
            .abortable();
            live.polling = Some(handle.abort_on_drop());
            tasks.push(task);
            return Task::batch(tasks);
        }
        tasks.push(self.take(incoming, Remote::default(), full));
        Task::batch(tasks)
    }

    /// Following failed: the reason decides what follows.
    fn poll_failed(&mut self, failure: &ApiFailure) -> Task<Message> {
        let Some(live) = self.cloud.live.as_mut() else {
            return Task::none();
        };
        live.polling = None;
        if failure.resync() {
            self.warn("Kaçırılan değişiklikler sunucuda artık tutulmuyor; proje sunucudan yeniden açılıyor, bu cihazdaki işiniz üstüne konacak.");
            return self.leave(Then::Reopen);
        }
        if failure.deleted() || failure.archived() || failure.not_found() {
            // A command on its way learns it itself; otherwise sending ends here.
            if live.sent.is_none() {
                let _ = live.sync.failed(failure);
                return self.ended();
            }
            live.poll_tries += 1;
            live.poll_at = Instant::now() + failure.backoff(live.poll_tries);
            return Task::none();
        }
        if failure.signed_out() {
            self.cloud.me = None;
            self.warn(format!(
                "Bulut oturumunuz sona erdi ({}); başkalarının değişiklikleri gelmiyor, sizinkiler bu cihazda bekliyor. Yeniden giriş yapın.",
                failure.message
            ));
            return Task::none();
        }
        // No answer: 1 s, doubling up to 30 s, then again from the cursor.
        live.poll_tries += 1;
        live.poll_at = Instant::now() + failure.backoff(live.poll_tries);
        self.heard(Some(failure))
    }

    /// Others' changes into the drawing; while an edit is open they wait for
    /// it. Then the next wait starts at once.
    fn take(&mut self, incoming: Incoming, remote: Remote, full: bool) -> Task<Message> {
        let (Some(live), Some(doc)) = (self.cloud.live.as_mut(), self.document.as_mut()) else {
            return Task::none();
        };
        let archived = incoming.archived;
        match live
            .sync
            .take_remote(&mut doc.model, incoming.clone(), remote.clone())
        {
            Err(_) => {
                live.taking = Some((incoming, remote, full));
                Task::none()
            }
            Ok(taken) => {
                live.poll_at = Instant::now();
                // A refused tree waited for these events (docs/adr/0081).
                let give_back = std::mem::take(&mut live.give_back);
                let asks = give_back && live.sync.may_give_back(&doc.model);
                let conflict = (give_back && !asks && live.sync.state() == SaveState::Conflict)
                    .then(|| conflict_text(&live.sync));
                self.report(&taken);
                if let Some(text) = conflict {
                    self.warn(text);
                }
                let ended = if archived { self.ended() } else { Task::none() };
                // The server's side to the copy, then the draft; then the
                // server's tree for a refused one, or the objects that waited
                // for a layer these events brought, or the next wait.
                let step = self.after_server_step();
                let next = if asks {
                    self.server_tree()
                } else {
                    self.fetch_arrived().unwrap_or_else(|| self.poll())
                };
                Task::batch([ended, step, next])
            }
        }
    }

    fn report(&mut self, taken: &Taken) {
        if taken.changed > 0 {
            self.output(format!(
                "Başka bir düzenleyicinin {} değişikliği çizime alındı.",
                taken.changed
            ));
        }
        if taken.conflicts > 0 {
            self.warn(format!(
                "Başka biri {} nesneyi değiştirdi ya da sildi; burada da değiştikleri için kayıt çakışması. Durum çubuğundaki Çakışma'ya tıklayıp seçin.",
                taken.conflicts
            ));
        }
        // A layer another editor removed stays for this device's unsent objects (docs/adr/0079).
        for kept in &taken.kept {
            self.warn(kept.text());
        }
    }

    /// What waited for an open edit to end, now that it has.
    pub(crate) fn retry_waiting(&mut self) -> Task<Message> {
        let busy = self.document.as_ref().is_none_or(|d| d.model.is_busy());
        let Some(live) = self.cloud.live.as_mut() else {
            return Task::none();
        };
        if busy {
            return Task::none();
        }
        if let Some(info) = live.giving.take() {
            return self.give_back_to(info);
        }
        if let Some(info) = live.theirs.take() {
            return self.take_theirs(info);
        }
        match live.taking.take() {
            Some((incoming, remote, full)) => self.take(incoming, remote, full),
            None => Task::none(),
        }
    }

    /// The server's copies of the conflicts (`info`: the metadata's, when it conflicted).
    fn take_theirs(&mut self, info: Option<ProjectInfo>) -> Task<Message> {
        let (Some(live), Some(doc)) = (self.cloud.live.as_mut(), self.document.as_mut()) else {
            return Task::none();
        };
        match live.sync.take_theirs(&mut doc.model, info.as_ref()) {
            Err(_) => {
                live.theirs = Some(info);
                Task::none()
            }
            Ok(kept) => {
                if live.sync.all_sent() {
                    let revision = doc.model.revision();
                    doc.model.mark_saved(revision);
                }
                self.close_conflicts();
                self.output("Sunucudaki hâller alındı.");
                for layer in &kept {
                    self.warn(layer.text());
                }
                // The server's tree may bring the layer objects waited for.
                let arrived = self.fetch_arrived().unwrap_or_else(Task::none);
                Task::batch([self.after_server_step(), arrived])
            }
        }
    }

    /// The missed events are in after a refused tree: the server's tree now
    /// (docs/adr/0081). Following goes on meanwhile.
    fn server_tree(&mut self) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let Some(live) = self.cloud.live.as_ref() else {
            return Task::none();
        };
        let Some(client) = client else {
            let text = conflict_text(&live.sync);
            self.warn(text);
            return Task::none();
        };
        let session = live.session;
        let ask = Task::perform(client.project(live.tenant, live.project), move |result| {
            crate::cloud::msg(Event::ServerTree { session, result })
        });
        Task::batch([ask, self.poll()])
    }

    fn server_tree_came(
        &mut self,
        session: u64,
        result: Result<ProjectInfo, ApiFailure>,
    ) -> Task<Message> {
        let Some(live) = self.cloud.live.as_ref().filter(|l| l.session == session) else {
            return Task::none();
        };
        match result {
            Ok(info) => self.give_back_to(info),
            Err(failure) => {
                let conflict =
                    (live.sync.state() == SaveState::Conflict).then(|| conflict_text(&live.sync));
                self.warn(format!(
                    "Katman ağacı sunucununkiyle karşılaştırılamadı: {}",
                    failure.message
                ));
                if let Some(text) = conflict {
                    self.warn(text);
                }
                Task::none()
            }
        }
    }

    /// The layers the refused tree dropped that others drew on come back
    /// from the server's tree (`ProjectSync::give_back`), each said once.
    /// When the server's guard alone refused, what is left goes at once;
    /// otherwise the conflict is said, the layers already back. Then the
    /// objects that waited for them are fetched.
    fn give_back_to(&mut self, info: ProjectInfo) -> Task<Message> {
        let (Some(live), Some(doc)) = (self.cloud.live.as_mut(), self.document.as_mut()) else {
            return Task::none();
        };
        let given = match live.sync.give_back(&mut doc.model, &info) {
            Ok(given) => given,
            Err(_) => {
                // An edit is open: once it ends.
                live.giving = Some(info);
                return Task::none();
            }
        };
        if given.guard_only {
            live.send_soon();
        }
        let conflict =
            (live.sync.state() == SaveState::Conflict).then(|| conflict_text(&live.sync));
        for name in &given.names {
            self.warn(given_back_text(name));
        }
        if let Some(text) = conflict {
            self.warn(text);
        }
        let draft = self.write_draft(false);
        let arrived = self.fetch_arrived().unwrap_or_else(Task::none);
        let send = if given.guard_only {
            self.live_tick(Instant::now())
        } else {
            Task::none()
        };
        Task::batch([draft, arrived, send])
    }

    /// Others' objects still waiting for a layer when the project is left:
    /// said once, by layer (docs/adr/0081).
    pub(crate) fn say_waiting(&mut self) {
        let texts = self
            .cloud
            .live
            .as_ref()
            .map(|l| l.sync.waiting_texts())
            .unwrap_or_default();
        for text in texts {
            self.warn(text);
        }
    }

    /// The account's access in the project changed: asked, and applied.
    fn access(&mut self, session: u64, result: Result<ProjectInfo, ApiFailure>) -> Task<Message> {
        let Some(live) = self.cloud.live.as_mut().filter(|l| l.session == session) else {
            return Task::none();
        };
        match result {
            Ok(info) => {
                let (write, meta) = access_of(&info);
                live.sync.set_access(write, meta);
                let role = crate::cloud::words::role(info.access.role);
                if let Some(source) = self.document.as_mut().and_then(|d| d.cloud_source_mut()) {
                    let changed = source.info.access.role != info.access.role;
                    source.info.access = info.access;
                    if changed {
                        let name = source.info.name.clone();
                        self.output(format!("“{name}” projesindeki rolünüz: {role}."));
                    }
                }
                Task::none()
            }
            Err(failure) if failure.not_found() && live.sent.is_none() => {
                let _ = live.sync.failed(&failure);
                self.ended()
            }
            Err(_) => Task::none(),
        }
    }

    /// Sending ended for good (deleted, archived, access taken away): said once.
    pub(crate) fn ended(&mut self) -> Task<Message> {
        let name = self
            .document
            .as_ref()
            .map_or(String::new(), |d| d.name().to_owned());
        let Some(live) = self.cloud.live.as_mut() else {
            return Task::none();
        };
        if live.told.ended || !live.sync.state().ended() {
            return Task::none();
        }
        live.told.ended = true;
        // Archived from this window: already said, without the notice (catalog_actions.rs).
        let by_me = live.sync.state() == SaveState::Archived
            && std::mem::take(&mut self.cloud.archived_by_me);
        // The copy says so too: it opens read-only from now on (docs/adr/0043).
        if let (Some(held), Some(ended)) = (
            self.cloud.held.as_ref(),
            kentos_cloud::replica::Ended::of(live.sync.state()),
        ) {
            let _ = held.replica.mark_ended(ended);
        }
        let text = match live.sync.state() {
            SaveState::Deleted => format!(
                "“{name}” bulut projesi silindi. Değişiklikleriniz artık buluta kaydedilmiyor; gönderilmemiş olanlar bu cihazda saklanıyor. Çizimi saklamak için Farklı kaydet ile yerel bir dosyaya kaydedin."
            ),
            SaveState::Archived => format!(
                "“{name}” bulut projesi arşivlendi; değişiklikleriniz artık buluta kaydedilmiyor, gönderilmemiş olanlar bu cihazda saklanıyor. Proje arşivden çıkarılınca yeniden açın: saklanan değişiklikler geri gelir."
            ),
            _ => format!(
                "“{name}” projesine erişiminiz kaldırıldı. Değişiklikleriniz artık buluta kaydedilmiyor; gönderilmemiş olanlar bu cihazda saklanıyor. Çizimi saklamak için Farklı kaydet ile yerel bir dosyaya kaydedin."
            ),
        };
        if !by_me {
            self.warn(text);
            // The web's notice (AccessLostNotice.ts): what happened, what stays, and
            // a local copy offered, unless another window is up.
            if self.dialog.is_none() {
                self.dialog = Some(Dialog::Ended);
            }
        }
        self.write_draft(false)
    }

    /// The notice of an ended project: its words by how it ended.
    pub(crate) fn ended_notice(&self) -> Option<(&'static str, String, String)> {
        let name = self.document.as_ref().map_or("", |d| d.name());
        let Some(live) = self.cloud.live.as_ref() else {
            return self.file_ended_notice(name);
        };
        let unsent = live.sync.pending();
        let kept = if unsent > 0 {
            format!(
                "Çizim ekranda kalıyor; gönderilmemiş {unsent} değişiklik bu cihazda saklanıyor."
            )
        } else {
            "Çizim ekranda kalıyor.".to_owned()
        };
        let (title, message) = match live.sync.state() {
            SaveState::Deleted => (
                "Proje çöp kutusuna taşındı",
                format!(
                    "“{name}” projesi çöp kutusuna taşındı; değişiklikleriniz bundan sonra buluta kaydedilmez."
                ),
            ),
            SaveState::Archived => (
                "Proje arşivlendi",
                format!(
                    "“{name}” projesi arşivlendi: salt okunurdur, değişiklikleriniz buluta kaydedilmez. Arşivden çıkarılınca yeniden açın; saklanan değişiklikler geri gelir."
                ),
            ),
            SaveState::Revoked => (
                "Projeye erişiminiz kaldırıldı",
                format!(
                    "“{name}” projesine artık erişemiyorsunuz; değişiklikleriniz bundan sonra buluta kaydedilmez."
                ),
            ),
            _ => return None,
        };
        Some((
            title,
            message,
            format!("{kept} Saklamak için yerel bir .kcad dosyasına kaydedin."),
        ))
    }

    /// A file project's notice (file_follow.rs): the drawing and its unsaved
    /// changes stay on screen, and a local file keeps them.
    fn file_ended_notice(&self, name: &str) -> Option<(&'static str, String, String)> {
        let ended = self.cloud.file.as_ref()?.ended?;
        let kept = if self.document.as_ref().is_some_and(|d| d.dirty()) {
            "Çizim ekranda kalıyor; kaydedilmemiş değişiklikler bu cihazda kurtarma kopyası olarak da saklanıyor."
        } else {
            "Çizim ekranda kalıyor."
        };
        let (title, message) = match ended {
            crate::cloud::revisions::Ended::Deleted => (
                "Proje çöp kutusuna taşındı",
                format!(
                    "“{name}” projesi çöp kutusuna taşındı; çizim bundan sonra buluta kaydedilmez."
                ),
            ),
            crate::cloud::revisions::Ended::Archived => (
                "Proje arşivlendi",
                format!(
                    "“{name}” projesi arşivlendi: salt okunurdur, çizim buluta kaydedilmez. Arşivden çıkarılınca yeniden açılabilir."
                ),
            ),
            crate::cloud::revisions::Ended::Revoked => (
                "Projeye erişiminiz kaldırıldı",
                format!(
                    "“{name}” projesine artık erişemiyorsunuz; çizim bundan sonra buluta kaydedilmez."
                ),
            ),
        };
        Some((
            title,
            message,
            format!("{kept} Saklamak için yerel bir .kcad dosyasına kaydedin."),
        ))
    }

    /// Opens the conflict window, if there is a conflict to choose on.
    pub(crate) fn show_conflicts(&mut self) {
        if self.cloud.file_conflict.is_some() {
            self.ask_file(crate::cloud::revisions::Via::Conflict);
        } else if self
            .cloud
            .live
            .as_ref()
            .is_some_and(|l| !l.sync.conflicts().is_empty())
        {
            self.dialog = Some(Dialog::Conflicts);
        } else {
            self.output("Çözülecek bir çakışma yok.");
        }
    }

    fn close_conflicts(&mut self) {
        if self.dialog == Some(Dialog::Conflicts) {
            self.dialog = None;
        }
    }
}
