//! What helps when a source could not be read (docs/adr/0740 §1, plan/gis.md CBS-40): one rule for every place that
//! reads a bound layer's or a join view's table, so a provider's error, a page that was not read, a connection that
//! dropped and a file that moved are worked out the same way wherever they are met (Kaynağın tümü, İstatistik's
//! Kaynağın tümü, a join view's source). The refusal itself is the command's (`cad.sources.query`'s code and words);
//! nothing is written by it, and the same question asked again goes on from its cursor.

use crate::codes;

/// What helps a question that a source refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trouble {
    /// The source is where the link says but did not answer as it is now: ask the same question again (a connection
    /// that dropped, a database restarting, a file another program holds or writes).
    Retry,
    /// The source is not where the link says: link it again (a file moved or deleted, a table renamed or dropped, an
    /// address or a view that changed).
    Relink,
    /// The source changed under the pages read: its first page again.
    Changed,
    /// The question reads more than one answer may: a filter or a box narrows it.
    Narrow,
}

impl Trouble {
    /// What helps a refusal with `code`; none when the question itself is wrong (a field that is not there, a filter
    /// that does not compile) or nothing helps but another question.
    pub fn of(code: &str) -> Option<Trouble> {
        Some(match code {
            codes::SOURCE_UNREACHABLE | codes::IMPORT_UNREADABLE => Trouble::Retry,
            codes::FILE_NOT_FOUND
            | codes::TABLE_NOT_FOUND
            | codes::INVALID_FILE_PATH
            | codes::INVALID_ADDRESS
            | codes::NO_ROW_KEY
            | codes::ROW_KEY_NOT_UNIQUE => Trouble::Relink,
            codes::SOURCE_CHANGED => Trouble::Changed,
            codes::ORDER_TOO_LARGE | codes::INPUT_TOO_LARGE => Trouble::Narrow,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_trouble_has_its_codes_and_the_rest_none() {
        for (code, want) in [
            ("source_unreachable", Some(Trouble::Retry)),
            ("import_unreadable", Some(Trouble::Retry)),
            ("file_not_found", Some(Trouble::Relink)),
            ("table_not_found", Some(Trouble::Relink)),
            ("invalid_file_path", Some(Trouble::Relink)),
            ("invalid_address", Some(Trouble::Relink)),
            ("no_row_key", Some(Trouble::Relink)),
            ("row_key_not_unique", Some(Trouble::Relink)),
            ("source_changed", Some(Trouble::Changed)),
            ("order_too_large", Some(Trouble::Narrow)),
            ("input_too_large", Some(Trouble::Narrow)),
            ("field_not_found", None),
            ("invalid_expression", None),
            ("crs_mismatch", None),
            ("layer_not_found", None),
            ("cancelled", None),
        ] {
            assert_eq!(Trouble::of(code), want, "{code}");
        }
        // Each code the rule names is a code of the table, with the kind the rule takes it for.
        for code in [
            codes::SOURCE_UNREACHABLE,
            codes::IMPORT_UNREADABLE,
            codes::FILE_NOT_FOUND,
            codes::TABLE_NOT_FOUND,
            codes::INVALID_FILE_PATH,
            codes::INVALID_ADDRESS,
            codes::NO_ROW_KEY,
            codes::ROW_KEY_NOT_UNIQUE,
            codes::ORDER_TOO_LARGE,
            codes::INPUT_TOO_LARGE,
        ] {
            assert_eq!(
                codes::info(code).map(|c| c.kind),
                Some(codes::CodeKind::Failed),
                "{code}"
            );
        }
        assert_eq!(
            codes::info(codes::SOURCE_CHANGED).map(|c| c.kind),
            Some(codes::CodeKind::Conflict)
        );
    }
}
