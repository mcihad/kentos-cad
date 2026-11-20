//! How long a million parcels take through the driver (docs/adr/9100 §9): adding the layer (the catalog), a street, a
//! district and the whole city drawn, a filtered count and a table page. The table is made on the server by
//! `generate_series` (inserting a million rows one by one would measure the inserts); run on a database named by
//! `KENTOS_DATA_PG`, in release:
//! `cargo test --release -p kentos-data-postgis --test perf -- --ignored --nocapture`.

use std::time::Instant;

use kentos_data::{Cancel, Condition, Driver, Order, Request, Source};
use kentos_data_postgis::PostGis;

const SIDE: i64 = 1000;
const CELL: f64 = 30.0;
const X0: f64 = 587_000.0;
const Y0: f64 = 4_400_000.0;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

/// The rows of a view, the first part's time and the whole time.
fn view(source: &dyn Source, r: &Request) -> (u64, f64, f64) {
    let t = Instant::now();
    let mut first = None;
    let mut n = 0u64;
    source
        .features("public.perf_parseller", r, &Cancel::new(), &mut |part| {
            if first.is_none() {
                first = Some(ms(t));
            }
            n += part.len() as u64;
            true
        })
        .expect("features");
    (n, first.unwrap_or(0.0), ms(t))
}

#[test]
#[ignore]
fn a_million_parcels() {
    let Ok(address) = std::env::var("KENTOS_DATA_PG") else {
        eprintln!("KENTOS_DATA_PG yok");
        return;
    };
    let t = Instant::now();
    let source = PostGis.open(&address).expect("open");
    println!("bağlantı: {:.1} ms", ms(t));
    let tx = source.begin().expect("begin");
    let setup = [
        "DROP TABLE IF EXISTS public.perf_parseller".to_owned(),
        "CREATE TABLE public.perf_parseller (fid bigint PRIMARY KEY, ada int, parsel int, mahalle text, alan float8, kullanim text, geom geometry(Polygon, 5256))".to_owned(),
        format!(
            "INSERT INTO public.perf_parseller
             SELECT i * {SIDE} + j + 1, 100 + i, j + 1, 'Mahalle ' || (i / 50), ({CELL} - 4) * ({CELL} - 4) - (i * 7 + j) % 300,
                    (ARRAY['Konut','Ticaret','Park','Eğitim'])[1 + (i * 31 + j) % 4],
                    ST_SetSRID(ST_MakePolygon(ST_MakeLine(ARRAY[
                      ST_MakePoint({X0} + j * {CELL} + 2, {Y0} + i * {CELL} + 2),
                      ST_MakePoint({X0} + j * {CELL} + {CELL} - 2, {Y0} + i * {CELL} + 2),
                      ST_MakePoint({X0} + j * {CELL} + {CELL} - 2, {Y0} + i * {CELL} + {CELL} / 2),
                      ST_MakePoint({X0} + j * {CELL} + {CELL} - 2, {Y0} + i * {CELL} + {CELL} - 2),
                      ST_MakePoint({X0} + j * {CELL} + {CELL} / 2, {Y0} + i * {CELL} + {CELL} - 2),
                      ST_MakePoint({X0} + j * {CELL} + 2, {Y0} + i * {CELL} + {CELL} - 2),
                      ST_MakePoint({X0} + j * {CELL} + 2, {Y0} + i * {CELL} + 2)])), 5256)
             FROM generate_series(0, {SIDE} - 1) i, generate_series(0, {SIDE} - 1) j"
        ),
        "CREATE INDEX perf_parseller_geom ON public.perf_parseller USING GIST (geom)".to_owned(),
    ];
    drop(tx.rollback());
    // The table is made over a plain connection of the test (the driver writes rows, not DDL batches).
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("rt");
    rt.block_on(async {
        use sqlx::Executor;
        let mut o: sqlx::postgres::PgConnectOptions = address.parse().expect("address");
        if let Ok(pw) = std::env::var("PGPASSWORD") {
            o = o.password(&pw);
        }
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_with(o)
            .await
            .expect("pool");
        for s in &setup {
            pool.execute(sqlx::AssertSqlSafe(s.as_str()))
                .await
                .expect("setup");
        }
        pool.execute("ANALYZE public.perf_parseller")
            .await
            .expect("analyze");
    });
    println!("tablo hazır: {:.0} ms", ms(t));

    let t = Instant::now();
    let tables = source.tables().expect("tables");
    println!("katalog ({} tablo): {:.1} ms", tables.len(), ms(t));
    let t = Instant::now();
    let schema = source.schema("public.perf_parseller").expect("schema");
    println!("şema ({} alan): {:.1} ms", schema.fields.len(), ms(t));
    let t = Instant::now();
    let n = source
        .count("public.perf_parseller", &Request::all(), true)
        .expect("count");
    println!("tahmini satır {n}: {:.1} ms", ms(t));

    for (name, w, h, tol) in [
        ("sokak 400 × 300 m", 400.0, 300.0, 0.3),
        ("mahalle 3 × 2 km", 3000.0, 2000.0, 2.0),
        ("ilçe 12 × 8 km", 12_000.0, 8000.0, 8.0),
        ("bütün kent 30 × 30 km", 30_000.0, 30_000.0, 20.0),
    ] {
        let b = [X0 + 9000.0, Y0 + 9000.0, X0 + 9000.0 + w, Y0 + 9000.0 + h];
        let b = if w >= 30_000.0 {
            [X0, Y0, X0 + w, Y0 + h]
        } else {
            b
        };
        let r = Request::view(b, vec!["kullanim".into()], Some(tol));
        let (rows, first, all) = view(source.as_ref(), &r);
        println!("{name}: {rows} satır, ilk parça {first:.1} ms, tamamı {all:.0} ms");
    }

    let t = Instant::now();
    let n = source
        .count(
            "public.perf_parseller",
            &Request {
                condition: Some(Condition::eq("kullanim", "Park")),
                ..Request::all()
            },
            false,
        )
        .expect("count");
    println!("süzgeçli sayım (kullanım = Park) {n}: {:.0} ms", ms(t));
    for offset in [0u64, 500_000] {
        let r = Request {
            order: vec![Order {
                field: "alan".into(),
                descending: true,
            }],
            limit: Some(50),
            offset,
            geometry: false,
            ..Request::all()
        };
        let (rows, _, all) = view(source.as_ref(), &r);
        println!("tablo sayfası (alana göre, {offset}. satırdan) {rows} satır: {all:.0} ms");
    }
    let r = Request {
        condition: Some(Condition::Contains {
            fields: vec!["mahalle".into(), "kullanim".into()],
            text: "EĞİT".into(),
        }),
        limit: Some(50),
        geometry: false,
        ..Request::all()
    };
    let (rows, _, all) = view(source.as_ref(), &r);
    println!("hızlı arama (“EĞİT”, ilk 50) {rows} satır: {all:.0} ms");
}
