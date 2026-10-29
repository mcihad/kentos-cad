//! Bağlantıyı dene and the browser's catalogue (docs/adr/0623 §2, §3): a connection tried as the window says it, what
//! the server is (PostgreSQL's and PostGIS's versions, the schemas and tables it shows the user), and a database's
//! tables, views and materialized views read from the catalogue alone: the geometry each has, its system, an estimate
//! of its rows (`reltuples`, never a count: a municipality's million-row tables list at once) and whether a bound
//! layer can read it (a single integer primary key, or for a view an integer `fid`, `id`, `gid` or `ogc_fid`, the
//! rule `shape` keeps). Every failure is said in plain Turkish with what to do ([`diagnose`]).

use std::str::FromStr;
use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgConnection};
use sqlx::{Connection, Row};

use crate::connections::{self, DbConnection, Key};

/// How long Bağlantıyı dene and the browser wait for a server.
const PROBE: Duration = Duration::from_secs(10);

/// What failed, so the window can act on it (ask for the password).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// The server's name does not resolve.
    Host,
    /// Nothing listens there, or the network does not reach it.
    Refused,
    /// No answer in time.
    Timeout,
    /// The server asked for a password KentOS does not know.
    PasswordNeeded,
    /// The password or the user is wrong.
    Password,
    /// The server does not let this user in from here (`pg_hba.conf`, no CONNECT).
    Access,
    /// No database of that name.
    Database,
    /// TLS asked for and not offered, or broken.
    Tls,
    /// The server's certificate does not verify.
    Certificate,
    /// Anything else.
    Other,
}

/// A failure said for people: what happened and what to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnosis {
    pub problem: Problem,
    pub message: String,
}

/// The server as a successful try found it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerFacts {
    /// `PostgreSQL 16.4`.
    pub version: String,
    /// PostGIS's version, none without the extension.
    pub postgis: Option<String>,
    /// The user the server let in.
    pub user: String,
    pub database: String,
    /// The schemas the user sees (the system's aside).
    pub schemas: usize,
    /// The tables and views with a geometry column.
    pub spatial: usize,
    /// All the tables and views.
    pub tables: usize,
}

impl ServerFacts {
    /// One line for the window: `PostgreSQL 16.4, PostGIS 3.4.2 · 3 şema, 27 tablo (12'si geometrili)`.
    pub fn line(&self) -> String {
        let postgis = self
            .postgis
            .as_ref()
            .map_or_else(|| "PostGIS kurulu değil".to_owned(), |v| format!("PostGIS {v}"));
        format!(
            "{}, {postgis} · {} şema, {} tablo ({} geometrili)",
            self.version, self.schemas, self.tables, self.spatial
        )
    }
}

/// What a relation is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationKind {
    Table,
    Partitioned,
    View,
    MaterializedView,
}

impl RelationKind {
    pub fn label(self) -> &'static str {
        match self {
            RelationKind::Table | RelationKind::Partitioned => "tablo",
            RelationKind::View => "view",
            RelationKind::MaterializedView => "somut view",
        }
    }
}

/// A table, view or materialized view of a database, as the catalogue says it.
#[derive(Clone, Debug, PartialEq)]
pub struct Relation {
    pub schema: String,
    pub name: String,
    pub kind: RelationKind,
    /// The first geometry column's type (`MULTIPOLYGON`), none without one.
    pub geometry: Option<String>,
    pub srid: Option<u32>,
    /// How many geometry columns it has (the first is read).
    pub geometry_columns: usize,
    /// The catalogue's estimate of its rows; none for a view or a table never analysed.
    pub estimate: Option<u64>,
    pub comment: Option<String>,
    /// The column a bound layer keys its rows by; none: it cannot be bound.
    pub key: Option<String>,
    /// Whether the user may read it.
    pub readable: bool,
    /// Whether the user may insert, update and delete its rows (a view never).
    pub writable: bool,
}

impl Relation {
    /// `schema.name`, as `cad.sources.edit` takes it.
    pub fn table(&self) -> String {
        format!("{}.{}", self.schema, self.name)
    }

    /// Why a bound layer cannot take it, when it cannot.
    pub fn why_not(&self) -> Option<String> {
        if !self.readable {
            return Some("Bu hesabın okuma yetkisi yok.".to_owned());
        }
        if self.key.is_some() {
            return None;
        }
        Some(match self.kind {
            RelationKind::Table | RelationKind::Partitioned => {
                "Tek sütunlu tam sayı birincil anahtarı yok; bağlı katman satırları bu anahtarla izler. Veritabanı yöneticiniz bir birincil anahtar ekleyebilir.".to_owned()
            }
            RelationKind::View | RelationKind::MaterializedView => {
                "Tam sayı bir fid, id, gid ya da ogc_fid sütunu yok; bağlı katman satırları bu sütunla izler.".to_owned()
            }
        })
    }
}

/// A database's relations and the server they came from.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Catalog {
    pub facts: ServerFacts,
    pub relations: Vec<Relation>,
}

impl Catalog {
    /// The schemas in order, each with its relations shown under `connection`'s options.
    pub fn schemas<'a>(&'a self, connection: &DbConnection) -> Vec<(&'a str, Vec<&'a Relation>)> {
        let mut out: Vec<(&str, Vec<&Relation>)> = Vec::new();
        for r in &self.relations {
            if connection.public_only && r.schema != "public" {
                continue;
            }
            if !connection.geometryless && r.geometry.is_none() {
                continue;
            }
            match out.last_mut() {
                Some((s, list)) if *s == r.schema => list.push(r),
                _ => out.push((r.schema.as_str(), vec![r])),
            }
        }
        out
    }

    /// How many relations without geometry `connection`'s options leave out.
    pub fn hidden(&self, connection: &DbConnection) -> usize {
        if connection.geometryless {
            return 0;
        }
        self.relations
            .iter()
            .filter(|r| r.geometry.is_none() && (!connection.public_only || r.schema == "public"))
            .count()
    }
}

/// `connection`'s options with the password it is tried with: the one typed, else the one KentOS knows, else
/// `PGPASSWORD` (and libpq's `~/.pgpass` through sqlx); whether one was given.
fn options(connection: &DbConnection, typed: Option<&str>) -> Result<(PgConnectOptions, bool), Diagnosis> {
    let mut options = PgConnectOptions::from_str(&connection.address()).map_err(|e| Diagnosis {
        problem: Problem::Other,
        message: format!("Bağlantının bilgileri okunamadı: {e}."),
    })?;
    let known = typed
        .map(str::to_owned)
        .or_else(|| connection.password.clone())
        .or_else(|| connections::password_for(&Key::of(connection)))
        .or_else(|| std::env::var("PGPASSWORD").ok().filter(|p| !p.is_empty()));
    let given = known.is_some();
    if let Some(pw) = known {
        options = options.password(&pw);
    }
    Ok((options, given))
}

/// Who and where a failure is about, for its words.
pub(crate) struct Endpoint<'a> {
    pub host: &'a str,
    pub port: u16,
    pub database: &'a str,
    pub user: &'a str,
}

/// A connection failure said for people: what happened, from the error's kind, its SQLSTATE and its words.
pub(crate) fn diagnose(e: &sqlx::Error, at: &Endpoint<'_>, password_given: bool) -> Diagnosis {
    let said = |problem, message: String| Diagnosis { problem, message };
    match e {
        sqlx::Error::Database(db) => {
            let code = db.code().map(|c| c.into_owned()).unwrap_or_default();
            let words = db.message().to_owned();
            match code.as_str() {
                "28P01" if !password_given => said(
                    Problem::PasswordNeeded,
                    format!("Sunucu “{}” kullanıcısı için parola istiyor; bu bağlantının parolası bilinmiyor. Parolayı yazıp yeniden deneyin.", at.user),
                ),
                "28P01" => said(
                    Problem::Password,
                    format!("Parola ya da kullanıcı adı (“{}”) yanlış. İkisini de denetleyip yeniden deneyin.", at.user),
                ),
                "28000" if words.contains("pg_hba") => said(
                    Problem::Access,
                    format!("Sunucu bu bilgisayardan “{}” kullanıcısıyla “{}” veritabanına bağlanılmasına izin vermiyor (pg_hba.conf). Veritabanı yöneticinizden bu bilgisayar için izin isteyin.", at.user, at.database),
                ),
                "28000" => said(
                    Problem::Access,
                    format!("Sunucu “{}” kullanıcısını kabul etmedi: {words}. Kullanıcı adını denetleyin ya da yöneticinizden yetki isteyin.", at.user),
                ),
                "3D000" => said(
                    Problem::Database,
                    format!("Sunucuda “{}” adlı bir veritabanı yok. Adı (büyük küçük harfiyle) denetleyin.", at.database),
                ),
                "42501" => said(
                    Problem::Access,
                    format!("“{}” kullanıcısının “{}” veritabanına bağlanma yetkisi yok (CONNECT). Yöneticinizden yetki isteyin.", at.user, at.database),
                ),
                "53300" => said(
                    Problem::Other,
                    "Sunucu şu an bağlantı sınırında; biraz sonra yeniden deneyin ya da yöneticinize bildirin.".to_owned(),
                ),
                "57P03" => said(
                    Problem::Other,
                    "Sunucu şu an bağlantı kabul etmiyor (açılıyor ya da kurtarmada); biraz sonra yeniden deneyin.".to_owned(),
                ),
                _ => said(Problem::Other, format!("Sunucu bağlantıyı reddetti: {words} ({code}).")),
            }
        }
        sqlx::Error::Io(io) => {
            let words = io.to_string();
            let lower = words.to_ascii_lowercase();
            if lower.contains("lookup address") || lower.contains("name or service not known")
                || lower.contains("nodename nor servname") || lower.contains("no such host")
                || lower.contains("temporary failure in name resolution")
            {
                return said(
                    Problem::Host,
                    format!("“{}” adlı sunucu bulunamadı. Sunucu adını ya da IP adresini denetleyin; ağa (VPN'e) bağlı olduğunuzdan emin olun.", at.host),
                );
            }
            match io.kind() {
                std::io::ErrorKind::ConnectionRefused => said(
                    Problem::Refused,
                    format!("{}:{} bağlantıyı kabul etmedi: orada PostgreSQL çalışmıyor ya da güvenlik duvarı engelliyor. Sunucu adını ve bağlantı noktasını (genellikle 5432) denetleyin.", at.host, at.port),
                ),
                std::io::ErrorKind::TimedOut => said(
                    Problem::Timeout,
                    format!("{}:{} yanıt vermedi: sunucu kapalı ya da ağ engelli olabilir.", at.host, at.port),
                ),
                std::io::ErrorKind::HostUnreachable | std::io::ErrorKind::NetworkUnreachable => said(
                    Problem::Refused,
                    format!("{} adresine ağdan ulaşılamıyor. Ağ bağlantınızı (VPN'i) denetleyin.", at.host),
                ),
                _ => said(Problem::Other, format!("{}:{} ile bağlantı kurulamadı: {words}.", at.host, at.port)),
            }
        }
        sqlx::Error::Tls(tls) => {
            let words = tls.to_string();
            let lower = words.to_ascii_lowercase();
            if lower.contains("does not support tls") || lower.contains("does not support ssl") {
                said(
                    Problem::Tls,
                    "Sunucu şifreli bağlantıyı (SSL) desteklemiyor. SSL kipini “Varsa kullan” yapın ya da yöneticinizden SSL'i açmasını isteyin.".to_owned(),
                )
            } else if lower.contains("certificate") || lower.contains("unknownissuer") || lower.contains("notvalidforname") {
                said(
                    Problem::Certificate,
                    format!("Sunucunun sertifikası doğrulanamadı ({words}). Kurum sertifikası kullanılıyorsa SSL kipini “Zorunlu” yapın ya da sertifikayı sisteme ekletin."),
                )
            } else {
                said(Problem::Tls, format!("Şifreli bağlantı (SSL) kurulamadı: {words}."))
            }
        }
        sqlx::Error::Protocol(words) if words.to_ascii_lowercase().contains("password") && !password_given => said(
            Problem::PasswordNeeded,
            format!("Sunucu “{}” kullanıcısı için parola istiyor; bu bağlantının parolası bilinmiyor. Parolayı yazıp yeniden deneyin.", at.user),
        ),
        sqlx::Error::Configuration(c) => said(Problem::Other, format!("Bağlantının bilgileri okunamadı: {c}.")),
        other => said(Problem::Other, format!("Bağlantı kurulamadı: {other}.")),
    }
}

async fn open(connection: &DbConnection, typed: Option<&str>) -> Result<PgConnection, Diagnosis> {
    let (options, given) = options(connection, typed)?;
    let at = Endpoint {
        host: connection.host.trim(),
        port: connection.port,
        database: connection.database.trim(),
        user: options.get_username(),
    };
    match tokio::time::timeout(PROBE, PgConnection::connect_with(&options)).await {
        Err(_) => Err(Diagnosis {
            problem: Problem::Timeout,
            message: format!(
                "{}:{} {} saniyede yanıt vermedi: sunucu kapalı, adres yanlış ya da ağ engelli olabilir.",
                at.host,
                at.port,
                PROBE.as_secs()
            ),
        }),
        Ok(Err(e)) => Err(diagnose(&e, &at, given)),
        Ok(Ok(c)) => Ok(c),
    }
}

fn runtime() -> Result<tokio::runtime::Runtime, Diagnosis> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Diagnosis {
            problem: Problem::Other,
            message: format!("Bağlantı için iş parçacığı kurulamadı: {e}."),
        })
}

fn failed(e: sqlx::Error) -> Diagnosis {
    Diagnosis {
        problem: Problem::Other,
        message: format!("Bağlandı ama katalog okunamadı: {e}."),
    }
}

/// The schemas a listing leaves out (the system's, PostGIS's own).
const SYSTEM_SCHEMAS: &str = "('pg_catalog', 'information_schema', 'topology', 'tiger', 'tiger_data')";

async fn facts(conn: &mut PgConnection) -> Result<(ServerFacts, bool), sqlx::Error> {
    let row = sqlx::query(
        "SELECT current_setting('server_version'), current_user::text, current_database()::text,
                (SELECT extversion::text FROM pg_extension WHERE extname = 'postgis')",
    )
    .fetch_one(&mut *conn)
    .await?;
    let version: String = row.try_get(0)?;
    let postgis: Option<String> = row.try_get(3)?;
    let schemas: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM pg_namespace WHERE nspname NOT IN {SYSTEM_SCHEMAS} AND nspname NOT LIKE 'pg\\_%'
           AND has_schema_privilege(oid, 'USAGE')"
    )))
    .fetch_one(&mut *conn)
    .await?;
    Ok((
        ServerFacts {
            version: format!(
                "PostgreSQL {}",
                version.split_whitespace().next().unwrap_or(version.as_str())
            ),
            postgis: postgis.clone(),
            user: row.try_get(1)?,
            database: row.try_get(2)?,
            schemas: usize::try_from(schemas).unwrap_or(0),
            spatial: 0,
            tables: 0,
        },
        postgis.is_some(),
    ))
}

async fn relations(conn: &mut PgConnection, postgis: bool) -> Result<Vec<Relation>, sqlx::Error> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT n.nspname::text, c.relname::text, c.relkind::text, c.reltuples::float8,
                obj_description(c.oid, 'pg_class'),
                has_table_privilege(c.oid, 'SELECT'),
                has_table_privilege(c.oid, 'INSERT,UPDATE,DELETE'),
                (SELECT array_agg(a.attname::text) FROM pg_index i
                   JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
                  WHERE i.indrelid = c.oid AND i.indisprimary),
                (SELECT array_agg(format_type(a.atttypid, a.atttypmod)) FROM pg_index i
                   JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
                  WHERE i.indrelid = c.oid AND i.indisprimary),
                (SELECT array_agg(a.attname::text) FROM pg_attribute a
                  WHERE a.attrelid = c.oid AND a.attnum > 0 AND NOT a.attisdropped
                    AND a.attname IN ('fid', 'id', 'gid', 'ogc_fid')
                    AND format_type(a.atttypid, a.atttypmod) IN ('smallint', 'integer', 'bigint'))
           FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
          WHERE c.relkind IN ('r', 'p', 'v', 'm') AND NOT c.relispartition
            AND n.nspname NOT IN {SYSTEM_SCHEMAS} AND n.nspname NOT LIKE 'pg\\_%'
            AND c.relname NOT IN ('spatial_ref_sys', 'kentos_uids', 'kentos_lineage', 'geometry_columns', 'geography_columns')
            AND has_schema_privilege(n.oid, 'USAGE')
          ORDER BY n.nspname, c.relname"
    )))
    .fetch_all(&mut *conn)
    .await?;
    // The geometry and geography columns, first by each table (PostGIS's catalogs: none without it; docs/adr/0745).
    let mut columns: std::collections::HashMap<(String, String), (String, Option<u32>, usize)> =
        std::collections::HashMap::new();
    if postgis {
        let geometry: Vec<(String, String, String, i32)> = sqlx::query_as(
            "SELECT f_table_schema::text, f_table_name::text, type::text, srid FROM geometry_columns
             UNION ALL
             SELECT f_table_schema::text, f_table_name::text, type::text, srid FROM geography_columns
              ORDER BY 1, 2",
        )
        .fetch_all(&mut *conn)
        .await?;
        for (schema, table, kind, srid) in geometry {
            columns
                .entry((schema, table))
                .and_modify(|e| e.2 += 1)
                .or_insert((kind, u32::try_from(srid).ok().filter(|&s| s > 0), 1));
        }
    }
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let schema: String = row.try_get(0)?;
        let name: String = row.try_get(1)?;
        let kind = match row.try_get::<String, _>(2)?.as_str() {
            "p" => RelationKind::Partitioned,
            "v" => RelationKind::View,
            "m" => RelationKind::MaterializedView,
            _ => RelationKind::Table,
        };
        let tuples: f64 = row.try_get(3)?;
        let pk: Option<Vec<String>> = row.try_get(7)?;
        let pk_types: Option<Vec<String>> = row.try_get(8)?;
        let named: Option<Vec<String>> = row.try_get(9)?;
        // The rule `shape` keeps: a table's single integer primary key; a view's first integer key column of KEYS.
        let key = match kind {
            RelationKind::Table | RelationKind::Partitioned => match (pk.as_deref(), pk_types.as_deref()) {
                (Some([k]), Some([t])) if matches!(t.as_str(), "smallint" | "integer" | "bigint") => Some(k.clone()),
                _ => None,
            },
            RelationKind::View | RelationKind::MaterializedView => {
                let named = named.unwrap_or_default();
                crate::KEYS.iter().find(|k| named.iter().any(|n| n == *k)).map(|k| (*k).to_owned())
            }
        };
        let geometry = columns.get(&(schema.clone(), name.clone()));
        let view = matches!(kind, RelationKind::View | RelationKind::MaterializedView);
        out.push(Relation {
            geometry: geometry.map(|g| g.0.to_ascii_uppercase()),
            srid: geometry.and_then(|g| g.1),
            geometry_columns: geometry.map_or(0, |g| g.2),
            estimate: (!matches!(kind, RelationKind::View) && tuples >= 0.0).then(|| tuples.round() as u64),
            comment: row.try_get::<Option<String>, _>(4)?.filter(|c| !c.trim().is_empty()),
            readable: row.try_get(5)?,
            writable: !view && row.try_get::<bool, _>(6)?,
            key,
            schema,
            name,
            kind,
        });
    }
    Ok(out)
}

/// Bağlantıyı dene: `connection` reached with the password typed in the window (none: the one KentOS knows), and
/// what the server is; or why not.
pub fn probe(connection: &DbConnection, typed: Option<&str>) -> Result<ServerFacts, Diagnosis> {
    browse(connection, typed).map(|c| c.facts)
}

/// `connection`'s database's relations from its catalogue, and the server's facts; or why not.
pub fn browse(connection: &DbConnection, typed: Option<&str>) -> Result<Catalog, Diagnosis> {
    if let Some((_, why)) = connection.problems().into_iter().find(|(f, _)| *f != connections::Field::Name) {
        return Err(Diagnosis {
            problem: Problem::Other,
            message: why,
        });
    }
    runtime()?.block_on(async {
        let mut conn = open(connection, typed).await?;
        let (mut facts, postgis) = facts(&mut conn).await.map_err(failed)?;
        let relations = relations(&mut conn, postgis).await.map_err(failed)?;
        let _ = conn.close().await;
        facts.tables = relations.len();
        facts.spatial = relations.iter().filter(|r| r.geometry.is_some()).count();
        Ok(Catalog { facts, relations })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at() -> Endpoint<'static> {
        Endpoint {
            host: "db.belediye.local",
            port: 5432,
            database: "kent",
            user: "ali",
        }
    }

    #[test]
    fn network_failures_are_said_with_what_to_do() {
        let refused = sqlx::Error::Io(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused"));
        let d = diagnose(&refused, &at(), true);
        assert_eq!(d.problem, Problem::Refused);
        assert!(d.message.contains("5432"), "{}", d.message);
        let dns = sqlx::Error::Io(std::io::Error::other(
            "failed to lookup address information: Name or service not known",
        ));
        let d = diagnose(&dns, &at(), true);
        assert_eq!(d.problem, Problem::Host);
        assert!(d.message.contains("db.belediye.local"));
        let tls = sqlx::Error::Tls("server does not support TLS".into());
        assert_eq!(diagnose(&tls, &at(), true).problem, Problem::Tls);
        let no_password = sqlx::Error::Protocol("expected password for authentication".into());
        assert_eq!(diagnose(&no_password, &at(), false).problem, Problem::PasswordNeeded);
    }

    /// A server's refusal as sqlx gives it: its SQLSTATE and its words.
    #[derive(Debug)]
    struct Refusal(&'static str, &'static str);

    impl std::fmt::Display for Refusal {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.1)
        }
    }

    impl std::error::Error for Refusal {}

    impl sqlx::error::DatabaseError for Refusal {
        fn message(&self) -> &str {
            self.1
        }
        fn code(&self) -> Option<std::borrow::Cow<'_, str>> {
            Some(self.0.into())
        }
        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }
        fn kind(&self) -> sqlx::error::ErrorKind {
            sqlx::error::ErrorKind::Other
        }
    }

    #[test]
    fn the_servers_refusals_are_said_with_what_to_do() {
        let refusal = |code, words| sqlx::Error::Database(Box::new(Refusal(code, words)));
        let auth = refusal("28P01", "password authentication failed for user \"ali\"");
        // No password was given: the server asks for one (the window opens its field); one was: it is wrong.
        assert_eq!(diagnose(&auth, &at(), false).problem, Problem::PasswordNeeded);
        assert_eq!(diagnose(&auth, &at(), true).problem, Problem::Password);
        let hba = refusal("28000", "no pg_hba.conf entry for host \"10.0.0.9\", user \"ali\", database \"kent\"");
        let d = diagnose(&hba, &at(), true);
        assert_eq!(d.problem, Problem::Access);
        assert!(d.message.contains("pg_hba.conf") && d.message.contains("yönetici"), "{}", d.message);
        let d = diagnose(&refusal("3D000", "database \"kent\" does not exist"), &at(), true);
        assert_eq!(d.problem, Problem::Database);
        assert!(d.message.contains("“kent”"));
        assert_eq!(diagnose(&refusal("42501", "permission denied"), &at(), true).problem, Problem::Access);
    }

    #[test]
    fn a_catalogue_shows_what_the_connection_asks_for() {
        let r = |schema: &str, name: &str, geometry: bool| Relation {
            schema: schema.into(),
            name: name.into(),
            kind: RelationKind::Table,
            geometry: geometry.then(|| "MULTIPOLYGON".to_owned()),
            srid: geometry.then_some(5256),
            geometry_columns: usize::from(geometry),
            estimate: Some(10),
            comment: None,
            key: Some("fid".into()),
            readable: true,
            writable: true,
        };
        let catalog = Catalog {
            facts: ServerFacts::default(),
            relations: vec![
                r("kadastro", "malik", false),
                r("kadastro", "parsel", true),
                r("public", "ilce", true),
                r("public", "not", false),
            ],
        };
        let mut c = DbConnection::default();
        let shown: Vec<(&str, usize)> = catalog.schemas(&c).iter().map(|(s, l)| (*s, l.len())).collect();
        assert_eq!(shown, [("kadastro", 1), ("public", 1)]);
        assert_eq!(catalog.hidden(&c), 2);
        c.geometryless = true;
        let shown: Vec<(&str, usize)> = catalog.schemas(&c).iter().map(|(s, l)| (*s, l.len())).collect();
        assert_eq!(shown, [("kadastro", 2), ("public", 2)]);
        c.public_only = true;
        let shown: Vec<(&str, usize)> = catalog.schemas(&c).iter().map(|(s, l)| (*s, l.len())).collect();
        assert_eq!(shown, [("public", 2)]);
        let mut keyless = r("public", "x", true);
        keyless.key = None;
        assert!(keyless.why_not().is_some_and(|w| w.contains("birincil anahtar")));
    }
}
