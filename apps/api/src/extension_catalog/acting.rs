//! A move of the flow on a stored release (docs/adr/8059 §10, §13): made on the record it was read from and written
//! over that record only, made again on the newer record when another came first; a person's decision only with a
//! fresh sign-in; a release that becomes published handed to the server's hook.

use kentos_extensions::release::{
    Actor, Channel, GateKind, Reason, Refusal, Release, ReleaseFacts, ReleaseState, Role,
    ScanOutcome, Timestamp,
};

use super::model::{changed, fresh_sign_in_required};
use super::store::StoreError;
use super::{Account, Catalog, CatalogError, CatalogId, Move, ReleaseKey, now};

/// How many times a move is made again on the newer record when another move was written first.
const ATTEMPTS: usize = 4;

/// The catalog itself when it reports a gate's rule it runs (docs/adr/8059 §14): a scanner that is no account.
const CATALOG_SCANNER: &str = "kentos-katalog";

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
        let mut release = self
            .moved(key, &actor, |r| {
                before = Some(r.state());
                apply(r, &actor, request, review.as_ref(), now)
            })
            .await?;
        if matches!(request, Move::Submit { .. }) {
            release = self.imitation_gate(key, release).await?;
            release = self.secrets_gate(key, release).await?;
            release = self.experimental_gate(key, release).await?;
            release = self.build_gate(key, release).await?;
        }
        if release.state() == ReleaseState::Published && before != Some(ReleaseState::Published) {
            self.published(&key.catalog, &release).await;
        }
        Ok((release, actor))
    }

    /// The publisher gate's imitation rule, run by the catalog itself when a version is sent (docs/adr/8059 §14, D13):
    /// a release whose name, id or icon reads as another publisher's listed extension is blocked at once and goes back
    /// to its author with the findings, before a scanner or a reviewer spends time on it. Finding none proves nothing
    /// of the gate (whether the publisher owns the id's namespace is its scanner's), so nothing is reported then.
    async fn imitation_gate(
        &self,
        key: &ReleaseKey,
        release: Release,
    ) -> Result<Release, CatalogError> {
        let all = self.store.releases(&key.catalog).await?;
        let found = release.imitations(&all);
        if found.is_empty() {
            return Ok(release);
        }
        let scanner = Actor::new(CATALOG_SCANNER, &[Role::Scanner], Channel::Program);
        let content = release.content().package.clone();
        let now = now();
        self.moved(key, &scanner, |r| {
            r.record_scan(
                &scanner,
                GateKind::Publisher,
                &content,
                ScanOutcome::Blocked(found.clone()),
                now,
            )
        })
        .await
    }

    /// The secret gate's rule the catalog runs itself when a version is sent (PY-CLOUD-05, D11;
    /// `kentos_extensions::secrets`): a key written in the package's text, a provider's key by its shape or a private
    /// key, sends the release back to its author with each file and line, never the value. Finding none proves nothing
    /// of the gate (its scanner keeps it), so nothing is reported then; a release another gate sent back is left.
    async fn secrets_gate(
        &self,
        key: &ReleaseKey,
        release: Release,
    ) -> Result<Release, CatalogError> {
        if release.state() != ReleaseState::Submitted {
            return Ok(release);
        }
        let content = release.content().package.clone();
        let bytes = self.stored(&content).await?;
        let opened = kentos_extensions::open(&bytes, &self.limits);
        let found: Vec<Reason> = kentos_extensions::secrets::gate_findings(&opened.files)
            .into_iter()
            .map(|(file, line, message)| Reason {
                file: Some(file),
                at: Some(format!("satır {line}")),
                ..Reason::new("secret_found", message)
            })
            .collect();
        if found.is_empty() {
            return Ok(release);
        }
        let scanner = Actor::new(CATALOG_SCANNER, &[Role::Scanner], Channel::Program);
        let now = now();
        self.moved(key, &scanner, |r| {
            r.record_scan(
                &scanner,
                GateKind::Secrets,
                &content,
                ScanOutcome::Blocked(found.clone()),
                now,
            )
        })
        .await
    }

    /// The package gate's rule for the SDK's experimental names, run by the catalog itself when a version is sent to a
    /// public or tenant catalog (ADR 8061 §5): a package whose Python uses one goes back with each file and line; a
    /// personal catalog publishes it, as the workshop and a local install run it. Finding none proves nothing of the
    /// gate, so nothing is reported then; a release another gate sent back is left.
    async fn experimental_gate(
        &self,
        key: &ReleaseKey,
        release: Release,
    ) -> Result<Release, CatalogError> {
        if release.state() != ReleaseState::Submitted
            || matches!(key.catalog, CatalogId::Personal(_))
            || self.experimental.is_empty()
        {
            return Ok(release);
        }
        let content = release.content().package.clone();
        let bytes = self.stored(&content).await?;
        let opened = kentos_extensions::open(&bytes, &self.limits);
        let found: Vec<Reason> =
            kentos_extensions::sdk_surface::experimental_in(&opened.files, self.experimental)
                .into_iter()
                .map(|p| Reason {
                    file: p.file,
                    at: p.at,
                    ..Reason::new(p.code, p.message)
                })
                .collect();
        if found.is_empty() {
            return Ok(release);
        }
        let scanner = Actor::new(CATALOG_SCANNER, &[Role::Scanner], Channel::Program);
        let now = now();
        self.moved(key, &scanner, |r| {
            r.record_scan(
                &scanner,
                GateKind::Package,
                &content,
                ScanOutcome::Blocked(found.clone()),
                now,
            )
        })
        .await
    }

    /// The dependency gate's rule for builds, run by the catalog itself when a version is sent to the public catalog
    /// (docs/adr/8057 §94; plan/eklenti.md D3, PY-UV-21): a package its lock builds from its source, or does not
    /// install, on a computer the release says it runs on sends it back with each package and computer. The gate
    /// allows judgement: a reviewer may lift it with a reason (a build the publisher stands behind, a computer nobody
    /// uses). A tenant's or a person's catalog publishes it, each desktop's install review saying the build. Finding
    /// none proves nothing of the gate, so nothing is reported then; a release another gate sent back is left.
    async fn build_gate(
        &self,
        key: &ReleaseKey,
        release: Release,
    ) -> Result<Release, CatalogError> {
        if release.state() != ReleaseState::Submitted || !matches!(key.catalog, CatalogId::Public) {
            return Ok(release);
        }
        let content = release.content().package.clone();
        let bytes = self.stored(&content).await?;
        let opened = kentos_extensions::open(&bytes, &self.limits);
        let Some(manifest) = opened.inspection.manifest.as_ref() else {
            return Ok(release);
        };
        let lock = manifest.python.as_ref().map(|p| p.lock_file.clone());
        let found: Vec<Reason> =
            kentos_extensions::python_builds::of_package(manifest, &opened.files)
                .iter()
                .flat_map(|u| {
                    kentos_extensions::python_builds::said(u)
                        .into_iter()
                        .map(|(code, message)| Reason {
                            file: lock.clone(),
                            dependency: Some(u.name.clone()),
                            ..Reason::new(code, message)
                        })
                })
                .collect();
        if found.is_empty() {
            return Ok(release);
        }
        let scanner = Actor::new(CATALOG_SCANNER, &[Role::Scanner], Channel::Program);
        let now = now();
        self.moved(key, &scanner, |r| {
            r.record_scan(
                &scanner,
                GateKind::Dependencies,
                &content,
                ScanOutcome::Blocked(found.clone()),
                now,
            )
        })
        .await
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
