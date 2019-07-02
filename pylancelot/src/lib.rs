use pyo3;
use pyo3::types::{PyBytes};
use pyo3::prelude::*;

use lancelot::workspace;
use lancelot::arch::{RVA};


#[pymodule]
fn pylancelot(_py: Python, m: &PyModule) -> PyResult<()> {
    #[pyfn(m, "from_bytes")]
    /// from_bytes(filename, buf, /)
    /// --
    ///
    /// Create a workspace from the given bytes.
    ///
    /// Args:
    ///   filename (str): the source filename
    ///   buf (bytes): the bytes containing a PE, shellcode, etc.
    ///
    /// Raises:
    ///   ValueError: if failed to create the workspace. TODO: more specific.
    ///
    fn from_bytes(_py: Python, filename: String, buf: &PyBytes) -> PyResult<PyWorkspace> {
        let ws = match workspace::Workspace::from_bytes(&filename, buf.as_bytes()).load() {
            Err(_) => return Err(pyo3::exceptions::ValueError::py_err("failed to create workspace")),
            Ok(ws) => ws,
        };
        Ok(PyWorkspace{ws})
    }

    #[pyclass]
    pub struct PyWorkspace {
        ws: workspace::Workspace
    }

    #[pyclass]
    pub struct PySection {
        #[pyo3(get)]
        pub addr: i64,
        #[pyo3(get)]
        pub length: u64,
        #[pyo3(get)]
        pub perms: u8,
        #[pyo3(get)]
        pub name: String,
    }

    m.add("PERM_NONE", lancelot::loader::Permissions::empty().bits())?;
    m.add("PERM_R", lancelot::loader::Permissions::R.bits())?;
    m.add("PERM_W", lancelot::loader::Permissions::W.bits())?;
    m.add("PERM_X", lancelot::loader::Permissions::X.bits())?;
    m.add("PERM_RW", lancelot::loader::Permissions::RW.bits())?;
    m.add("PERM_RX", lancelot::loader::Permissions::RX.bits())?;
    m.add("PERM_RWX", lancelot::loader::Permissions::RWX.bits())?;

    #[pymethods]
    impl PyWorkspace {
        #[getter]
        /// filename(self, /)
        /// --
        ///
        /// Fetch the filename.
        /// ```
        pub fn filename(&self) -> PyResult<String> {
            Ok(self.ws.filename.clone())
        }

        #[getter]
        /// loader(self, /)
        /// --
        ///
        /// Fetch the name of the loader used to create the workspace.
        pub fn loader(&self) -> PyResult<String> {
            Ok(self.ws.loader.get_name())
        }

        #[getter]
        /// base_address(self, /)
        /// --
        ///
        /// Fetch the base address to which the module was loaded.
        pub fn base_address(&self) -> PyResult<u64> {
            Ok(self.ws.module.base_address.into())
        }

        #[getter]
        pub fn sections(&self) -> PyResult<Vec<PySection>> {
            Ok(self.ws.module.sections.iter()
                .map(|section| PySection{
                    addr: section.addr.into(),
                    length: section.buf.len() as u64,
                    perms: section.perms.bits(),
                    name: section.name.clone(),
                })
                .collect())
        }

        /// probe(self, rva, length=1, /)
        /// --
        ///
        /// Is the given address mapped?
        pub fn probe(&self, rva: i64, length: Option<usize>) -> PyResult<bool> {
            match length {
                Some(length) => Ok(self.ws.probe(RVA::from(rva), length)),
                None =>  Ok(self.ws.probe(RVA::from(rva), 1)),
            }
        }
    }

    #[pyproto]
    impl pyo3::class::basic::PyObjectProtocol for PyWorkspace {
        fn __str__(&self) -> PyResult<String> {
            PyWorkspace::__repr__(self)
        }

        fn __repr__(&self) -> PyResult<String> {
            Ok(format!("PyWorkspace(filename: {} loader: {})",
                self.ws.filename.clone(),
                self.ws.loader.get_name(),
            ))
        }
    }


    #[pyproto]
    impl pyo3::class::basic::PyObjectProtocol for PySection {
        fn __str__(&self) -> PyResult<String> {
            PySection::__repr__(self)
        }

        fn __repr__(&self) -> PyResult<String> {
            Ok(format!("PySection(addr: {:#x} length: {:#x} perms: {:#x} name: {})",
                self.addr,
                self.length,
                self.perms,
                self.name,
            ))
        }
    }

    Ok(())
}


