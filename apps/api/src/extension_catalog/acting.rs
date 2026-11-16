//! A move of the flow on a stored release (docs/adr/8059 §10, §13): made on the record it was read from and written
//! over that record only, made again on the newer record when another came first; a person's decision only with a
//! fresh sign-in; a release that becomes published handed to the server's hook.

use kentos_extensions::release::{
    Actor, Channel, Refusal, Release, ReleaseFacts, ReleaseState, Timestamp,
};

use super::model::{changed, fresh_sign_in_required};
use super::store::StoreError;
use super::{Account, Catalog, CatalogError, CatalogId, Move, ReleaseKey, now};

/// How many times a move is made again on the newer record when another move was written first.
const ATTEMPTS: usize = 4;

/// What a review starts with (D12): the release's package, the version approved before it with its package, and
/// whether the catalog trusts the publisher.
struct ReviewInputs {
    current: ReleaseFacts,
    baseline: Option<(Release, ReleaseFacts)>,
    trusted: bool,
}

impl Catalog {
    /// A move of the flow. A person's decision without a fresh sign-in is tried first and refused only when the flow
    /// would have allowed it: the flow's own refusal says more.
    pub async fn act(
        &self,
        key: &ReleaseKey,
        account: &Account,
        request: &Move,
    ) -> Result<(Release, Actor), CatalogError> {
        let actor = self.actor(&key.catalog, account).await?;
        let review = match request {
            Move::StartReview => Some(self.review_inputs(key, &actor).await?),
            _ => None,
        };
        let now = now();
        let action = request.action();
        if action.needs_person() && account.via == Channel::Window && !account.fresh_sign_in {
            let mut trial = self.visible(key, &actor).await?;
            apply(&mut trial, &actor, request, review.as_ref(), now)
                .map_err(CatalogError::Refused)?;
            return Err(CatalogError::Refused(fresh_sign_in_required(action)));
        }
        let mut before = None;
        let release = self
            .moved(key, &actor, |r| {
                before = Some(r.state());
                apply(r, &actor, request, review.as_ref(), now)
            })
            .await?;
        if release.state() == ReleaseState::Published && before != Some(ReleaseState::Published) {
            self.published(&key.catalog, &release).await;
        }
        Ok((release, actor))
    }

    /// Hands a release that became published to the server's hook, with its approved bytes. The move is kept already:
    /// a hook that cannot be given the bytes is logged, not the request's failure.
    async fn published(&self, catalog: &CatalogId, release: &Release) {
        let Some(hook) = &self.on_publish else {
            return;
        };
        match self.stored(&release.content().package).await {
            Ok(bytes) => hook.published(catalog, release, bytes),
            Err(e) => tracing::warn!(
                eklenti = release.extension_id(),
                surum = release.version(),
                hata = ?e,
                "yayımlanan sürümün paketi okunamadı; denetim ortamı hazırlanmadı"
            ),
        }
    }

    /// Makes a move on the stored record and writes it over that record only; when another move came first, the move
    /// is made again on the newer record, where the flow checks it anew.
    pub(super) async fn moved<F>(
        &self,
        key: &ReleaseKey,
        actor: &Actor,
        mut change: F,
    ) -> Result<Release, CatalogError>
    where
        F: FnMut(&mut Release) -> Result<(), Refusal> + Send,
    {
        for _ in 0..ATTEMPTS {
            let mut release = self.visible(key, actor).await?;
            let moves = release.audit().len();
            change(&mut release).map_err(CatalogError::Refused)?;
            match self.store.write(key, Some(moves), &release).await {
                Ok(()) => return Ok(release),
                Err(StoreError::Changed) => continue,
                Err(e) => return Err(e.into()),
            }
        }
        Err(CatalogError::Refused(changed()))
    }

    async fn review_inputs(
        &self,
        key: &ReleaseKey,
        actor: &Actor,
    ) -> Result<ReviewInputs, CatalogError> {
        let release = self.visible(key, actor).await?;
        let current = self.stored_facts(&release.content().package).await?;
        let all = self.store.releases(&key.catalog).await?;
        let baseline = match release.baseline(&all) {
            Some(before) => {
                let facts = self.stored_facts(&before.content().package).await?;
                Some((before.clone(), facts))
            }
            None => None,
        };
        let trusted = self
            .store
            .trusted(&key.catalog, release.publisher())
            .await?;
        Ok(ReviewInputs {
            current,
            baseline,
            trusted,
        })
    }
}

/// The move a request asks for, made on a release.
fn apply(
    release: &mut Release,
    actor: &Actor,
    request: &Move,
    review: Option<&ReviewInputs>,
    now: Timestamp,
) -> Result<(), Refusal> {
    match request {
        Move::Submit { note } => release.submit(actor, note.as_deref(), now),
        Move::Scan {
            gate,
            content,
            outcome,
        } => release.record_scan(actor, *gate, content, outcome.clone(), now),
        Move::StartReview => match review {
            Some(inputs) => {
                let baseline = inputs
                    .baseline
                    .as_ref()
                    .map(|(before, facts)| (before, facts));
                release.start_review(actor, &inputs.current, baseline, inputs.trusted, now)
            }
            None => Err(Refusal::new(
                "package_mismatch",
                "İncelemenin paketleri okunamadı; isteği yeniden gönderin.",
            )),
        },
        Move::RequestChanges { changes } => release.request_changes(actor, changes, now),
        Move::Waive { gate, reason } => release.waive(actor, *gate, reason, now),
        Move::Approve { note } => release.approve(actor, note.as_deref(), now),
        Move::Publish => release.publish(actor, now),
        Move::Suspend { reason } => release.suspend(actor, reason, now),
        Move::Reinstate { reason } => release.reinstate(actor, reason, now),
        Move::Withdraw { reason } => release.withdraw(actor, reason, now),
        Move::Archive { reason } => release.archive(actor, reason.as_deref(), now),
    }
}
