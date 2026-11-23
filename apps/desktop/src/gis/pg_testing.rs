//! What the desktop's tests on a real PostgreSQL share (the usage report's PostgreSQL remainders): the database
//! `KENTOS_DATA_PG` names (`postgresql://user@host:port/db`, its password in `PGPASSWORD`; without it the test says so
//! and passes, as every database test of the workspace does), a table name of the test's own, a second reader's row
//! count, and a relay between the program and the server.
//!
//! The relay is the real outage: every byte goes through it, and it can cut a connection at the COMMIT of a
//! transaction that wrote, before the COMMIT reaches the server or after it did and before its answer comes back (a
//! lost answer, docs/adr/9126). It can also close, cutting every connection and refusing new ones (the server
//! unreachable), and open again.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The database the tests use, or none (said, and the test passes).
pub(crate) fn pg(what: &str) -> Option<String> {
    let found = std::env::var("KENTOS_DATA_PG").ok().filter(|a| !a.is_empty());
    if found.is_none() {
        eprintln!("KENTOS_DATA_PG yok: {what} atlandı");
    }
    found
}

/// A table name of the test's own in `public`.
pub(crate) fn table(tag: &str) -> String {
    format!("public.kentos_{tag}_{}", std::process::id())
}

/// The rows of `table` as a reader of its own counts them (not the program's connection).
pub(crate) fn rows(address: &str, table: &str) -> u64 {
    let source = kentos_data::registry().source(address).expect("the database");
    source.count(table, &kentos_data::Request::default(), false).expect("counted")
}

/// The table taken away when the test is done.
pub(crate) fn drop_table(address: &str, table: &str) {
    let source = kentos_data::registry().source(address).expect("the database");
    let mut tx = source.begin().expect("begin");
    tx.drop_table(table).expect("drop");
    tx.commit().expect("commit");
}

/// `statements` run in order on the database as the address's account (an account made, its rights given).
pub(crate) fn sql(address: &str, statements: &[&str]) {
    use sqlx::Connection;
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("a runtime");
    runtime.block_on(async {
        let mut options: sqlx::postgres::PgConnectOptions = address.parse().expect("the address");
        if let Ok(pw) = std::env::var("PGPASSWORD") {
            options = options.password(&pw);
        }
        let mut conn = sqlx::PgConnection::connect_with(&options).await.expect("connected");
        for s in statements {
            // The tests' own statements, their names made here: nothing from outside goes in.
            sqlx::query(sqlx::AssertSqlSafe(s.to_string()))
                .execute(&mut conn)
                .await
                .unwrap_or_else(|e| panic!("{s}: {e}"));
        }
    });
}

/// An account of the test's own on the database, its password the tests' (`PGPASSWORD`), with `rights` on `table`
/// (`SELECT`, `SELECT, UPDATE, DELETE`…): its address. An account of that name there before is taken away first.
pub(crate) fn account(address: &str, name: &str, table: &str, rights: &str) -> String {
    let pw = std::env::var("PGPASSWORD").unwrap_or_default().replace('\'', "''");
    let schema = table.split_once('.').map_or("public", |(s, _)| s);
    let gone = format!(
        "DO $$ BEGIN IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '{name}') THEN \
         EXECUTE 'DROP OWNED BY {name}'; EXECUTE 'DROP ROLE {name}'; END IF; END $$"
    );
    sql(
        address,
        &[
            &gone,
            &format!("CREATE ROLE {name} LOGIN PASSWORD '{pw}'"),
            &format!("GRANT USAGE ON SCHEMA {schema} TO {name}"),
            &format!("GRANT {rights} ON {table} TO {name}"),
            &format!("GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA {schema} TO {name}"),
        ],
    );
    let rest = address.split_once("://").map_or(address, |(_, r)| r);
    let after = rest.split_once('@').map_or(rest, |(_, r)| r);
    format!("postgresql://{name}@{after}")
}

/// Where a relay cuts the next COMMIT of a transaction that wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cut {
    Pass,
    /// The COMMIT never reaches the server: the transaction is rolled back there.
    BeforeCommit,
    /// The COMMIT reaches the server and is made; its answer never comes back.
    AfterCommit,
}

/// A relay between the program and the server (see the module).
pub(crate) struct Relay {
    /// The database's address through the relay.
    pub address: String,
    cut: Arc<Mutex<Cut>>,
    open: Arc<AtomicBool>,
    live: Arc<Mutex<Vec<TcpStream>>>,
}

/// `host:port` of a `postgresql://user@host:port/db` address.
fn host_port(address: &str) -> String {
    let rest = address.split_once("://").map_or(address, |(_, r)| r);
    let rest = rest.split_once('@').map_or(rest, |(_, r)| r);
    rest.split(['/', '?']).next().unwrap_or(rest).to_owned()
}

impl Relay {
    /// A relay to the server of `address`: the same address through a port of its own.
    pub(crate) fn over(address: &str) -> Relay {
        let target = host_port(address);
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
        let port = listener.local_addr().expect("its port").port();
        let relayed = address.replacen(&target, &format!("127.0.0.1:{port}"), 1);
        let cut = Arc::new(Mutex::new(Cut::Pass));
        let open = Arc::new(AtomicBool::new(true));
        let live: Arc<Mutex<Vec<TcpStream>>> = Arc::new(Mutex::new(Vec::new()));
        let (c, o, l) = (cut.clone(), open.clone(), live.clone());
        std::thread::spawn(move || {
            for client in listener.incoming().flatten() {
                if !o.load(Ordering::SeqCst) {
                    let _ = client.shutdown(Shutdown::Both);
                    continue;
                }
                let (target, c, l) = (target.clone(), c.clone(), l.clone());
                std::thread::spawn(move || relay(client, &target, &c, &l));
            }
        });
        Relay { address: relayed, cut, open, live }
    }

    /// The next COMMIT of a transaction that wrote is cut as `cut` says (once).
    pub(crate) fn cut(&self, cut: Cut) {
        *self.cut.lock().expect("the cut") = cut;
    }

    /// Closed: every connection cut and new ones refused, the server unreachable; open: through again.
    pub(crate) fn set_open(&self, open: bool) {
        self.open.store(open, Ordering::SeqCst);
        if !open {
            for s in self.live.lock().expect("the connections").drain(..) {
                let _ = s.shutdown(Shutdown::Both);
            }
        }
    }
}

fn holds(chunk: &[u8], word: &[u8]) -> bool {
    chunk.windows(word.len()).any(|w| w.eq_ignore_ascii_case(word))
}

/// One connection relayed: the server's bytes to the program as they come, the program's to the server, a COMMIT of a
/// transaction that wrote cut when the relay says so.
fn relay(client: TcpStream, target: &str, cut: &Mutex<Cut>, live: &Mutex<Vec<TcpStream>>) {
    let Ok(server) = TcpStream::connect(target) else {
        let _ = client.shutdown(Shutdown::Both);
        return;
    };
    if let (Ok(a), Ok(b)) = (client.try_clone(), server.try_clone()) {
        live.lock().expect("the connections").extend([a, b]);
    }
    let dead = Arc::new(AtomicBool::new(false));
    let (Ok(mut from_server), Ok(mut to_client)) = (server.try_clone(), client.try_clone()) else {
        return;
    };
    let d = dead.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 65536];
        loop {
            match from_server.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    // A cut connection's answers never reach the program.
                    if d.load(Ordering::SeqCst) || to_client.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
            }
        }
        let _ = to_client.shutdown(Shutdown::Both);
    });
    let (mut from_client, mut to_server) = (client, server);
    let mut buf = [0u8; 65536];
    let mut wrote = false;
    loop {
        let n = match from_client.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let chunk = &buf[..n];
        if holds(chunk, b"INSERT") || holds(chunk, b"UPDATE ") || holds(chunk, b"DELETE") {
            wrote = true;
        }
        if holds(chunk, b"COMMIT") || holds(chunk, b"ROLLBACK") {
            let now = if wrote && holds(chunk, b"COMMIT") {
                std::mem::replace(&mut *cut.lock().expect("the cut"), Cut::Pass)
            } else {
                Cut::Pass
            };
            wrote = false;
            match now {
                Cut::BeforeCommit => {
                    dead.store(true, Ordering::SeqCst);
                    break;
                }
                Cut::AfterCommit => {
                    dead.store(true, Ordering::SeqCst);
                    let _ = to_server.write_all(chunk);
                    // The server makes the COMMIT before it sees the connection go.
                    std::thread::sleep(Duration::from_millis(300));
                    break;
                }
                Cut::Pass => {}
            }
        }
        if to_server.write_all(chunk).is_err() {
            break;
        }
    }
    let _ = to_server.shutdown(Shutdown::Both);
    let _ = from_client.shutdown(Shutdown::Both);
}
