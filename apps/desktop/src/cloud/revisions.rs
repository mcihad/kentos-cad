//! An open file project's revisions (docs/specs/file-revisions.md; the web's
//! app/cloud/fileRevisionsPlan.ts): what the drawing knows of the server's
//! newest revision and how it learns it (an event says someone committed a
//! revision; the server is then asked which, by whom and when), the save
//! cell's state, what Kaydet does first, what a resync asks when the missed
//! events cannot be replayed, the question a click brings and what each
//! answer does to the drawing and its unsaved work, and the words. A newer
//! revision is said and offered, never loaded by itself.
//!
//! Apart from the windows and the network: file_follow.rs follows the
//! events and asks, file.rs saves, cells.rs says the cell, view.rs asks the
//! questions. Both platforms play fixtures/cloud/v1/file-revisions.json.

use kentos_contracts::{EventRecord, FileRevisions, ProjectState};

use super::cells_plan::FileState;
use super::local_time::{Zone, when};

/// A newer revision than the drawing's: its number, who saved it and when,
/// as far as the server said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NewerRevision {
    pub revision: String,
    /// Who saved it; empty when not known (a refused Kaydet names only the number).
    pub by: String,
    /// When it was saved (RFC 3339); none when not known.
    pub at: Option<String>,
}

/// The revisions a refused Kaydet met: the drawing's base and the server's newest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RevisionConflict {
    pub expected: String,
    pub actual: String,
}

/// Where a Kaydet is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SaveStage {
    #[default]
    Idle,
    Encoding,
    Uploading,
    Verifying,
}

/// Why nothing more is saved to the project.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ended {
    Deleted,
    Revoked,
    Archived,
}

/// What the open file project knows: the facts its save cell and Kaydet read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RevisionState {
    /// The revision the drawing on screen is based on ("0": none yet).
    pub base: String,
    /// A newer revision than `base` on the server, as far as known.
    pub newer: Option<NewerRevision>,
    /// Set when a Kaydet was refused (or known to be): until the user chooses, Kaydet asks again.
    pub conflict: Option<RevisionConflict>,
    /// The drawing has changes no revision holds.
    pub dirty: bool,
    pub stage: SaveStage,
    /// The last Kaydet failed for another reason than a newer revision;
    /// forgotten when the next one begins.
    pub failed: bool,
    /// This account may write revisions (feature.write).
    pub writable: bool,
    pub ended: Option<Ended>,
}

/// What changes an open file project's state. The desktop keeps the
/// drawing's changes, a Kaydet's stage and failure, the access and the end
/// where they already were (file_follow.rs `file_revisions`): only a newest
/// revision, a refusal and a commit go through [`step`] there. The others
/// are the plan's, which the fixture plays.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum RevisionInput {
    /// The drawing's unsaved changes came, or went (undone back to the saved state).
    Dirty(bool),
    /// The server named its newest revision (asked after someone else's
    /// event, after a resync, or on a refused Kaydet); none: none yet.
    Newest(Option<NewerRevision>),
    /// A Kaydet reached a stage; `Encoding` begins one, and its last failure is forgotten.
    Stage(SaveStage),
    /// The server committed this window's Kaydet as `revision`; `dirty`:
    /// edits made while it went stay unsaved.
    Committed { revision: String, dirty: bool },
    /// Someone saved `actual` first: the server refused the Kaydet
    /// (`@file`), or Kaydet knew it would.
    Refused { actual: String },
    /// The Kaydet failed otherwise: no answer, or a refusal the drawing can do nothing about.
    Failed,
    /// A Kaydet found the drawing as its base: nothing to write, and the last failure is forgotten.
    Unchanged,
    /// What this account may do changed.
    Access(bool),
    /// The project was deleted, archived, or is out of reach: nothing more is saved there.
    Ended(Ended),
}

/// A revision's number (decimal text); 0 for anything else.
fn num(revision: &str) -> u64 {
    revision.parse().unwrap_or(0)
}

/// One input: the state after it, and whether a newer revision became
/// known and is to be said (once; the log line). Once the project ended
/// only the drawing's own changes count.
pub(crate) fn step(s: &RevisionState, i: &RevisionInput) -> (RevisionState, bool) {
    let same = (s.clone(), false);
    if s.ended.is_some() && !matches!(i, RevisionInput::Dirty(_)) {
        return same;
    }
    let mut next = s.clone();
    match i {
        RevisionInput::Dirty(dirty) => {
            next.dirty = *dirty;
            (next, false)
        }
        RevisionInput::Newest(newest) => {
            let Some(n) = newest else {
                return same;
            };
            if num(&n.revision) <= num(&s.base) {
                return same;
            }
            if let Some(known) = &s.newer {
                if num(&n.revision) < num(&known.revision) {
                    return same;
                }
                if n.revision == known.revision {
                    // Said once; what was not known of it (a refused Kaydet
                    // names only the number) is filled in.
                    let by = if known.by.is_empty() {
                        n.by.clone()
                    } else {
                        known.by.clone()
                    };
                    let at = known.at.clone().or_else(|| n.at.clone());
                    if by == known.by && at == known.at {
                        return same;
                    }
                    next.newer = Some(NewerRevision {
                        revision: known.revision.clone(),
                        by,
                        at,
                    });
                    return (next, false);
                }
            }
            // A standing conflict is with the newest revision known.
            next.conflict = s.conflict.as_ref().map(|c| RevisionConflict {
                expected: c.expected.clone(),
                actual: n.revision.clone(),
            });
            next.newer = Some(n.clone());
            (next, true)
        }
        RevisionInput::Stage(stage) => {
            next.stage = *stage;
            if *stage == SaveStage::Encoding {
                next.failed = false;
            }
            (next, false)
        }
        RevisionInput::Committed { revision, dirty } => {
            next.newer = s.newer.clone().filter(|n| num(&n.revision) > num(revision));
            next.base.clone_from(revision);
            next.dirty = *dirty;
            next.stage = SaveStage::Idle;
            next.failed = false;
            (next, false)
        }
        RevisionInput::Refused { actual } => {
            // What is known of the newer revision stays when it is the same one, or a later one.
            let newer = s
                .newer
                .clone()
                .filter(|n| num(&n.revision) >= num(actual))
                .unwrap_or_else(|| NewerRevision {
                    revision: actual.clone(),
                    by: String::new(),
                    at: None,
                });
            next.conflict = Some(RevisionConflict {
                expected: s.base.clone(),
                actual: newer.revision.clone(),
            });
            next.newer = Some(newer);
            next.stage = SaveStage::Idle;
            (next, false)
        }
        RevisionInput::Failed => {
            next.stage = SaveStage::Idle;
            next.failed = true;
            (next, false)
        }
        RevisionInput::Unchanged => {
            next.failed = false;
            (next, false)
        }
        RevisionInput::Access(writable) => {
            next.writable = *writable;
            (next, false)
        }
        RevisionInput::Ended(why) => {
            next.ended = Some(*why);
            next.stage = SaveStage::Idle;
            (next, false)
        }
    }
}

/// The save cell's state: why nothing is saved any more; a Kaydet's stage;
/// a conflict; a newer revision (also over unsaved work, and while
/// read-only: it can be opened); read-only; the last Kaydet's failure;
/// unsaved changes; saved.
pub(crate) fn cell_state(s: &RevisionState) -> FileState {
    if let Some(ended) = s.ended {
        return match ended {
            Ended::Deleted => FileState::Deleted,
            Ended::Revoked => FileState::Revoked,
            Ended::Archived => FileState::Archived,
        };
    }
    match s.stage {
        SaveStage::Encoding => return FileState::Encoding,
        SaveStage::Uploading => return FileState::Uploading,
        SaveStage::Verifying => return FileState::Verifying,
        SaveStage::Idle => {}
    }
    if s.conflict.is_some() {
        FileState::Conflict
    } else if s.newer.is_some() {
        FileState::Outdated
    } else if !s.writable {
        FileState::ReadOnly
    } else if s.failed {
        FileState::Error
    } else if s.dirty {
        FileState::Pending
    } else {
        FileState::Saved
    }
}

/// What a Kaydet does first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SaveStep {
    Ended,
    ReadOnly,
    Conflict,
    Unchanged,
    Behind,
    Save,
}

/// What a Kaydet does first: nothing where nothing is saved any more or
/// the account may not write; the question again while a conflict stands;
/// nothing when the drawing is its base (a project without a revision
/// writes its first even so); the question when a newer revision is known
/// (`Behind`: the server would refuse, so nothing is uploaded; the state
/// takes it as refused); else it writes the next revision on `base`.
pub(crate) fn save_step(s: &RevisionState) -> SaveStep {
    if s.ended.is_some() {
        SaveStep::Ended
    } else if !s.writable {
        SaveStep::ReadOnly
    } else if s.conflict.is_some() {
        SaveStep::Conflict
    } else if !s.dirty && s.base != "0" {
        SaveStep::Unchanged
    } else if s.newer.is_some() {
        SaveStep::Behind
    } else {
        SaveStep::Save
    }
}

// ── Events ─────────────────────────────────────────────────────────────────

/// What a batch of the project's events asks for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct EventsRead {
    /// The last event's cursor; none for an empty batch.
    pub cursor: Option<String>,
    /// The project was deleted or archived: nothing after it counts. The
    /// flag: this window archived it (quiet).
    pub end: Option<(Ended, bool)>,
    /// A grant changed: what this account may do is asked again.
    pub access: bool,
    /// Someone else committed a revision: the server is asked which is its
    /// newest (once for the batch).
    pub newest: bool,
}

/// Reads a batch of the project's events, in order: `project.deleted` and
/// `project.archived` end it; `project.access` asks the access again;
/// `project.file` from another request than this window's own commits asks
/// the newest revision. The event names no revision number (its
/// `dataRevision` is the project's, not the file's).
pub(crate) fn read_events(events: &[EventRecord], own: impl Fn(&str) -> bool) -> EventsRead {
    let mut read = EventsRead::default();
    for e in events {
        read.cursor = Some(e.seq.clone());
        let mine = e.request_id.as_deref().is_some_and(&own);
        match e.kind.as_str() {
            "project.deleted" => {
                return EventsRead {
                    cursor: read.cursor,
                    end: Some((Ended::Deleted, false)),
                    ..EventsRead::default()
                };
            }
            "project.archived" => {
                return EventsRead {
                    cursor: read.cursor,
                    end: Some((Ended::Archived, mine)),
                    ..EventsRead::default()
                };
            }
            "project.access" => read.access = true,
            "project.file" if !mine => read.newest = true,
            _ => {}
        }
    }
    read
}

/// The server's newest revision from its list (`GET …/files`): who saved
/// it and when; none before the first.
pub(crate) fn newest_of(r: &FileRevisions) -> Option<NewerRevision> {
    let current = r.current.as_ref()?;
    let listed = r.revisions.iter().find(|x| &x.revision == current);
    Some(NewerRevision {
        revision: current.clone(),
        by: listed.map_or_else(String::new, |x| x.created_by_name.clone()),
        at: listed.map(|x| x.created_at.clone()),
    })
}

// ── Resync ─────────────────────────────────────────────────────────────────

/// How long a resync that got no answer waits before the events are asked again.
pub(crate) const RESYNC_RETRY: std::time::Duration = std::time::Duration::from_secs(30);

/// The server's answer about the project (`GET …/projects/{id}`) when the
/// events it missed cannot be replayed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ProjectAnswer {
    Project {
        state: ProjectState,
        event_cursor: String,
    },
    /// 410: in the trash.
    Deleted,
    /// 404: gone for this account (removed, or its access taken).
    NotFound,
    /// 403: its organisation may not be used now; the server says why.
    Forbidden(String),
    /// No answer, after the passing failures were tried again.
    Unreachable,
    /// Any other refusal.
    Failed(String),
}

/// What a resync does next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResyncStep {
    /// Nothing more is saved there (`reason`: the server's words, when it gave some).
    End { why: Ended, reason: String },
    /// The same later ([`RESYNC_RETRY`]): the drawing and what is known stay.
    Retry,
    /// Events are followed from `cursor`; the access is asked again and the
    /// newest revision asked, as their events would.
    Follow { cursor: String },
}

/// A resync (the server keeps no events from the cursor any more, or the
/// cursor is beyond its newest): the project is asked what became of it;
/// the drawing is never replaced.
pub(crate) fn resync_step(a: &ProjectAnswer) -> ResyncStep {
    let end = |why, reason: &str| ResyncStep::End {
        why,
        reason: reason.to_owned(),
    };
    match a {
        ProjectAnswer::Project {
            state: ProjectState::Trashed,
            ..
        }
        | ProjectAnswer::Deleted => end(Ended::Deleted, ""),
        ProjectAnswer::Project {
            state: ProjectState::Archived,
            ..
        } => end(Ended::Archived, ""),
        ProjectAnswer::Project { event_cursor, .. } => ResyncStep::Follow {
            cursor: event_cursor.clone(),
        },
        ProjectAnswer::NotFound => end(Ended::Revoked, ""),
        ProjectAnswer::Forbidden(message) => end(Ended::Revoked, message),
        ProjectAnswer::Unreachable | ProjectAnswer::Failed(_) => ResyncStep::Retry,
    }
}

// ── Questions ──────────────────────────────────────────────────────────────

/// What an answer does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnswerDoes {
    /// The upload window on file storage, named “<name> (kopya)”: the
    /// drawing becomes a new file project (its revision 1) and the open
    /// one; this project stays as it is.
    Copy,
    /// Farklı kaydet: the drawing goes to a local .kcad and leaves the
    /// cloud project, which stays as it is.
    Local,
    /// The server's newest revision is opened: it replaces the drawing and is its base.
    Latest,
    /// Nothing: the drawing, its unsaved changes, a conflict and a newer revision stay.
    Nothing,
}

/// What an answer does to the drawing's unsaved changes (their recovery
/// copy, and a save kept on this device, docs/adr/0043).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnswerWork {
    /// Written by it (into the copy, or the local file): saved once that is written.
    Saved,
    /// Dropped on purpose: their recovery copy (a kept save) goes too.
    Dropped,
    /// Kept on screen, unsaved; their recovery copy too.
    Kept,
    /// The drawing has none.
    None,
}

/// An answer's value, as the web names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    Copy,
    Local,
    Latest,
    Stay,
    Discard,
    Open,
    Later,
}

impl Answer {
    /// The web's name (tests).
    #[cfg(test)]
    pub(crate) fn name(self) -> &'static str {
        match self {
            Answer::Copy => "copy",
            Answer::Local => "local",
            Answer::Latest => "latest",
            Answer::Stay => "stay",
            Answer::Discard => "discard",
            Answer::Open => "open",
            Answer::Later => "later",
        }
    }
}

/// How an answer's button looks: the one amber button (Enter), or one that removes something.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnswerKind {
    Primary,
    Danger,
}

/// One answer of a question.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RevisionAnswer {
    pub value: Answer,
    pub label: &'static str,
    pub kind: Option<AnswerKind>,
    /// On the left of the bar, apart from the others.
    pub aside: bool,
    pub does: AnswerDoes,
    pub work: AnswerWork,
}

/// Which question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QuestionId {
    Conflict,
    Unsaved,
    Newest,
}

impl QuestionId {
    /// The web's name (tests).
    #[cfg(test)]
    pub(crate) fn name(self) -> &'static str {
        match self {
            QuestionId::Conflict => "conflict",
            QuestionId::Unsaved => "unsaved",
            QuestionId::Newest => "newest",
        }
    }
}

/// A question about the open file project's revisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RevisionQuestion {
    pub id: QuestionId,
    pub title: &'static str,
    pub message: String,
    /// What each choice does, point by point.
    pub details: Vec<String>,
    /// In the bar's order (the aside ones go left).
    pub answers: Vec<RevisionAnswer>,
    /// The answer of Esc, × and the backdrop: the one that changes nothing.
    pub cancel: Answer,
}

/// How a line instead of a question is said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    Info,
    Warn,
}

/// What asking brings: a question, a line instead of one, or nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Offer {
    Ask(RevisionQuestion),
    Say(Tone, String),
    None,
}

/// Which way the question is asked: the newest revision offered (the save
/// cell's click on a newer revision, or Son revizyonu aç…), or a conflict
/// (a refused Kaydet, or Kayıt çakışmalarını çöz…).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Via {
    Newest,
    Conflict,
}

/// ` (who, when)` of a revision, as far as known; empty when neither is.
pub(crate) fn revision_who(n: &NewerRevision, zone: &Zone) -> String {
    let at =
        n.at.as_deref()
            .map_or_else(String::new, |at| when(at, zone));
    let parts: Vec<&str> = [n.by.as_str(), at.as_str()]
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join(", "))
    }
}

/// “Açık çizim revizyon 4”, or that it has none yet; `lower` inside a sentence.
fn open_at(base: &str, lower: bool) -> String {
    let a = if lower { 'a' } else { 'A' };
    if base == "0" {
        format!("{a}çık çizimin henüz revizyonu yok")
    } else {
        format!("{a}çık çizim revizyon {base}")
    }
}

/// The question after a refused Kaydet, or before the newest revision over
/// unsaved work: the drawing needs a place first. None without a conflict
/// or a newer revision.
pub(crate) fn conflict_question(
    name: &str,
    s: &RevisionState,
    zone: &Zone,
) -> Option<RevisionQuestion> {
    let c = s.conflict.clone().or_else(|| {
        s.newer.as_ref().map(|n| RevisionConflict {
            expected: s.base.clone(),
            actual: n.revision.clone(),
        })
    })?;
    // A standing conflict is always with the newer revision known (`step`): who saved it and when are its.
    let who = s
        .newer
        .as_ref()
        .map_or_else(String::new, |n| revision_who(n, zone));
    let on = if c.expected == "0" {
        "çiziminiz henüz kaydedilmiş bir revizyona dayanmıyor".to_owned()
    } else {
        format!("çiziminizin dayandığı revizyon {}", c.expected)
    };
    let w = |had: AnswerWork| if s.dirty { had } else { AnswerWork::None };
    let actual = &c.actual;
    Some(RevisionQuestion {
        id: QuestionId::Conflict,
        title: "Dosya başka biri tarafından kaydedildi",
        message: format!(
            "“{name}” siz çalışırken başka biri tarafından kaydedildi: sunucuda revizyon {actual}{who} var; {on}. Hiçbir şey yazılmadı; iki dosya birleştirilmez."
        ),
        details: vec![
            format!(
                "Ayrı kopya olarak kaydet: çiziminiz yeni bir bulut dosya projesi olur ve açık proje o olur; “{name}” olduğu gibi kalır."
            ),
            "Yerel dosyaya kaydet: çiziminiz bu bilgisayara .kcad olarak kaydedilir ve çizim buluttaki projeden ayrılır.".to_owned(),
            if s.dirty {
                format!(
                    "Son revizyonu aç: revizyon {actual} açılır; bu çizimdeki kaydedilmemiş değişiklikler atılır."
                )
            } else {
                format!("Son revizyonu aç: revizyon {actual} açılır.")
            },
        ],
        answers: vec![
            RevisionAnswer {
                value: Answer::Latest,
                label: "Son revizyonu aç",
                kind: Some(AnswerKind::Danger),
                aside: true,
                does: AnswerDoes::Latest,
                work: w(AnswerWork::Dropped),
            },
            RevisionAnswer {
                value: Answer::Stay,
                label: "Vazgeç",
                kind: None,
                aside: false,
                does: AnswerDoes::Nothing,
                work: w(AnswerWork::Kept),
            },
            RevisionAnswer {
                value: Answer::Local,
                label: "Yerel dosyaya kaydet",
                kind: None,
                aside: false,
                does: AnswerDoes::Local,
                work: w(AnswerWork::Saved),
            },
            RevisionAnswer {
                value: Answer::Copy,
                label: "Ayrı kopya olarak kaydet",
                kind: Some(AnswerKind::Primary),
                aside: false,
                does: AnswerDoes::Copy,
                work: w(AnswerWork::Saved),
            },
        ],
        cancel: Answer::Stay,
    })
}

/// Unsaved work and no newer revision known: the newest may be the
/// drawing's own base, and opening it drops the work.
pub(crate) fn unsaved_question(name: &str, base: &str) -> RevisionQuestion {
    RevisionQuestion {
        id: QuestionId::Unsaved,
        title: "Kaydedilmemiş değişiklikler",
        message: format!(
            "“{name}” içinde kaydedilmemiş değişiklikler var. Sunucudaki en yeni revizyon açılırsa bu değişiklikler atılır ({}). Saklamak için önce Kaydet ile kaydedin.",
            open_at(base, true)
        ),
        details: Vec::new(),
        answers: vec![
            RevisionAnswer {
                value: Answer::Discard,
                label: "Kaydetmeden aç",
                kind: None,
                aside: true,
                does: AnswerDoes::Latest,
                work: AnswerWork::Dropped,
            },
            RevisionAnswer {
                value: Answer::Stay,
                label: "Vazgeç",
                kind: None,
                aside: false,
                does: AnswerDoes::Nothing,
                work: AnswerWork::Kept,
            },
        ],
        cancel: Answer::Stay,
    }
}

/// A clean drawing: a plain question.
pub(crate) fn newest_question(
    name: &str,
    base: &str,
    newer: Option<&NewerRevision>,
    zone: &Zone,
) -> RevisionQuestion {
    let open = open_at(base, false);
    RevisionQuestion {
        id: QuestionId::Newest,
        title: "Son revizyonu aç",
        message: match newer {
            Some(n) => format!(
                "“{name}” başka bir yerde kaydedildi: revizyon {}{}. {open}; kaydedilmemiş değişikliği yok.",
                n.revision,
                revision_who(n, zone)
            ),
            None => format!(
                "“{name}” projesinin sunucudaki en yeni revizyonu açılsın mı? {open}; kaydedilmemiş değişikliği yok."
            ),
        },
        details: Vec::new(),
        answers: vec![
            RevisionAnswer {
                value: Answer::Later,
                label: "Sonra",
                kind: None,
                aside: false,
                does: AnswerDoes::Nothing,
                work: AnswerWork::None,
            },
            RevisionAnswer {
                value: Answer::Open,
                label: "Son revizyonu aç",
                kind: Some(AnswerKind::Primary),
                aside: false,
                does: AnswerDoes::Latest,
                work: AnswerWork::None,
            },
        ],
        cancel: Answer::Later,
    }
}

/// What asking brings. `Via::Newest`: the save cell's click on a newer
/// revision, or Son revizyonu aç…: a line while a Kaydet is on its way or
/// where the project cannot be read any more; the conflict's question over
/// unsaved work (or a standing conflict); the unsaved question when no
/// newer revision is known; else the plain question. `Via::Conflict`: a
/// refused Kaydet, or Kayıt çakışmalarını çöz…: the conflict's question, or
/// nothing.
pub(crate) fn offer(name: &str, s: &RevisionState, busy: bool, via: Via, zone: &Zone) -> Offer {
    let ask = |question: Option<RevisionQuestion>| question.map_or(Offer::None, Offer::Ask);
    if via == Via::Conflict {
        return ask(conflict_question(name, s, zone));
    }
    if busy {
        return Offer::Say(Tone::Info, texts::busy(name));
    }
    if let Some(ended @ (Ended::Deleted | Ended::Revoked)) = s.ended {
        return Offer::Say(Tone::Warn, texts::unreadable(ended, name));
    }
    if s.conflict.is_some() || (s.dirty && s.newer.is_some()) {
        return ask(conflict_question(name, s, zone));
    }
    if s.dirty {
        return Offer::Ask(unsaved_question(name, &s.base));
    }
    Offer::Ask(newest_question(name, &s.base, s.newer.as_ref(), zone))
}

// ── Words ──────────────────────────────────────────────────────────────────

pub(crate) mod texts {
    use super::Ended;

    /// Son revizyonu aç while a Kaydet is on its way.
    pub(crate) fn busy(name: &str) -> String {
        format!("“{name}” kaydediliyor; son revizyonu açmak için kaydın bitmesini bekleyin.")
    }

    /// Son revizyonu aç where the project cannot be read any more.
    pub(crate) fn unreadable(ended: Ended, name: &str) -> String {
        match ended {
            Ended::Revoked => format!(
                "“{name}” projesine erişiminiz kaldırıldı; son revizyonu açılamaz. Çizimi saklamak için yerel bir dosyaya kaydedin."
            ),
            _ => format!(
                "“{name}” bulut projesi silindi; son revizyonu açılamaz. Çizimi saklamak için yerel bir dosyaya kaydedin."
            ),
        }
    }

    /// A resync got no answer (said once until one comes).
    pub(crate) fn resync_failed(name: &str) -> String {
        format!(
            "“{name}” projesinin kaçırılan olayları alınamadı ve sunucu yanıt vermedi; başkasının kaydettiği bir revizyon şimdilik bilinmiyor. Yarım dakika sonra yeniden sorulur; çizim olduğu gibi duruyor."
        )
    }

    /// A local file written from the conflict's question.
    pub(crate) fn detached(name: &str) -> String {
        format!(
            "Çizim yerel dosyaya kaydedildi ve “{name}” bulut projesinden ayrıldı; proje olduğu gibi duruyor."
        )
    }

    /// The newest revision could not be opened.
    pub(crate) fn open_failed(name: &str, why: &str) -> String {
        format!("“{name}” son revizyonu açılamadı: {why}")
    }
}

/// The log line when a newer revision becomes known (warning): who saved
/// it and when, the drawing's base, and what a click on the save cell
/// offers; over unsaved work, that Kaydet cannot write over it.
pub(crate) fn newer_line(
    name: &str,
    newer: &NewerRevision,
    base: &str,
    dirty: bool,
    zone: &Zone,
) -> String {
    let on = if base == "0" {
        "Açık çizimin henüz revizyonu yok".to_owned()
    } else {
        format!("Açık çizimin dayandığı revizyon: {base}")
    };
    let then = if dirty {
        "kaydedilmemiş değişiklikleriniz Kaydet ile bu revizyonun üzerine yazılamaz. Ayrı kopya, yerel dosya ya da son revizyon için durum çubuğundaki kayıt durumuna tıklayın."
    } else {
        "yeni revizyonu açmak için durum çubuğundaki kayıt durumuna tıklayın."
    };
    format!(
        "“{name}” başka bir yerde kaydedildi: revizyon {}{}. {on}; {then} Kendiliğinden yeniden yüklenmez.",
        newer.revision,
        revision_who(newer, zone)
    )
}

/// The tip's line about a newer revision (cells_plan.rs `file_tip`).
pub(crate) fn newer_tip(newer: &NewerRevision, dirty: bool, zone: &Zone) -> String {
    let head = format!(
        "Sunucuda daha yeni revizyon var: {}{}",
        newer.revision,
        revision_who(newer, zone)
    );
    if dirty {
        format!(
            "{head}; kaydedilmemiş değişiklikleriniz Kaydet ile onun üzerine yazılamaz. Seçenekler için tıklayın."
        )
    } else {
        format!("{head}; açmak için tıklayın.")
    }
}

/// The history's marks of a revision's row.
pub(crate) mod marks {
    /// The server's newest revision.
    pub(crate) const NEWEST: &str = "En yeni";
    /// The revision the open drawing is based on.
    pub(crate) const BASE: &str = "Açık çizim";
}

/// The marks of a revision's row in the history: the newest; the one the
/// open drawing is based on (when the project is open here).
pub(crate) fn revision_marks(
    revision: &str,
    current: Option<&str>,
    open_base: Option<&str>,
) -> Vec<&'static str> {
    let mut out = Vec::new();
    if current == Some(revision) {
        out.push(marks::NEWEST);
    }
    if open_base == Some(revision) {
        out.push(marks::BASE);
    }
    out
}
