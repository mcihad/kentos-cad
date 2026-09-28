//! `kentos._native`: the headless command host (`kentos-headless`,
//! docs/adr/0130) as a CPython extension under the Python SDK `kentos.cad`
//! (docs/adr/0131).
//!
//! Deliberately thin. JSON text crosses in both directions, in the catalog's
//! wire form; the typed Python layer (`python/kentos/cad`, generated from the
//! catalog) writes and reads it. A failure of the host is
//! `HostError(code, message)`; a command's own refusal is its answer.
//!
//! The GIL is released while Rust works (a file read or written, a command
//! run), so other Python threads go on. A second call on the same drawing in
//! the meantime is refused by PyO3's borrow check, not queued.
#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

use std::path::PathBuf;

use kentos_contracts::Bounds;
use kentos_headless::{HeadlessError, NewDrawing, Op, Session};
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use serde_json::Value;

create_exception!(
    _native,
    HostError,
    PyException,
    "The host could not do what it was asked; args: (code, message)."
);

fn host(e: HeadlessError) -> PyErr {
    HostError::new_err((e.code, e.message))
}

fn text<T: serde::Serialize + ?Sized>(value: &T) -> PyResult<String> {
    serde_json::to_string(value).map_err(|e| {
        host(HeadlessError::new(
            "unwritable_result",
            format!("sonuç JSON'a yazılamadı: {e}"),
        ))
    })
}

/// A contract enum from its wire name (`hybrid`, `barlow`).
fn named<T: serde::de::DeserializeOwned>(value: &str, what: &str) -> PyResult<T> {
    serde_json::from_value(Value::String(value.to_owned())).map_err(|_| {
        host(HeadlessError::new(
            "invalid_input",
            format!("“{value}” bir {what} değil."),
        ))
    })
}

/// A drawing open in this process (`kentos.cad.Document` wraps it).
#[pyclass(module = "kentos._native", name = "Session")]
struct PySession {
    inner: Session,
}

#[pymethods]
impl PySession {
    /// A new project: the standard layer tree, the zone's work area, the
    /// default units (as Yeni proje makes it).
    #[staticmethod]
    fn new_project(
        py: Python<'_>,
        name: String,
        srid: u32,
        plot_scale: f64,
        workspace: &str,
        drawing_font: &str,
    ) -> PyResult<Self> {
        let choices = NewDrawing {
            name,
            srid,
            plot_scale,
            workspace: named(workspace, "çalışma modu (hybrid, cad, gis …)")?,
            drawing_font: named(drawing_font, "çizim yazı tipi (barlow, arimo …)")?,
        };
        py.detach(|| Session::new(&choices))
            .map(|inner| Self { inner })
            .map_err(host)
    }

    /// A `.kcad` file (v2, or v1 JSON).
    #[staticmethod]
    fn open(py: Python<'_>, path: PathBuf) -> PyResult<Self> {
        py.detach(|| Session::open(&path))
            .map(|inner| Self { inner })
            .map_err(host)
    }

    /// A drawing from a `.kcad` file's bytes (copied once: Python's buffer
    /// is not read while the GIL is released).
    #[staticmethod]
    fn from_bytes(py: Python<'_>, data: &[u8]) -> PyResult<Self> {
        let data = data.to_vec();
        py.detach(|| Session::from_bytes(&data))
            .map(|inner| Self { inner })
            .map_err(host)
    }

    /// Runs a catalog command: `op` is `validate`, `plan` or `execute`;
    /// `input` and the answer (`CommandResult`) are JSON text.
    #[pyo3(signature = (command, op, input, version = None))]
    fn run(
        &mut self,
        py: Python<'_>,
        command: &str,
        op: &str,
        input: &str,
        version: Option<u32>,
    ) -> PyResult<String> {
        let op = Op::parse(op).map_err(host)?;
        let input: Value = serde_json::from_str(input).map_err(|e| {
            host(HeadlessError::new(
                "invalid_input",
                format!("{command} girdisi JSON değil: {e}"),
            ))
        })?;
        let inner = &mut self.inner;
        let answer = py
            .detach(|| inner.run(command, version, op, input))
            .map_err(host)?;
        text(&answer)
    }

    /// Saves as `.kcad` v2 to `path`, or where it was read from; the answer
    /// (`path`, `bytes`, `revision`) as JSON text.
    #[pyo3(signature = (path = None))]
    fn save(&mut self, py: Python<'_>, path: Option<PathBuf>) -> PyResult<String> {
        let inner = &mut self.inner;
        let saved = py.detach(|| inner.save(path.as_deref())).map_err(host)?;
        text(&saved)
    }

    /// The `.kcad` v2 bytes of the drawing as it is now, verified.
    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let inner = &self.inner;
        let bytes = py.detach(|| inner.to_bytes()).map_err(host)?;
        Ok(PyBytes::new(py, &bytes))
    }

    fn summary(&self) -> PyResult<String> {
        text(&self.inner.summary())
    }

    fn layers(&self) -> PyResult<String> {
        text(self.inner.layers())
    }

    /// A page of objects: `bbox` is `(min_x, min_y, max_x, max_y)`, `after`
    /// the cursor a page gave.
    #[pyo3(signature = (layer = None, kinds = None, bbox = None, after = None, limit = None))]
    fn entities(
        &self,
        layer: Option<String>,
        kinds: Option<Vec<String>>,
        bbox: Option<(f64, f64, f64, f64)>,
        after: Option<String>,
        limit: Option<usize>,
    ) -> PyResult<String> {
        let bbox = bbox.map(|(min_x, min_y, max_x, max_y)| Bounds {
            min_x,
            min_y,
            max_x,
            max_y,
        });
        let page = self
            .inner
            .entities(layer, kinds, bbox, after, limit)
            .map_err(host)?;
        text(&page)
    }

    fn entity(&self, uid: &str) -> PyResult<String> {
        text(&self.inner.entity(uid).map_err(host)?)
    }

    fn measure(&self, uid: &str) -> PyResult<String> {
        text(&self.inner.measure(uid).map_err(host)?)
    }

    /// Undoes the last step; its name, or None.
    fn undo(&mut self) -> Option<String> {
        self.inner.undo()
    }

    /// Redoes the last undone step; its name, or None.
    fn redo(&mut self) -> Option<String> {
        self.inner.redo()
    }

    fn begin_group(&mut self, label: &str) -> PyResult<()> {
        self.inner.begin_group(label).map_err(host)
    }

    fn end_group(&mut self) -> bool {
        self.inner.end_group()
    }

    fn cancel_group(&mut self) -> bool {
        self.inner.cancel_group()
    }

    fn __repr__(&self) -> String {
        let s = self.inner.summary();
        format!(
            "<kentos._native.Session {:?}, {} nesne, revizyon {}>",
            s["name"].as_str().unwrap_or(""),
            s["objects"],
            s["revision"].as_str().unwrap_or("")
        )
    }
}

/// The command catalog as JSON text (every command, its schemas and examples).
#[pyfunction]
fn catalog() -> PyResult<String> {
    text(&kentos_headless::catalog())
}

/// The commands this host runs, as `(id, version)` pairs.
#[pyfunction]
fn local_commands() -> Vec<(&'static str, u32)> {
    kentos_headless::DESKTOP.to_vec()
}

#[pymodule(name = "_native")]
mod native {
    #[pymodule_export]
    use super::{PySession, catalog, local_commands};
    use pyo3::prelude::*;

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add("HostError", m.py().get_type::<super::HostError>())?;
        m.add("__version__", env!("CARGO_PKG_VERSION"))?;
        Ok(())
    }
}
