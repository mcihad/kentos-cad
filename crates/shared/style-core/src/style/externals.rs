//! The extensions' functions a layer's style may call (`ext.<eklenti>.<işlev>(…)`, docs/adr/8057 §8): in a rule's
//! condition, a category's or a class's value, a data-defined symbol. The host registers which functions there are and
//! what answers them; a drawing never waits: the host's answers are the ones it already has, an answer it does not
//! have yet is empty and asked in the background, and when answers come it builds the layers calling them again
//! ([`answers_generation`]). Without a host (the web, a headless run) there are none and such a call does not compile.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use crate::expr::external::{ExternalCalls, ExternalDef};
use crate::expr::{CompileError, Expr, Schema, Unit, compile_with};

/// What answers the calls: the host's, never waiting.
pub type Calls = Arc<dyn ExternalCalls + Send + Sync>;

static EXTERNALS: RwLock<Option<(Vec<ExternalDef>, Calls)>> = RwLock::new(None);

/// How many times the host's answers grew.
static ANSWERS: AtomicU64 = AtomicU64::new(0);

/// The functions a style may call and what answers them; replaced as the extensions change.
pub fn set_externals(defs: Vec<ExternalDef>, calls: Calls) {
    if let Ok(mut slot) = EXTERNALS.write() {
        *slot = Some((defs, calls));
    }
}

/// The host got answers it did not have: the layers whose styles call extensions' functions are built again.
pub fn answers_changed() {
    ANSWERS.fetch_add(1, Ordering::Relaxed);
}

/// How many times the host's answers grew.
pub fn answers_generation() -> u64 {
    ANSWERS.load(Ordering::Relaxed)
}

/// The functions registered; none without a host.
pub fn defs() -> Vec<ExternalDef> {
    EXTERNALS
        .read()
        .ok()
        .and_then(|e| e.as_ref().map(|(d, _)| d.clone()))
        .unwrap_or_default()
}

/// What answers them; none without a host.
pub fn calls() -> Option<Calls> {
    EXTERNALS
        .read()
        .ok()
        .and_then(|e| e.as_ref().map(|(_, c)| Arc::clone(c)))
}

/// `source` compiled with the registered functions known, its geometry values in `unit`.
pub fn compile_in(source: &str, unit: Unit) -> Result<Expr, CompileError> {
    let schema = Schema {
        unit,
        externals: defs(),
        ..Schema::default()
    };
    compile_with(source, &schema)
}

/// Whether a style (its JSON's text) may call an extension's function: the layers built again when answers come.
pub fn mentions_externals(style_text: &str) -> bool {
    style_text.contains("ext.")
}

/// A scope with the host's answers added: everything else is `inner`'s.
pub struct Calling<'s> {
    pub inner: &'s dyn crate::expr::Scope,
    pub calls: &'s dyn ExternalCalls,
}

impl crate::expr::Scope for Calling<'_> {
    fn field(&self, i: usize) -> Option<&str> {
        self.inner.field(i)
    }

    fn measured(&self) -> crate::expr::Measured {
        self.inner.measured()
    }

    fn vertices(&self) -> Option<f64> {
        self.inner.vertices()
    }

    fn kind(&self) -> &str {
        self.inner.kind()
    }

    fn layer(&self) -> &str {
        self.inner.layer()
    }

    fn label(&self) -> Option<&str> {
        self.inner.label()
    }

    fn index(&self) -> f64 {
        self.inner.index()
    }

    fn id(&self) -> f64 {
        self.inner.id()
    }

    fn scale(&self) -> Option<f64> {
        self.inner.scale()
    }

    fn local(&self, slot: usize) -> crate::expr::Value<'_> {
        self.inner.local(slot)
    }

    fn world(&self) -> Option<&dyn crate::expr::world::WorldCalls> {
        self.inner.world()
    }

    fn externals(&self) -> Option<&dyn ExternalCalls> {
        Some(self.calls)
    }

    fn geometry(&self, what: crate::expr::Geometry) -> Option<f64> {
        self.inner.geometry(what)
    }
}
