//! A data layer's values for its style's classes (docs/adr/9110 §1). The drawing holds none of its rows, so Değerlerden
//! sınıfla, Sınıfla and the Nesne column read the field from the source, off the interface's thread: each value with how
//! many rows hold it (`Source::groups`, in the field's order). A graduated style's breaks over a field of more values
//! than [`MOST`] read its numbers whole.

use kentos_commands::DataLayer;
use kentos_data::{Cancel, Request};
use kentos_native_style::classify::{ValueCount, unique_values};

/// The most values a field's groups are read to.
pub const MOST: usize = 200_000;

/// What the window says.
pub const READING: &str = "Değerler kaynaktan okunuyor…";
pub const UNREAD: &str = "Değerler kaynaktan okunamadı:";
pub const MORE: &str = "Alanın 200 000'den çok değeri var; ilk 200 000'i listelendi.";
pub const FIELD_ONLY: &str = "Veri katmanı tablosunun bir alanıyla sınıflandırılır: değere alanın adını yazın.";

/// What the window does with the values when they come.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Then {
    /// Only the counts (the window opened on the style).
    Show,
    /// Değerlerden sınıfla.
    Classify,
    /// Sınıfla.
    Graduate,
}

/// What the source said of a field.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceValues {
    /// The field asked.
    pub field: String,
    /// Each value as the drawing reads it (none: empty) with its rows.
    pub groups: Vec<(Option<String>, u64)>,
    /// Each number with its rows (from the groups, or every number when there were more values than [`MOST`]).
    pub numbers: Vec<(Option<f64>, u64)>,
    /// More values than [`MOST`] were there.
    pub more: bool,
}

impl SourceValues {
    /// The values a classification lists, each with its rows: those there, not empty, in the order a drawing layer's
    /// are listed (`unique_values`: Turkish, numbers by their size).
    pub fn values(&self) -> Vec<ValueCount> {
        let once: Vec<Option<String>> = self.groups.iter().map(|(v, _)| v.clone()).collect();
        let rows: std::collections::HashMap<&str, u64> =
            self.groups.iter().filter_map(|(v, n)| v.as_deref().map(|v| (v, *n))).collect();
        unique_values(&once)
            .into_iter()
            .map(|v| ValueCount {
                count: usize::try_from(rows.get(v.value.as_str()).copied().unwrap_or(0)).unwrap_or(usize::MAX),
                value: v.value,
            })
            .collect()
    }

    /// Every number, as many times as rows hold it (a graduated style's breaks).
    pub fn every_number(&self) -> Vec<f64> {
        self.numbers
            .iter()
            .filter_map(|(v, n)| v.map(|v| (v, *n)))
            .flat_map(|(v, n)| std::iter::repeat_n(v, usize::try_from(n).unwrap_or(0)))
            .collect()
    }
}

fn number(text: &str) -> Option<f64> {
    text.trim().parse::<f64>().ok().filter(|x| x.is_finite())
}

/// The field's values read from the source (the thread's work).
pub fn read(data: &DataLayer, field: &str, then: Then) -> Result<SourceValues, String> {
    let source = kentos_data::registry().source(&data.address).map_err(|e| e.to_string())?;
    let mut groups = source.groups(&data.table, field, None, MOST + 1).map_err(|e| e.to_string())?;
    let more = groups.len() > MOST;
    groups.truncate(MOST);
    let groups: Vec<(Option<String>, u64)> = groups
        .into_iter()
        .map(|(v, n)| (kentos_native_application::geopackage::attribute_text(&v), n))
        .collect();
    let numbers = if more && then == Then::Graduate {
        // The breaks need every number: the field's column read, nothing else.
        let request = Request {
            columns: Some(vec![field.to_owned()]),
            geometry: false,
            ..Request::default()
        };
        let mut all = Vec::new();
        source
            .features(&data.table, &request, &Cancel::new(), &mut |part| {
                all.extend(part.iter().map(|f| (f.values.first().and_then(kentos_data::Value::number), 1)));
                true
            })
            .map_err(|e| e.to_string())?;
        all
    } else {
        groups.iter().map(|(v, n)| (v.as_deref().and_then(number), *n)).collect()
    };
    Ok(SourceValues { field: field.to_owned(), groups, numbers, more })
}
