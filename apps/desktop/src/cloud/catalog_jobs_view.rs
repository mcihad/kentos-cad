//! The catalog's Sunucu işleri tab as it is drawn (docs/adr/0547): “Projeyi dışa aktar (.kcad)”, the project's jobs
//! (what they are, their state, how far a running one is, when they were asked and ended, what a finished one made
//! or why it failed) each with Durdur and İndir, an action the account may not take stays visible, off, and says
//! why. What the words and the rights are is `kentos_cloud::server_jobs`.

use iced::widget::{Column, Row, column, container, row};
use iced::{Background, Border, Center, Element, Fill, Theme};
use kentos_cloud::server_jobs;
use kentos_commands::{ProjectSummary, ServerJob, ServerJobState};
use kentos_ui::icon::Icon;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Tip, progress, tip};
use kentos_ui::{label, style};

use super::catalog_view::{Chip, chip, cloud, muted, small_button, web};
use crate::app::{App, Message};
use crate::cloud::Event;
use crate::cloud::catalog_jobs::{JobsState, Working};
use crate::cloud::local_time::{Zone, when};
use crate::cloud::view::secondary;

/// A small row button, off with its reason as the tip (an empty reason: off while the window is held, no tip).
fn act<'a>(
    caption: &'a str,
    glyph: Icon,
    on: Message,
    why: Option<String>,
    danger: bool,
) -> Element<'a, Message> {
    let b = small_button(caption, glyph, why.is_none().then_some(on), danger);
    match why {
        Some(why) if !why.is_empty() => tip(b, Tip::new(why), iced::widget::tooltip::Position::Top),
        _ => b.into(),
    }
}

/// A row of the list (the same frame as the history's).
fn frame<'a>(body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(body)
        .padding([8, 10])
        .width(Fill)
        .style(|theme: &Theme| {
            let t = Tokens::of(theme);
            container::Style {
                background: Some(Background::Color(t.surface)),
                border: Border {
                    color: t.border,
                    width: 1.0,
                    radius: kentos_ui::theme::shape::radius(6.0).into(),
                },
                ..container::Style::default()
            }
        })
        .into()
}

fn line<'a>(text: impl Into<String>) -> Element<'a, Message> {
    muted(text.into()).width(Fill).into()
}

/// How a state looks as a chip.
fn state_chip<'a>(state: ServerJobState) -> Element<'a, Message> {
    chip(
        server_jobs::state_label(state),
        match state {
            ServerJobState::Queued | ServerJobState::Cancelled => Chip::Plain,
            ServerJobState::Running => Chip::Open,
            ServerJobState::Succeeded => Chip::Kind,
            ServerJobState::Failed => Chip::Danger,
        },
    )
}

impl App {
    /// The Sunucu işleri tab's content for `p`.
    pub(super) fn jobs_tab<'a>(&'a self, p: &'a ProjectSummary) -> Vec<Element<'a, Message>> {
        let Some(c) = &self.cloud.catalog else {
            return Vec::new();
        };
        let j = &c.jobs;
        let perms = &p.access.permissions;
        let held = self.cloud.opening.is_some() || c.busy() || j.busy();
        let mut out: Vec<Element<'a, Message>> = Vec::new();
        let ask_why = server_jobs::why_not_export(perms, p.storage, p.state);
        out.push(
            row![
                act(
                    "Projeyi dışa aktar (.kcad)",
                    web("play"),
                    cloud(Event::JobExport),
                    ask_why.or_else(|| held.then(String::new)),
                    false,
                ),
                act(
                    "Yenile",
                    web("feedRefresh"),
                    cloud(Event::JobsRetry),
                    j.busy().then(String::new),
                    false,
                ),
            ]
            .spacing(6)
            .into(),
        );
        out.push(line(
            "Sunucu işleri sunucuda çalışır: pencereyi ya da uygulamayı kapatsanız da sürer, bitince buradan indirebilirsiniz.",
        ));
        if let Some(said) = &j.status {
            let error = said.error;
            out.push(
                label::caption(said.text.clone())
                    .style(move |theme: &Theme| {
                        let t = Tokens::of(theme);
                        iced::widget::text::Style {
                            color: Some(if error { t.danger } else { t.muted }),
                        }
                    })
                    .into(),
            );
        }
        match &j.state {
            JobsState::Loading => out.push(line("İşler yükleniyor…")),
            JobsState::Failed(why) => out.push(
                column![
                    label::caption(format!("İşler okunamadı: {why}")).style(style::text::danger),
                    secondary("Yeniden dene", Some(cloud(Event::JobsRetry))),
                ]
                .spacing(8)
                .into(),
            ),
            JobsState::None => out.push(line("Bu projenin işleri gösterilemiyor.")),
            JobsState::Loaded(jobs) if jobs.is_empty() => out.push(line(
                "Henüz iş yok: projenin şimdiki hâlini tek bir .kcad dosyası olarak sunucuda hazırlatmak için “Projeyi dışa aktar”.",
            )),
            JobsState::Loaded(jobs) => {
                let me = self.cloud.me.as_ref().map(|m| m.user.id.as_str());
                out.push(
                    row![
                        label::caption("İşler")
                            .font(typography::ui_strong())
                            .style(style::text::muted),
                        muted(jobs.len().to_string()),
                    ]
                    .spacing(6)
                    .into(),
                );
                out.push(
                    jobs.iter()
                        .fold(Column::new().spacing(6), |col, job| {
                            col.push(self.job_row(job, perms, me, held))
                        })
                        .into(),
                );
            }
        }
        out
    }

    fn job_row<'a>(
        &self,
        job: &'a ServerJob,
        perms: &[kentos_commands::ProjectPermission],
        me: Option<&str>,
        held: bool,
    ) -> Element<'a, Message> {
        let zone = Zone::system();
        let stopping = matches!(
            self.cloud.catalog.as_ref().map(|c| &c.jobs.working),
            Some(Some(Working::Stopping(id))) if *id == job.id
        );
        let off = |why: Option<String>| why.or_else(|| held.then(String::new));
        let mut head = Row::new()
            .spacing(6)
            .align_y(Center)
            .push(label::caption(server_jobs::kind_label(job)).font(typography::ui_strong()));
        head = head.push(state_chip(job.state));
        if let Some(percent) = server_jobs::progress_text(job) {
            head = head.push(muted(percent));
        }
        let mut main = column![head.wrap().vertical_spacing(4)].spacing(3);
        if job.state == ServerJobState::Running {
            main = main.push(progress::bar(Some(job.progress.clamp(0.0, 1.0))));
        }
        let mut times = format!("istendi {}", when(&job.created_at, &zone));
        if let Some(end) = job.finished_at.as_deref().filter(|e| !e.is_empty()) {
            times.push_str(&format!(" · bitti {}", when(end, &zone)));
        }
        if job.attempts > 1 {
            times.push_str(&format!(" · {}. deneme", job.attempts));
        }
        main = main.push(muted(times));
        if let Some(done) = server_jobs::result_text(job) {
            main = main.push(label::caption(done));
        }
        if let Some(why) = server_jobs::failure_text(job) {
            main = main.push(label::caption(why).style(style::text::danger).width(Fill));
        }
        let acts = Row::new()
            .spacing(6)
            .push(act(
                "İndir",
                web("export"),
                cloud(Event::JobDownload(job.clone())),
                off(server_jobs::why_not_download(job, perms)),
                false,
            ))
            .push(act(
                if stopping {
                    "Durduruluyor…"
                } else {
                    "Durdur"
                },
                web("close"),
                cloud(Event::JobStop(job.id.clone())),
                off(server_jobs::why_not_stop(job, me, perms)),
                true,
            ));
        frame(column![main, acts.wrap().vertical_spacing(6)].spacing(6))
    }
}
