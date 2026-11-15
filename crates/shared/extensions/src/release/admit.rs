//! Who may move a release, how and from where (plan/eklenti.md ADIM 22A; PY-CLOUD-07, 09, 18): the permission, the
//! ownership or the independence a move takes, a person for a decision, then the state. The first that fails is the
//! answer, a [`Refusal`] that says what would make the move possible.

use super::{
    Action, Actor, CatalogScope, Channel, Permission, Refusal, Release, ReleaseState, Role,
};

impl Release {
    /// Who may, how, and from where: permission, ownership or independence, a person when it is a decision, then the
    /// state. The first that fails is the answer.
    pub(super) fn admit(&self, actor: &Actor, action: Action) -> Result<(), Refusal> {
        match action {
            Action::Upload | Action::Replace | Action::Submit | Action::Publish => {
                need(actor, action.permission(), action)?;
                self.owner(actor, action)?;
            }
            Action::Scan => {
                need(actor, Permission::Scan, action)?;
                if self.owned_by(actor) {
                    return Err(Refusal::new(
                        "self_scan",
                        "Yazar ve yayımcı kendi sürümünün denetim sonucunu bildiremez; sonuçları kataloğun denetim \
                         hizmeti bildirir.",
                    ));
                }
            }
            Action::StartReview | Action::RequestChanges | Action::Waive | Action::Approve => {
                need(actor, action.permission(), action)?;
                self.independent(actor)?;
            }
            Action::Suspend => need(actor, Permission::Moderate, action)?,
            Action::Reinstate => {
                need(actor, Permission::Moderate, action)?;
                self.independent(actor)?;
            }
            Action::Withdraw | Action::Archive => {
                if !actor.can(Permission::Moderate) {
                    if !actor.can(Permission::Publish) {
                        return Err(Refusal::new(
                            "permission_missing",
                            format!(
                                "“{}” için “{}” yetkisi ya da kendi yayımcınızın sürümünde “{}” yetkisi gerekir.",
                                action.title(),
                                Permission::Moderate.title(),
                                Permission::Publish.title()
                            ),
                        ));
                    }
                    self.owner(actor, action)?;
                }
            }
        }
        if action.needs_person() && actor.via != Channel::Window {
            return Err(Refusal::new(
                "decision_by_program",
                format!(
                    "“{}” bir kişinin kararıdır ve katalog penceresinden verilir; programlı çağrı (Python, MCP, ajan) bu \
                     kararı veremez, hangi hesabın jetonunu taşırsa taşısın.",
                    action.title()
                ),
            ));
        }
        if !action.sources().contains(&self.state) {
            return Err(self.wrong_state(action));
        }
        Ok(())
    }

    fn owner(&self, actor: &Actor, action: Action) -> Result<(), Refusal> {
        if self.owned_by(actor) {
            return Ok(());
        }
        Err(Refusal::new(
            "not_owner",
            format!(
                "{} {} “{}” yayımcısının; “{}” yalnız yazarı ve yayımcının üyeleri yapabilir.",
                self.extension_id,
                self.version,
                self.publisher,
                action.title()
            ),
        ))
    }

    /// The four eyes (PY-CLOUD-07): outside one's personal catalog the author never reviews, approves or reinstates
    /// their own release; in the public catalog no member of its publisher does either.
    fn independent(&self, actor: &Actor) -> Result<(), Refusal> {
        match self.scope {
            CatalogScope::Private => Ok(()),
            _ if actor.id == self.author => Err(Refusal::new(
                "self_review",
                "Yazar kendi sürümünü inceleyemez, onaylayamaz ve askıdan alamaz; bunu başka bir inceleyici yapar.",
            )),
            CatalogScope::Public if actor.publishers.contains(&self.publisher) => {
                Err(Refusal::new(
                    "self_review",
                    format!(
                        "Genel katalogda “{}” yayımcısının üyeleri kendi yayımcılarının sürümünü inceleyemez ve \
                         onaylayamaz; bağımsız bir inceleyici gerekir.",
                        self.publisher
                    ),
                ))
            }
            _ => Ok(()),
        }
    }

    fn wrong_state(&self, action: Action) -> Refusal {
        let state = self.state.title();
        let unapproved = matches!(
            self.state,
            ReleaseState::Draft
                | ReleaseState::Submitted
                | ReleaseState::InReview
                | ReleaseState::ChangesRequested
        );
        match action {
            // What a program that tries to skip review hears (PY-CLOUD-18).
            Action::Publish if unapproved => Refusal::new(
                "not_approved",
                format!(
                    "Onaylanmamış sürüm yayımlanamaz; sürüm şimdi “{state}”. Yayın için sürüm incelemeye gönderilir, \
                     kendiliğinden denetimleri doğrulanır ve bir inceleyici açıkça onaylar."
                ),
            ),
            Action::Replace => Refusal::new(
                "content_frozen",
                format!(
                    "Sürüm “{state}”: gönderilen sürümün içeriği değişmez. Yeni içerik yeni bir sürüm numarasıyla \
                     yüklenir; inceleyici değişiklik ister ya da bir denetim engellerse aynı sürüme yeni içerik \
                     yüklenebilir."
                ),
            ),
            Action::Scan => Refusal::new(
                "gates_closed",
                format!(
                    "Sürüm “{state}”: denetim sonuçları artık değişmez. Tarama sonucu yalnız taslağa, gönderilmiş, \
                     incelenen ya da geri gönderilmiş sürüme yazılır; onaydan sonra bulunan sorun askıya almayla \
                     ele alınır."
                ),
            ),
            _ => {
                let next: Vec<&str> = self.state.actions().iter().map(|a| a.title()).collect();
                let way = if next.is_empty() {
                    "Bu durumdan çıkış yok.".to_owned()
                } else {
                    format!("Bu durumda yapılabilecekler: {}.", next.join(", "))
                };
                Refusal::new(
                    "wrong_state",
                    format!(
                        "Sürüm “{state}”: “{}” bu durumda yapılmaz. {way}",
                        action.title()
                    ),
                )
            }
        }
    }
}

/// The permission a move takes, or the refusal that names it and the roles that hold it.
pub(super) fn need(actor: &Actor, permission: Permission, action: Action) -> Result<(), Refusal> {
    if actor.can(permission) {
        return Ok(());
    }
    Err(Refusal::new(
        "permission_missing",
        format!(
            "“{}” için “{}” yetkisi gerekir; bu yetki {} rolündedir.",
            action.title(),
            permission.title(),
            Role::holding(permission)
        ),
    ))
}
