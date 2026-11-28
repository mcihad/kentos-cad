//! Projects kept in a source with its data (docs/adr/9107): a GeoPackage's `kentos_projects` table, a PostgreSQL
//! schema's. A project is its `.kcad` bytes under a name, with when and by whom it was last written. A driver whose
//! source can keep projects answers [`crate::Source::project_store`]; a new database driver can keep them as well, and
//! [`crate::conformance::projects`] says whether it keeps them right.

use crate::error::DataError;

/// The table a source keeps its projects in.
pub const PROJECTS_TABLE: &str = "kentos_projects";

/// The longest project name, in characters.
pub const NAME_MOST: usize = 200;

/// A project kept in a source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredProject {
    pub name: String,
    /// When it was last written, UTC (`YYYY-MM-DDTHH:MM:SSZ`), as the source's clock said it.
    pub modified: Option<String>,
    /// Who wrote it last: the account the program ran as.
    pub user: Option<String>,
    /// Its bytes.
    pub size: u64,
}

/// Where a source keeps projects: a file keeps them itself, a database in one of its schemas (`schema`; a file has
/// none and ignores it).
pub trait ProjectStore: Send + Sync {
    /// The projects kept there, by name in the Turkish alphabet's order; none when the table is not there yet.
    fn projects(&self, schema: Option<&str>) -> Result<Vec<StoredProject>, DataError>;
    /// A project's bytes; [`DataError::NoProject`] when none has that name.
    fn read_project(&self, schema: Option<&str>, name: &str) -> Result<Vec<u8>, DataError>;
    /// Writes a project in one transaction, over one of its name; the table is made the first time.
    fn write_project(
        &self,
        schema: Option<&str>,
        name: &str,
        content: &[u8],
        user: Option<&str>,
    ) -> Result<(), DataError>;
    /// Removes a project; [`DataError::NoProject`] when none has that name.
    fn delete_project(&self, schema: Option<&str>, name: &str) -> Result<(), DataError>;
}

/// A project's name as it is kept: trimmed; refused when empty, longer than [`NAME_MOST`] characters or holding a
/// control character.
pub fn checked_name(name: &str) -> Result<&str, DataError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DataError::Invalid("Projenin adı boş olamaz.".into()));
    }
    if name.chars().count() > NAME_MOST {
        return Err(DataError::Invalid(format!(
            "Projenin adı en çok {NAME_MOST} karakter olabilir."
        )));
    }
    if name.chars().any(char::is_control) {
        return Err(DataError::Invalid(
            "Projenin adında satır sonu ya da denetim karakteri olamaz.".into(),
        ));
    }
    Ok(name)
}

/// Projects ordered by name in the Turkish alphabet's order.
pub fn ordered(mut list: Vec<StoredProject>) -> Vec<StoredProject> {
    list.sort_by(|a, b| crate::turkish::compare(&a.name, &b.name));
    list
}
