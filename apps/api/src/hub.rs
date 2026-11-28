//! "Project changed" signals inside this process. They carry no data: each
//! subscriber reads its events from the outbox with its own access scope,
//! so the database stays the one source of truth and every push is
//! authorized per connection. Commits made elsewhere (another server
//! process, the CLI) are picked up by the subscribers' periodic check.

use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Clone)]
pub struct Hub {
    tx: broadcast::Sender<(Uuid, Uuid)>,
}

impl Default for Hub {
    fn default() -> Self {
        Self {
            tx: broadcast::channel(1024).0,
        }
    }
}

impl Hub {
    /// A commit landed in `(tenant, project)`.
    pub fn notify(&self, tenant: Uuid, project: Uuid) {
        let _ = self.tx.send((tenant, project));
    }

    pub fn subscribe(&self) -> broadcast::Receiver<(Uuid, Uuid)> {
        self.tx.subscribe()
    }
}

/// "A notification of this person's was written or read" (docs/adr/9000 §23): the database signals it on
/// `kentos_notification` (migration 0052), so a notification written by any server process, or by a definer function
/// of the database itself, reaches the person's open pages; `listen` relays the signals here. Like the project's,
/// the signal is an id alone: each stream reads the count with its own person's rights.
#[derive(Clone)]
pub struct Notices {
    tx: broadcast::Sender<Uuid>,
}

impl Default for Notices {
    fn default() -> Self {
        Self {
            tx: broadcast::channel(1024).0,
        }
    }
}

impl Notices {
    pub fn subscribe(&self) -> broadcast::Receiver<Uuid> {
        self.tx.subscribe()
    }

    pub fn send(&self, user: Uuid) {
        let _ = self.tx.send(user);
    }
}

/// Listens on `kentos_notification` for as long as the server runs; a lost connection is opened again after a pause
/// (the pages fall back to asking every few minutes meanwhile).
pub async fn listen(db: kentos_postgres::Db, notices: Notices) {
    loop {
        match sqlx::postgres::PgListener::connect_with(&db.pool).await {
            Ok(mut listener) => {
                if let Err(e) = listener.listen("kentos_notification").await {
                    tracing::warn!(error = ?e, "bildirim kanalı dinlenemedi");
                } else {
                    loop {
                        match listener.recv().await {
                            Ok(n) => {
                                if let Ok(user) = n.payload().parse::<Uuid>() {
                                    notices.send(user);
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = ?e, "bildirim kanalı koptu; yeniden bağlanılacak");
                                break;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = ?e, "bildirim kanalına bağlanılamadı; yeniden denenecek")
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
