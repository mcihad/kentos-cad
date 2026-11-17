//! What each side may do with a dossier and what each step makes of it (docs/adr/9000 §5). Pure: the use cases
//! (`steps.rs`) ask it under the dossier's lock, and the page shows the steps it allows.
//!
//! An application goes from the professional (draft → submitted) to the municipality, which reviews it, asks for
//! changes (the professional sends a corrected round), approves or rejects it; the professional may withdraw it until
//! it is decided. A transfer goes from the municipality to the professional, who takes it on or declines, and delivers
//! rounds until the municipality completes it (or asks for revisions); the municipality may cancel it until then.
//! In the organisation, reading takes a seat; working on dossiers is for editors and above; deciding (approving,
//! rejecting, sending and completing transfers) for project managers and above.

use kentos_contracts::{DossierAction, DossierDirection, DossierSide, DossierStatus, TenantRole};

/// Who asks: the dossier's professional, a colleague of the office it is shared with (docs/adr/9000 §18), or a member
/// of its organisation, the last two with their role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Party {
    Professional,
    Colleague(TenantRole),
    Member(TenantRole),
}

impl Party {
    pub fn side(self) -> DossierSide {
        match self {
            Party::Professional | Party::Colleague(_) => DossierSide::Professional,
            Party::Member(_) => DossierSide::Municipality,
        }
    }

    fn at_least(self, role: TenantRole) -> bool {
        matches!(self, Party::Member(r) if r >= role)
    }

    /// Prepares the professional's side: the professional, or an editor of their office and above.
    pub fn works_for_professional(self) -> bool {
        matches!(self, Party::Professional) || matches!(self, Party::Colleague(r) if r >= TenantRole::Editor)
    }

    /// Takes the professional's steps (sends, withdraws, takes on, delivers): the professional, or a project manager
    /// of their office and above, as the organisation's decisions are its managers'.
    pub fn decides_for_professional(self) -> bool {
        matches!(self, Party::Professional) || matches!(self, Party::Colleague(r) if r >= TenantRole::ProjectManager)
    }
}

/// Whether a note goes with the step, and of what kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Note {
    None,
    Optional,
    /// The step is not taken without one (a refusal, a deficiency list says why).
    Required,
}

/// What a step makes of the dossier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub to: DossierStatus,
    /// A package goes with it: the pending files become round `round + 1`.
    pub sends: bool,
    /// The package may not be empty.
    pub needs_files: bool,
    pub note: Note,
    /// The kind of message a note is kept as.
    pub note_kind: &'static str,
}

pub fn closed(status: DossierStatus) -> bool {
    matches!(
        status,
        DossierStatus::Approved
            | DossierStatus::Rejected
            | DossierStatus::Withdrawn
            | DossierStatus::Declined
            | DossierStatus::Completed
            | DossierStatus::Cancelled
    )
}

/// The side the dossier waits for, none once it is closed.
pub fn waiting_for(direction: DossierDirection, status: DossierStatus) -> Option<DossierSide> {
    use DossierStatus as S;
    match (direction, status) {
        (DossierDirection::Application, S::Draft | S::ChangesRequested) => {
            Some(DossierSide::Professional)
        }
        (DossierDirection::Application, S::Submitted | S::InReview) => {
            Some(DossierSide::Municipality)
        }
        (DossierDirection::Transfer, S::Draft | S::Delivered) => Some(DossierSide::Municipality),
        (DossierDirection::Transfer, S::Sent | S::Accepted | S::RevisionRequested) => {
            Some(DossierSide::Professional)
        }
        _ => None,
    }
}

/// The side preparing the next package now, if any: files may be added to it.
pub fn preparing(direction: DossierDirection, status: DossierStatus) -> Option<DossierSide> {
    use DossierStatus as S;
    match (direction, status) {
        (DossierDirection::Application, S::Draft | S::ChangesRequested) => {
            Some(DossierSide::Professional)
        }
        (DossierDirection::Transfer, S::Draft) => Some(DossierSide::Municipality),
        (DossierDirection::Transfer, S::Accepted | S::RevisionRequested) => {
            Some(DossierSide::Professional)
        }
        _ => None,
    }
}

/// Whether `party` adds files to the package being prepared.
pub fn can_add_files(direction: DossierDirection, status: DossierStatus, party: Party) -> bool {
    match preparing(direction, status) {
        Some(DossierSide::Professional) => party.works_for_professional(),
        Some(DossierSide::Municipality) => party.at_least(TenantRole::ProjectManager),
        None => false,
    }
}

/// Whether `party` changes the dossier's title, kind, place and the like: its sending side while preparing it.
pub fn can_edit(direction: DossierDirection, status: DossierStatus, party: Party) -> bool {
    use DossierStatus as S;
    match direction {
        DossierDirection::Application => {
            party.works_for_professional() && matches!(status, S::Draft | S::ChangesRequested)
        }
        DossierDirection::Transfer => {
            status == S::Draft && party.at_least(TenantRole::ProjectManager)
        }
    }
}

/// Whether `party` removes it: only a draft, by its side.
pub fn can_delete(direction: DossierDirection, status: DossierStatus, party: Party) -> bool {
    status == DossierStatus::Draft
        && can_edit(direction, status, party)
        && (!matches!(party, Party::Colleague(_)) || party.decides_for_professional())
}

/// Whether `party` writes to the other side: once it was sent and until it is closed; editors and above in the
/// organisation.
pub fn can_message(status: DossierStatus, party: Party) -> bool {
    status != DossierStatus::Draft
        && !closed(status)
        && (party.works_for_professional() || party.at_least(TenantRole::Editor))
}

/// Why a step may not be taken: the caller's role (forbidden) or the dossier's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub message: &'static str,
    pub role: bool,
}

const fn step(to: DossierStatus, note: Note, note_kind: &'static str) -> Step {
    Step {
        to,
        sends: false,
        needs_files: false,
        note,
        note_kind,
    }
}

/// The step `action` takes from `status` for `party`, or why it may not be taken.
pub fn step_of(
    direction: DossierDirection,
    status: DossierStatus,
    action: DossierAction,
    party: Party,
) -> Result<Step, Refusal> {
    use DossierAction as A;
    use DossierStatus as S;
    let pro = party.decides_for_professional();
    let colleague = matches!(party, Party::Colleague(_));
    let editor = party.at_least(TenantRole::Editor);
    let manager = party.at_least(TenantRole::ProjectManager);
    let member = matches!(party, Party::Member(_));
    let found = match (direction, action, status) {
        // ── An application ──
        (DossierDirection::Application, A::Send, S::Draft | S::ChangesRequested) if pro => Step {
            sends: true,
            needs_files: true,
            ..step(S::Submitted, Note::Optional, "note")
        },
        (
            DossierDirection::Application,
            A::Withdraw,
            S::Submitted | S::InReview | S::ChangesRequested,
        ) if pro => step(S::Withdrawn, Note::Optional, "note"),
        (DossierDirection::Application, A::StartReview, S::Submitted) if editor => {
            step(S::InReview, Note::None, "note")
        }
        (DossierDirection::Application, A::Assign, S::Submitted | S::InReview) if editor => {
            step(status, Note::None, "note")
        }
        (DossierDirection::Application, A::RequestChanges, S::Submitted | S::InReview)
            if editor =>
        {
            step(S::ChangesRequested, Note::Required, "deficiency")
        }
        (DossierDirection::Application, A::Approve, S::Submitted | S::InReview) if manager => {
            step(S::Approved, Note::Optional, "decision")
        }
        (DossierDirection::Application, A::Reject, S::Submitted | S::InReview) if manager => {
            step(S::Rejected, Note::Required, "decision")
        }
        // ── A transfer ──
        (DossierDirection::Transfer, A::Send, S::Draft) if manager => Step {
            sends: true,
            ..step(S::Sent, Note::Optional, "note")
        },
        (DossierDirection::Transfer, A::Accept, S::Sent) if pro => {
            step(S::Accepted, Note::Optional, "note")
        }
        (DossierDirection::Transfer, A::Decline, S::Sent) if pro => {
            step(S::Declined, Note::Required, "decision")
        }
        (DossierDirection::Transfer, A::Deliver, S::Accepted | S::RevisionRequested) if pro => {
            Step {
                sends: true,
                needs_files: true,
                ..step(S::Delivered, Note::Optional, "note")
            }
        }
        (DossierDirection::Transfer, A::RequestRevision, S::Delivered) if editor => {
            step(S::RevisionRequested, Note::Required, "deficiency")
        }
        (DossierDirection::Transfer, A::Complete, S::Delivered) if manager => {
            step(S::Completed, Note::Optional, "decision")
        }
        (
            DossierDirection::Transfer,
            A::Cancel,
            S::Sent | S::Accepted | S::Delivered | S::RevisionRequested,
        ) if manager => step(S::Cancelled, Note::Required, "decision"),
        (
            DossierDirection::Transfer,
            A::Assign,
            S::Sent | S::Accepted | S::Delivered | S::RevisionRequested,
        ) if editor => step(status, Note::None, "note"),
        _ => {
            let (message, role) = if closed(status) {
                ("Dosya kapandı; üzerinde adım atılmaz.", false)
            } else if colleague
                && !pro
                && matches!(action, A::Send | A::Withdraw | A::Accept | A::Decline | A::Deliver)
            {
                (
                    "Bu adımı dosyanın sorumlu meslek mensubu ya da büroda proje yöneticisi atar.",
                    true,
                )
            } else if member && !editor {
                (
                    "Bu adım için kurumunuzda en az düzenleyici olmalısınız.",
                    true,
                )
            } else if member
                && !manager
                && matches!(
                    action,
                    A::Approve | A::Reject | A::Send | A::Complete | A::Cancel
                )
            {
                (
                    "Bu kararı kurumunuzda proje yöneticisi ya da yöneticiler verir.",
                    true,
                )
            } else {
                ("Bu adım dosyanın şimdiki durumunda atılmaz.", false)
            };
            return Err(Refusal { message, role });
        }
    };
    Ok(found)
}

pub const ACTIONS: [DossierAction; 13] = [
    DossierAction::Send,
    DossierAction::StartReview,
    DossierAction::Accept,
    DossierAction::Deliver,
    DossierAction::Approve,
    DossierAction::Complete,
    DossierAction::RequestChanges,
    DossierAction::RequestRevision,
    DossierAction::Assign,
    DossierAction::Reject,
    DossierAction::Decline,
    DossierAction::Withdraw,
    DossierAction::Cancel,
];

/// The steps `party` may take now, in the order the page offers them.
pub fn actions(
    direction: DossierDirection,
    status: DossierStatus,
    party: Party,
) -> Vec<DossierAction> {
    ACTIONS
        .into_iter()
        .filter(|a| step_of(direction, status, *a, party).is_ok())
        .collect()
}

pub fn action_code(action: DossierAction) -> &'static str {
    match action {
        DossierAction::Send => "send",
        DossierAction::Withdraw => "withdraw",
        DossierAction::StartReview => "start_review",
        DossierAction::Assign => "assign",
        DossierAction::RequestChanges => "request_changes",
        DossierAction::Approve => "approve",
        DossierAction::Reject => "reject",
        DossierAction::Accept => "accept",
        DossierAction::Decline => "decline",
        DossierAction::Deliver => "deliver",
        DossierAction::RequestRevision => "request_revision",
        DossierAction::Complete => "complete",
        DossierAction::Cancel => "cancel",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use DossierAction as A;
    use DossierStatus as S;

    const APP: DossierDirection = DossierDirection::Application;
    const TRANSFER: DossierDirection = DossierDirection::Transfer;
    const PRO: Party = Party::Professional;
    const VIEWER: Party = Party::Member(TenantRole::Viewer);
    const EDITOR: Party = Party::Member(TenantRole::Editor);
    const MANAGER: Party = Party::Member(TenantRole::ProjectManager);

    #[test]
    fn an_offices_editor_prepares_and_its_manager_sends_for_the_professional() {
        let editor = Party::Colleague(TenantRole::Editor);
        let manager = Party::Colleague(TenantRole::ProjectManager);
        let viewer = Party::Colleague(TenantRole::Viewer);
        assert_eq!(editor.side(), DossierSide::Professional);
        // An editor prepares the package and writes, but sends nothing; the office's manager sends as the
        // professional does.
        assert!(can_add_files(APP, S::Draft, editor) && can_edit(APP, S::Draft, editor));
        assert!(actions(APP, S::Draft, editor).is_empty());
        assert_eq!(actions(APP, S::Draft, manager), [A::Send]);
        let refused = step_of(APP, S::Draft, A::Send, editor).unwrap_err();
        assert!(refused.role && refused.message.contains("sorumlu meslek mensubu"));
        assert_eq!(actions(TRANSFER, S::Sent, manager), [A::Accept, A::Decline]);
        assert!(can_message(S::Submitted, editor) && !can_message(S::Submitted, viewer));
        // A viewer reads; an editor does not delete the draft, a manager may.
        assert!(!can_add_files(APP, S::Draft, viewer) && !can_delete(APP, S::Draft, editor));
        assert!(can_delete(APP, S::Draft, manager));
        // Never the municipality's steps.
        assert!(step_of(APP, S::Submitted, A::Approve, manager).is_err());
    }

    #[test]
    fn an_application_goes_round_by_round_until_it_is_decided() {
        assert_eq!(actions(APP, S::Draft, PRO), [A::Send]);
        let sent = step_of(APP, S::Draft, A::Send, PRO).unwrap();
        assert!(sent.sends && sent.needs_files && sent.to == S::Submitted);
        // The municipality: an editor reviews and asks for changes, a manager decides.
        assert_eq!(
            actions(APP, S::Submitted, EDITOR),
            [A::StartReview, A::RequestChanges, A::Assign]
        );
        assert_eq!(
            actions(APP, S::InReview, MANAGER),
            [A::Approve, A::RequestChanges, A::Assign, A::Reject]
        );
        let changes = step_of(APP, S::InReview, A::RequestChanges, EDITOR).unwrap();
        assert_eq!(
            (changes.to, changes.note, changes.note_kind),
            (S::ChangesRequested, Note::Required, "deficiency")
        );
        // Corrected and sent again; withdrawn at any point before the decision.
        assert_eq!(
            actions(APP, S::ChangesRequested, PRO),
            [A::Send, A::Withdraw]
        );
        assert_eq!(actions(APP, S::InReview, PRO), [A::Withdraw]);
        assert!(step_of(APP, S::InReview, A::Reject, MANAGER).unwrap().note == Note::Required);
        // Decided: nothing more, and said so.
        assert!(
            actions(APP, S::Approved, MANAGER).is_empty()
                && actions(APP, S::Approved, PRO).is_empty()
        );
        assert_eq!(
            step_of(APP, S::Approved, A::Withdraw, PRO)
                .unwrap_err()
                .message,
            "Dosya kapandı; üzerinde adım atılmaz."
        );
    }

    #[test]
    fn a_seat_reads_an_editor_works_a_manager_decides() {
        assert!(actions(APP, S::Submitted, VIEWER).is_empty());
        let refused = step_of(APP, S::Submitted, A::StartReview, VIEWER).unwrap_err();
        assert!(refused.role && refused.message.contains("düzenleyici"));
        assert!(
            step_of(APP, S::InReview, A::Approve, EDITOR)
                .unwrap_err()
                .message
                .contains("proje yöneticisi")
        );
        assert!(
            !can_message(S::Submitted, VIEWER)
                && can_message(S::Submitted, EDITOR)
                && can_message(S::Submitted, PRO)
        );
        // The professional never takes the municipality's steps, nor the municipality the professional's.
        assert!(step_of(APP, S::Submitted, A::Approve, PRO).is_err());
        assert!(step_of(APP, S::Draft, A::Send, MANAGER).is_err());
    }

    #[test]
    fn a_transfer_is_taken_on_delivered_and_completed() {
        assert_eq!(actions(TRANSFER, S::Draft, MANAGER), [A::Send]);
        assert!(actions(TRANSFER, S::Draft, EDITOR).is_empty());
        let sent = step_of(TRANSFER, S::Draft, A::Send, MANAGER).unwrap();
        assert!(sent.sends && !sent.needs_files);
        assert_eq!(actions(TRANSFER, S::Sent, PRO), [A::Accept, A::Decline]);
        assert_eq!(actions(TRANSFER, S::Accepted, PRO), [A::Deliver]);
        assert!(
            step_of(TRANSFER, S::Accepted, A::Deliver, PRO)
                .unwrap()
                .needs_files
        );
        assert_eq!(
            actions(TRANSFER, S::Delivered, MANAGER),
            [A::Complete, A::RequestRevision, A::Assign, A::Cancel]
        );
        assert_eq!(actions(TRANSFER, S::RevisionRequested, PRO), [A::Deliver]);
        assert!(actions(TRANSFER, S::Completed, MANAGER).is_empty());
    }

    #[test]
    fn files_and_edits_belong_to_the_side_preparing_the_package() {
        assert!(can_add_files(APP, S::Draft, PRO) && !can_add_files(APP, S::Draft, MANAGER));
        assert!(!can_add_files(APP, S::Submitted, PRO));
        assert!(
            can_add_files(TRANSFER, S::Draft, MANAGER)
                && !can_add_files(TRANSFER, S::Draft, EDITOR)
        );
        assert!(can_add_files(TRANSFER, S::RevisionRequested, PRO));
        assert!(can_edit(APP, S::ChangesRequested, PRO) && !can_edit(APP, S::Submitted, PRO));
        assert!(can_delete(APP, S::Draft, PRO) && !can_delete(APP, S::ChangesRequested, PRO));
        assert!(can_delete(TRANSFER, S::Draft, MANAGER) && !can_delete(TRANSFER, S::Sent, MANAGER));
        assert_eq!(
            waiting_for(TRANSFER, S::Delivered),
            Some(DossierSide::Municipality)
        );
        assert_eq!(
            waiting_for(APP, S::ChangesRequested),
            Some(DossierSide::Professional)
        );
        assert_eq!(waiting_for(APP, S::Rejected), None);
    }
}
