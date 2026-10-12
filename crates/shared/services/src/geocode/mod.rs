//! Adres arama's online providers (docs/adr/0229 §10): the ready ones and a
//! device's settings of them (`providers`), the requests a host sends
//! (`request`), their answers read as candidates (`read`) and scored by the
//! locator's own rules (`score`), and a batch's addresses made unique
//! (`batch`). A host fetches; nothing here reaches for the network. The
//! independent reference is `scripts/fixtures/geocode_online_cases.py`
//! (fixtures/geocode/v1/online.json).

pub mod batch;
pub mod providers;
pub mod read;
pub mod request;
pub mod score;

pub use providers::{Provider, ProviderKind, Providers, StoreRule};
pub use read::OnlineCandidate;
