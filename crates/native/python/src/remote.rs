//! The Python module's network (docs/adr/0645 §5): the extension carries no HTTP client (the dependency rule keeps
//! `reqwest` out of it), so an addressed file is read through Python's own — `kentos._remote.Http`, urllib with HTTP
//! ranges — registered once when `kentos` is imported. A command runs with the GIL released; a read takes it back
//! for the call only.

use kentos_services::remote::{RangeRead, Remote};
use pyo3::prelude::*;

/// The registered Python reader: `size(url) -> (int, str | None)`, `read(url, offset, length, version) -> bytes`,
/// `object(url) -> bytes | None`.
struct PyRemote(Py<PyAny>);

/// An address opened by the Python reader.
struct PyOpened {
    reader: Py<PyAny>,
    url: String,
    size: u64,
    version: Option<String>,
}

fn said(e: PyErr) -> String {
    Python::attach(|py| e.value(py).to_string())
}

impl RangeRead for PyOpened {
    fn size(&self) -> u64 {
        self.size
    }

    fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>, String> {
        let len = len.min(self.size.saturating_sub(offset));
        if len == 0 {
            return Ok(Vec::new());
        }
        Python::attach(|py| {
            let got = self.reader.call_method1(
                py,
                "read",
                (&self.url, offset, len, self.version.as_deref()),
            )?;
            let bytes: Vec<u8> = got.extract(py)?;
            Ok::<_, PyErr>(bytes)
        })
        .map_err(said)
    }
}

impl Remote for PyRemote {
    fn open(&self, url: &str) -> Result<Box<dyn RangeRead>, String> {
        let (size, version) = Python::attach(|py| {
            let got = self.0.call_method1(py, "size", (url,))?;
            got.extract::<(u64, Option<String>)>(py)
        })
        .map_err(said)?;
        Ok(Box::new(PyOpened {
            reader: Python::attach(|py| self.0.clone_ref(py)),
            url: url.to_owned(),
            size,
            version,
        }))
    }

    fn object(&self, url: &str) -> Result<Option<Vec<u8>>, String> {
        Python::attach(|py| {
            let got = self.0.call_method1(py, "object", (url,))?;
            if got.is_none(py) {
                return Ok(None);
            }
            let bytes: Vec<u8> = got.extract(py)?;
            Ok::<_, PyErr>(Some(bytes))
        })
        .map_err(said)
    }
}

/// Registers `reader` as the module's network (the first registration stays).
#[pyfunction]
pub fn set_remote(reader: Py<PyAny>) {
    kentos_native_application::remote_host::register(Box::new(PyRemote(reader)));
}
