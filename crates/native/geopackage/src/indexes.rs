//! A table's indexes (docs/adr/0758 §5): read from `PRAGMA index_list` and `PRAGMA index_info` (an index made by
//! CREATE INDEX and a UNIQUE constraint's; the primary key's, a partial one and one over an expression left out: they do
//! not say what a column is looked up by), and written with CREATE INDEX. SQLite takes every empty value (NULL) as
//! different from the others (<https://www.sqlite.org/lang_createindex.html>), so an index taking them as the same is
//! written as a plain unique one, and said.

use rusqlite::ErrorCode;

use crate::{GeoPackage, Result, Session, quote};

/// An index of a table: its SQLite name, its columns in order, whether it is unique, and whether it is a UNIQUE
/// constraint's (SQLite named it) rather than one CREATE INDEX made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Index {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub constraint: bool,
}

/// How an index was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexMade {
    /// Its name in the file (the table's name before it, made free of the file's other names).
    pub name: String,
    /// Whether it was made unique: a unique one the rows already break is made plain.
    pub unique: bool,
}

impl GeoPackage {
    /// `table`'s indexes, oldest first.
    pub fn indexes(&self, table: &str) -> Result<Vec<Index>> {
        let mut st = self
            .conn
            .prepare(&format!("PRAGMA index_list({})", quote(table)))?;
        let listed: Vec<(String, i64, String, i64)> = st
            .query_map([], |r| Ok((r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let mut out = Vec::new();
        for (name, unique, origin, partial) in listed {
            if partial != 0 || !(origin == "u" || origin == "c") {
                continue;
            }
            let mut st = self
                .conn
                .prepare(&format!("PRAGMA index_info({})", quote(&name)))?;
            let cols: Vec<(i64, Option<String>)> = st
                .query_map([], |r| Ok((r.get(1)?, r.get(2)?)))?
                .collect::<rusqlite::Result<_>>()?;
            if cols.is_empty() || cols.iter().any(|(cid, n)| *cid < 0 || n.is_none()) {
                continue;
            }
            out.push(Index {
                name,
                columns: cols.into_iter().filter_map(|(_, n)| n).collect(),
                unique: unique == 1,
                constraint: origin == "u",
            });
        }
        // PRAGMA index_list lists the newest first.
        out.reverse();
        Ok(out)
    }
}

impl Session<'_> {
    /// Makes an index `name` over `table`'s `columns`, named `{table}_{name}` (or with `_2`, `_3` … after it, a name
    /// the file has taken). A unique one the rows already break is made plain.
    pub fn create_index(
        &self,
        table: &str,
        name: &str,
        columns: &[String],
        unique: bool,
    ) -> Result<IndexMade> {
        let base = format!("{table}_{name}");
        let taken = |n: &str| -> Result<bool> {
            let k: i64 = self.tx.query_row(
                "SELECT count(*) FROM sqlite_master WHERE name = ?1 COLLATE NOCASE",
                [n],
                |r| r.get(0),
            )?;
            Ok(k > 0)
        };
        let mut sql_name = base.clone();
        let mut n = 1;
        while taken(&sql_name)? {
            n += 1;
            sql_name = format!("{base}_{n}");
        }
        let list = columns
            .iter()
            .map(|c| quote(c))
            .collect::<Vec<_>>()
            .join(", ");
        let make = |unique: bool| {
            self.tx.execute_batch(&format!(
                "CREATE {}INDEX {} ON {} ({list});",
                if unique { "UNIQUE " } else { "" },
                quote(&sql_name),
                quote(table)
            ))
        };
        if unique {
            match make(true) {
                Ok(()) => {
                    return Ok(IndexMade {
                        name: sql_name,
                        unique: true,
                    });
                }
                // A statement SQLite refuses is undone by itself; the session goes on.
                Err(rusqlite::Error::SqliteFailure(e, _))
                    if e.code == ErrorCode::ConstraintViolation => {}
                Err(e) => return Err(e.into()),
            }
        }
        make(false)?;
        Ok(IndexMade {
            name: sql_name,
            unique: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColumnKind, DataType, GeoPackage, NewRow, TableDef, Value};

    fn column(name: &str) -> crate::Column {
        crate::Column {
            name: name.into(),
            decl: String::new(),
            kind: ColumnKind::Text(None),
            not_null: false,
            default: None,
        }
    }

    #[test]
    fn indexes_go_in_and_come_back() {
        let dir = std::env::temp_dir().join(format!("kentos-gpkg-indexes-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("dizin.gpkg");
        let _ = std::fs::remove_file(&path);
        let mut gp = GeoPackage::create(&path).unwrap();
        let session = gp.session().unwrap();
        let def = TableDef {
            name: "parsel".into(),
            data_type: DataType::Attributes,
            identifier: None,
            description: None,
            srs_id: None,
            geometry: None,
            columns: vec![column("ada"), column("parsel"), column("mahalle")],
        };
        let mut w = session.start_table(&def).unwrap();
        let row = |a: &str, p: &str| NewRow {
            fid: None,
            geometry: None,
            values: vec![Value::Text(a.into()), Value::Text(p.into()), Value::Null],
        };
        session
            .append(
                &mut w,
                &[row("101", "5"), row("101", "6"), row("102", "5")],
                None,
            )
            .unwrap();
        session.finish_table(w).unwrap();
        let key = session
            .create_index(
                "parsel",
                "Ada ve parsel",
                &["ada".into(), "parsel".into()],
                true,
            )
            .unwrap();
        assert_eq!(
            key,
            IndexMade {
                name: "parsel_Ada ve parsel".into(),
                unique: true
            }
        );
        // The rows hold “101” twice: a unique index over ada alone is made plain.
        let ada = session
            .create_index("parsel", "ada", &["ada".into()], true)
            .unwrap();
        assert!(!ada.unique);
        // A name the file has taken gets a number.
        let again = session
            .create_index("parsel", "ada", &["mahalle".into()], false)
            .unwrap();
        assert_eq!(again.name, "parsel_ada_2");
        session.commit().unwrap();
        let read = gp.indexes("parsel").unwrap();
        assert_eq!(
            read,
            vec![
                Index {
                    name: "parsel_Ada ve parsel".into(),
                    columns: vec!["ada".into(), "parsel".into()],
                    unique: true,
                    constraint: false
                },
                Index {
                    name: "parsel_ada".into(),
                    columns: vec!["ada".into()],
                    unique: false,
                    constraint: false
                },
                Index {
                    name: "parsel_ada_2".into(),
                    columns: vec!["mahalle".into()],
                    unique: false,
                    constraint: false
                },
            ]
        );
        drop(gp);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
