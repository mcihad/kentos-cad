//! Many-to-many references in a PostGIS database (docs/adr/0590): KentOS's junction table between a child table and a
//! parent table, as the GeoPackage relation profile is in a file (docs/adr/0532 §2). The table has `base_id` (the
//! child's key) and `related_id` (the parent's), both foreign keys that go with their rows (`ON DELETE CASCADE`), and
//! `position` (the reference's order); its comment `kentos_references base=… related=…` marks it KentOS's. Another
//! table joining the two by foreign keys is someone else's: reported, never written over and never read as text.

use kentos_geopackage::relations::Key;
use kentos_native_application::provider::{DbRelation, Edited, MapWrite, ProviderError};
use sqlx::postgres::PgConnection;
use sqlx::{Executor, Row};

use crate::{qualified, refused, shape, unreachable};

/// The mark KentOS's junction tables carry in their comment.
const MARK: &str = "kentos_references";

/// `schema.table` with `public` for a name without a schema.
fn full(table: &str) -> String {
    let (_, schema, name) = qualified(table);
    format!("{schema}.{name}")
}

/// The comment of KentOS's junction between `child` and `parent`.
fn note(child: &str, parent: &str) -> String {
    format!("{MARK} base={} related={}", full(child), full(parent))
}

/// The two tables a KentOS comment names, if it is one.
fn marked(note: &str) -> Option<(String, String)> {
    let rest = note.strip_prefix(MARK)?.trim();
    let mut base = None;
    let mut related = None;
    for word in rest.split_whitespace() {
        if let Some(v) = word.strip_prefix("base=") {
            base = Some(v.to_owned());
        } else if let Some(v) = word.strip_prefix("related=") {
            related = Some(v.to_owned());
        }
    }
    Some((base?, related?))
}

/// A table that joins rows by single-column foreign keys: its foreign keys (column, referenced table), its columns and
/// its comment.
#[derive(Default)]
struct Joining {
    keys: Vec<(String, String)>,
    columns: Vec<String>,
    note: String,
}

/// How the database holds the reference from `child`'s rows to `parent`'s.
pub(crate) async fn relation(
    conn: &mut PgConnection,
    child: &str,
    parent: &str,
) -> Result<DbRelation, ProviderError> {
    let (child_sql, ..) = qualified(child);
    let (parent_sql, ..) = qualified(parent);
    let (child, parent) = (full(child), full(parent));
    let rows = sqlx::query(
        "SELECT n.nspname || '.' || c.relname AS tbl, a.attname::text AS col,
                rn.nspname || '.' || rc.relname AS ref, coalesce(obj_description(c.oid, 'pg_class'), '') AS note
         FROM pg_constraint k
         JOIN pg_class c ON c.oid = k.conrelid JOIN pg_namespace n ON n.oid = c.relnamespace
         JOIN pg_class rc ON rc.oid = k.confrelid JOIN pg_namespace rn ON rn.oid = rc.relnamespace
         JOIN pg_attribute a ON a.attrelid = k.conrelid AND a.attnum = k.conkey[1]
         WHERE k.contype = 'f' AND array_length(k.conkey, 1) = 1
           AND k.confrelid IN (to_regclass($1), to_regclass($2))
         UNION ALL
         SELECT n.nspname || '.' || c.relname, '', '', obj_description(c.oid, 'pg_class')
         FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE c.relkind IN ('r', 'p') AND obj_description(c.oid, 'pg_class') LIKE 'kentos_references%'
         ORDER BY 1, 2",
    )
    .bind(&child_sql)
    .bind(&parent_sql)
    .fetch_all(&mut *conn)
    .await
    .map_err(unreachable)?;
    let mut tables: Vec<(String, Joining)> = Vec::new();
    for r in rows {
        let tbl: String = r.get("tbl");
        if tbl == child || tbl == parent {
            continue;
        }
        let col: String = r.get("col");
        let reference: String = r.get("ref");
        let note: String = r.get("note");
        let at = match tables.iter().position(|(t, _)| *t == tbl) {
            Some(i) => i,
            None => {
                tables.push((tbl.clone(), Joining::default()));
                tables.len() - 1
            }
        };
        let j = &mut tables[at].1;
        j.note = note;
        if !col.is_empty() {
            j.keys.push((col, reference));
        }
    }
    let mut found: Option<DbRelation> = None;
    for (tbl, j) in &mut tables {
        let (_, schema, name) = qualified(tbl);
        j.columns = sqlx::query_scalar(
            "SELECT column_name::text FROM information_schema.columns WHERE table_schema = $1 AND table_name = $2",
        )
        .bind(&schema)
        .bind(&name)
        .fetch_all(&mut *conn)
        .await
        .map_err(unreachable)?;
        let to = |table: &str| j.keys.iter().any(|(_, r)| r == table);
        let kept = match marked(&j.note) {
            Some((b, r)) if b == child && r == parent => {
                let key =
                    |col: &str, table: &str| j.keys.iter().any(|(c, r)| c == col && r == table);
                if key("base_id", &child)
                    && key("related_id", &parent)
                    && j.columns.iter().any(|c| c == "position")
                {
                    DbRelation::Mapped(tbl.clone())
                } else {
                    DbRelation::Broken(format!(
                        "“{tbl}” KentOS'un bağlantı tablosu olarak işaretli ama {child}'e giden base_id, {parent}'e giden related_id yabancı anahtarı ya da position sütunu yok"
                    ))
                }
            }
            Some((b, r)) if b == parent && r == child => DbRelation::Foreign(format!(
                "“{tbl}” ilişkisi ters yönde (taban {parent}, ilgili {child}); KentOS tabanı alt katman yazar"
            )),
            Some(_) => continue,
            None if to(&child) && to(&parent) => DbRelation::Foreign(format!(
                "“{tbl}” tablosu {child} ile {parent}'i yabancı anahtarlarla bağlıyor ama KentOS'un bağlantı tablosu değil"
            )),
            None => continue,
        };
        // KentOS's wins; otherwise the first one found is told.
        match (&found, &kept) {
            (None, _)
            | (Some(DbRelation::Foreign(_) | DbRelation::Broken(_)), DbRelation::Mapped(_)) => {
                found = Some(kept);
            }
            _ => {}
        }
    }
    Ok(found.unwrap_or(DbRelation::Text))
}

/// A junction table's rows, by the child's key and the reference's order.
pub(crate) async fn rows(
    conn: &mut PgConnection,
    mapping: &str,
) -> Result<Vec<(i64, i64)>, ProviderError> {
    let (sql, ..) = qualified(mapping);
    let exists: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
        .bind(&sql)
        .fetch_one(&mut *conn)
        .await
        .map_err(unreachable)?;
    if exists.is_none() {
        return Err(ProviderError::NoTable(mapping.to_owned()));
    }
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT base_id::bigint, related_id::bigint FROM {sql} ORDER BY base_id, position"
    )))
    .fetch_all(&mut *conn)
    .await
    .map_err(unreachable)
}

/// The pairs with each child's places: 0, 1, … in the order its parents come.
fn placed(pairs: &[(i64, i64)]) -> Vec<(i64, i64, i32)> {
    let mut out = Vec::with_capacity(pairs.len());
    let mut last: Option<i64> = None;
    let mut n = 0i32;
    for (b, r) in pairs {
        n = if last == Some(*b) { n + 1 } else { 0 };
        last = Some(*b);
        out.push((*b, *r, n));
    }
    out
}

async fn insert(
    conn: &mut PgConnection,
    sql_name: &str,
    pairs: &[(i64, i64)],
) -> Result<(), ProviderError> {
    if pairs.is_empty() {
        return Ok(());
    }
    let rows = placed(pairs);
    let bases: Vec<i64> = rows.iter().map(|r| r.0).collect();
    let related: Vec<i64> = rows.iter().map(|r| r.1).collect();
    let places: Vec<i32> = rows.iter().map(|r| r.2).collect();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO {sql_name} (base_id, related_id, position)
         SELECT * FROM unnest($1::bigint[], $2::bigint[], $3::integer[]) ON CONFLICT DO NOTHING"
    )))
    .bind(&bases)
    .bind(&related)
    .bind(&places)
    .execute(&mut *conn)
    .await
    .map_err(refused)?;
    Ok(())
}

/// KentOS's junction table `mapping` between `child` and `parent` with `pairs`, inside an open transaction.
pub(crate) async fn map(
    conn: &mut PgConnection,
    child: &str,
    parent: &str,
    mapping: &str,
    pairs: &[(i64, i64)],
) -> Result<(), ProviderError> {
    let (sql_name, ..) = qualified(mapping);
    let exists: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
        .bind(&sql_name)
        .fetch_one(&mut *conn)
        .await
        .map_err(unreachable)?;
    if exists.is_some() {
        return Err(ProviderError::Exists(full(mapping)));
    }
    let c = shape(&mut *conn, child).await?;
    let p = shape(&mut *conn, parent).await?;
    conn.execute(sqlx::AssertSqlSafe(format!(
        "CREATE TABLE {sql_name} (
           base_id bigint NOT NULL REFERENCES {} ({}) ON DELETE CASCADE,
           related_id bigint NOT NULL REFERENCES {} ({}) ON DELETE CASCADE,
           position integer NOT NULL,
           PRIMARY KEY (base_id, related_id))",
        c.from,
        crate::quote(&c.layer.fid),
        p.from,
        crate::quote(&p.layer.fid)
    )))
    .await
    .map_err(|e| refused(format!("bağlantı tablosu kurulamadı: {e}")))?;
    conn.execute(sqlx::AssertSqlSafe(format!(
        "COMMENT ON TABLE {sql_name} IS '{}'",
        note(child, parent).replace('\'', "''")
    )))
    .await
    .map_err(refused)?;
    conn.execute(sqlx::AssertSqlSafe(format!(
        "CREATE INDEX ON {sql_name} (related_id)"
    )))
    .await
    .map_err(refused)?;
    insert(conn, &sql_name, pairs).await
}

/// Takes KentOS's junction table `mapping` away; a table that is not KentOS's is refused.
pub(crate) async fn unmap(conn: &mut PgConnection, mapping: &str) -> Result<(), ProviderError> {
    let (sql_name, ..) = qualified(mapping);
    let note: Option<String> =
        sqlx::query_scalar("SELECT obj_description(to_regclass($1), 'pg_class')")
            .bind(&sql_name)
            .fetch_one(&mut *conn)
            .await
            .map_err(unreachable)?;
    if note.as_deref().and_then(marked).is_none() {
        return Err(refused(format!(
            "“{}” KentOS'un bağlantı tablosu değil; kaldırılmadı",
            full(mapping)
        )));
    }
    conn.execute(sqlx::AssertSqlSafe(format!("DROP TABLE {sql_name}")))
        .await
        .map_err(refused)?;
    Ok(())
}

/// The key a junction row names: one the database holds, or one a table of the same transaction just inserted.
fn resolve(k: Key, edited: &[Edited]) -> Result<i64, ProviderError> {
    match k {
        Key::Fid(f) => Ok(f),
        Key::Inserted(job, i) => edited
            .get(job)
            .and_then(|e| e.inserted.get(i))
            .copied()
            .ok_or_else(|| refused("bağlantı satırı yazılmayan bir kayda gidiyor")),
    }
}

/// A junction's rows rewritten inside the tables' transaction: those of the children `bases` taken off, `pairs` put in.
pub(crate) async fn write(
    conn: &mut PgConnection,
    m: &MapWrite,
    edited: &[Edited],
) -> Result<(), ProviderError> {
    let (sql_name, ..) = qualified(&m.mapping);
    let bases: Vec<i64> = m
        .bases
        .iter()
        .map(|k| resolve(*k, edited))
        .collect::<Result<_, _>>()?;
    let pairs: Vec<(i64, i64)> = m
        .pairs
        .iter()
        .map(|(b, r)| Ok((resolve(*b, edited)?, resolve(*r, edited)?)))
        .collect::<Result<_, ProviderError>>()?;
    if !bases.is_empty() {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {sql_name} WHERE base_id = ANY($1::bigint[])"
        )))
        .bind(&bases)
        .execute(&mut *conn)
        .await
        .map_err(refused)?;
    }
    insert(conn, &sql_name, &pairs).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mark_names_its_tables_and_places_count_per_child() {
        assert_eq!(
            note("iliski.parsel", "malik"),
            "kentos_references base=iliski.parsel related=public.malik"
        );
        assert_eq!(
            marked("kentos_references base=iliski.parsel related=iliski.malik"),
            Some(("iliski.parsel".into(), "iliski.malik".into()))
        );
        assert_eq!(marked("başka bir not"), None);
        assert_eq!(marked("kentos_references base=a.b"), None);
        assert_eq!(
            placed(&[(1, 5), (1, 3), (2, 5), (4, 1), (4, 2)]),
            vec![(1, 5, 0), (1, 3, 1), (2, 5, 0), (4, 1, 0), (4, 2, 1)]
        );
    }
}
