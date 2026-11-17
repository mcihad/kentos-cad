//! A municipality's figures for a period (docs/adr/9000 §22; plan BUL-20), from its dossiers as its members see them:
//! the applications received, decided (and how fast, in how many rounds), withdrawn, open and late; the transfers;
//! the fees; by kind, by month and by person in charge. The same figures, with the period's dossiers, as a workbook
//! (XLSX through the shared writer; CSV of the dossiers). A period is days of Turkey's calendar, both ends in it.

use kentos_contracts::{
    CellKind, DossierDirection, PortalStatistics, StatisticsAssigneeRow, StatisticsKindRow,
    StatisticsMonthRow, WorkbookColumn, WorkbookFormat, WorkbookSheet, WorkbookWriteInput,
};
use kentos_postgres::Db;
use sqlx::{Postgres, Transaction};
use time::{Date, Month, OffsetDateTime};

use super::catalog;
use super::holidays;
use super::rows::{SUMMARY_SELECT, SummaryRow};
use super::{date_at, today, turkey};
use crate::error::{AppError, AppResult};
use crate::tenancy::Access;

/// The longest period one asks for (a workbook of years of dossiers is a backup, not a report).
const MAX_DAYS: i64 = 3 * 366;

/// The period asked for: this year until today when not given.
pub fn period(from: Option<&str>, to: Option<&str>) -> AppResult<(Date, Date)> {
    let today = today();
    let to = date_at("to", to)?.unwrap_or(today);
    let from = date_at("from", from)?
        .unwrap_or_else(|| Date::from_calendar_date(to.year(), Month::January, 1).unwrap_or(to));
    if from > to {
        return Err(AppError::invalid_at("from", "Dönemin başı sonundan sonra olamaz."));
    }
    if (to - from).whole_days() > MAX_DAYS {
        return Err(AppError::invalid_at("from", "Dönem en çok üç yıl olabilir."));
    }
    Ok((from, to))
}

async fn municipality_tx(db: &Db, access: &Access) -> AppResult<Transaction<'static, Postgres>> {
    let mut tx = db.scoped(access.scope()).await?;
    let municipality: bool =
        sqlx::query_scalar("select category = 'municipality' from kentos.tenant where id = $1")
            .bind(access.tenant)
            .fetch_optional(&mut *tx)
            .await?
            .unwrap_or(false);
    if !municipality {
        return Err(AppError::invalid("İstatistikler belediyelerin başvuru ve aktarımları içindir."));
    }
    Ok(tx)
}

fn count(n: i64) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn days(x: Option<f64>) -> Option<f64> {
    x.map(|v| (v * 10.0).round() / 10.0)
}

/// In the period, on Turkey's calendar: `{col}` between `$2` and `$3`.
fn within(col: &str) -> String {
    format!("({col} at time zone 'Europe/Istanbul')::date between $2 and $3")
}

/// `GET /v1/tenants/{tenant}/portal/statistics`: any member of the municipality.
pub async fn statistics(db: &Db, access: &Access, from: Date, to: Date) -> AppResult<PortalStatistics> {
    let mut tx = municipality_tx(db, access).await?;
    let sent = within("d.sent_at");
    let closed = within("d.closed_at");
    let decided = format!("d.status in ('approved', 'rejected') and {closed}");
    type Totals = (i64, i64, i64, i64, i64, i64, Option<f64>, Option<f64>, Option<f64>, i64, i64);
    let t: Totals = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select count(*) filter (where d.direction = 'application' and {sent}),
                count(*) filter (where d.direction = 'application' and {decided}),
                count(*) filter (where d.direction = 'application' and d.status = 'approved' and {closed}),
                count(*) filter (where d.direction = 'application' and d.status = 'rejected' and {closed}),
                count(*) filter (where d.direction = 'application' and d.status = 'withdrawn' and {closed}),
                count(*) filter (where d.direction = 'application'
                                   and d.status in ('submitted', 'in_review', 'changes_requested')),
                (avg(extract(epoch from d.closed_at - d.sent_at) / 86400)
                  filter (where d.direction = 'application' and {decided}))::float8,
                percentile_cont(0.5) within group
                  (order by (extract(epoch from d.closed_at - d.sent_at) / 86400)::float8)
                  filter (where d.direction = 'application' and {decided}),
                (avg(d.round) filter (where d.direction = 'application' and {decided}))::float8,
                count(*) filter (where d.direction = 'transfer' and {sent}),
                count(*) filter (where d.direction = 'transfer' and d.status = 'completed' and {closed})
           from kentos.dossier d where d.tenant_id = $1"
    )))
    .bind(access.tenant)
    .bind(from)
    .bind(to)
    .fetch_one(&mut *tx)
    .await?;
    let kinds: Vec<(String, i64, i64, i64, Option<f64>)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select d.kind,
                count(*) filter (where {sent}),
                count(*) filter (where d.status = 'approved' and {closed}),
                count(*) filter (where d.status = 'rejected' and {closed}),
                (avg(extract(epoch from d.closed_at - d.sent_at) / 86400) filter (where {decided}))::float8
           from kentos.dossier d
          where d.tenant_id = $1 and d.direction = 'application' and ({sent} or {closed})
          group by d.kind order by 2 desc, 1"
    )))
    .bind(access.tenant)
    .bind(from)
    .bind(to)
    .fetch_all(&mut *tx)
    .await?;
    let months: Vec<(String, i64, i64)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "with m as (select to_char(g, 'YYYY-MM') as month
                      from generate_series(date_trunc('month', $2::date), date_trunc('month', $3::date),
                                           interval '1 month') g)
         select m.month,
                (select count(*) from kentos.dossier d where d.tenant_id = $1 and d.direction = 'application'
                    and {sent} and to_char(d.sent_at at time zone 'Europe/Istanbul', 'YYYY-MM') = m.month),
                (select count(*) from kentos.dossier d where d.tenant_id = $1 and d.direction = 'application'
                    and {decided} and to_char(d.closed_at at time zone 'Europe/Istanbul', 'YYYY-MM') = m.month)
           from m order by m.month"
    )))
    .bind(access.tenant)
    .bind(from)
    .bind(to)
    .fetch_all(&mut *tx)
    .await?;
    let people: Vec<(String, i64, i64)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select coalesce(d.assignee_name, 'Atanmamış'),
                count(*) filter (where d.status in ('submitted', 'in_review', 'changes_requested')),
                count(*) filter (where {decided})
           from kentos.dossier d
          where d.tenant_id = $1 and d.direction = 'application'
          group by 1 having count(*) filter (where d.status in ('submitted', 'in_review', 'changes_requested')) > 0
                         or count(*) filter (where {decided}) > 0
          order by 2 desc, 3 desc, 1"
    )))
    .bind(access.tenant)
    .bind(from)
    .bind(to)
    .fetch_all(&mut *tx)
    .await?;
    let (assessed, paid): (i64, i64) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select coalesce(sum(f.total) filter (where f.state <> 'cancelled' and {}), 0)::bigint,
                coalesce(sum(f.total) filter (where f.state = 'paid' and f.paid_at between $2 and $3), 0)::bigint
           from kentos.dossier_fee f where f.tenant_id = $1",
        within("f.issued_at")
    )))
    .bind(access.tenant)
    .bind(from)
    .bind(to)
    .fetch_one(&mut *tx)
    .await?;
    // Late now: the legal clock of the ones waiting for the municipality, as their pages show it.
    let waiting: Vec<SummaryRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SUMMARY_SELECT} where d.tenant_id = $1 and d.direction = 'application' and d.status in ('submitted', 'in_review')"
    )))
    .bind(access.tenant)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let now = today();
    let calendar = holidays::current();
    let overdue = waiting
        .iter()
        .filter_map(|r| {
            super::clock::clock(
                r.direction(),
                &r.kind,
                r.status(),
                r.round(),
                r.round_sent_at.map(|t| t.to_offset(turkey()).date()),
                now,
                &calendar,
            )
        })
        .filter(|c| c.days_left < 0)
        .count();
    Ok(PortalStatistics {
        from: from.to_string(),
        to: to.to_string(),
        received: count(t.0),
        decided: count(t.1),
        approved: count(t.2),
        rejected: count(t.3),
        withdrawn: count(t.4),
        open_now: count(t.5),
        overdue_now: u32::try_from(overdue).unwrap_or(u32::MAX),
        average_decision_days: days(t.6),
        median_decision_days: days(t.7),
        average_rounds: days(t.8),
        transfers_sent: count(t.9),
        transfers_completed: count(t.10),
        fees_assessed: assessed,
        fees_paid: paid,
        by_kind: kinds
            .into_iter()
            .map(|(kind, received, approved, rejected, avg)| StatisticsKindRow {
                label: catalog::kind_label(DossierDirection::Application, &kind),
                kind,
                received: count(received),
                approved: count(approved),
                rejected: count(rejected),
                average_decision_days: days(avg),
            })
            .collect(),
        by_month: months
            .into_iter()
            .map(|(month, received, decided)| StatisticsMonthRow {
                month,
                received: count(received),
                decided: count(decided),
            })
            .collect(),
        by_assignee: people
            .into_iter()
            .map(|(name, open, decided)| StatisticsAssigneeRow { name, open: count(open), decided: count(decided) })
            .collect(),
    })
}

fn col(name: &str, kind: CellKind) -> WorkbookColumn {
    WorkbookColumn { name: name.into(), kind, decimals: None }
}

fn num(x: Option<f64>) -> String {
    x.map(|v| v.to_string()).unwrap_or_default()
}

fn lira(kurus: i64) -> String {
    format!("{}.{:02}", kurus / 100, (kurus % 100).abs())
}

/// The period's dossiers, one a row, as the workbook's “Dosyalar” sheet and the CSV file.
async fn dossiers_sheet(db: &Db, access: &Access, from: Date, to: Date) -> AppResult<WorkbookSheet> {
    let mut tx = municipality_tx(db, access).await?;
    let rows: Vec<SummaryRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SUMMARY_SELECT} where d.tenant_id = $1 and ({} or {}) order by d.sent_at nulls last, d.created_at",
        within("d.sent_at"),
        within("d.closed_at")
    )))
    .bind(access.tenant)
    .bind(from)
    .bind(to)
    .fetch_all(&mut *tx)
    .await?;
    let closed: Vec<(uuid::Uuid, Option<OffsetDateTime>)> =
        sqlx::query_as("select id, closed_at from kentos.dossier where tenant_id = $1 and closed_at is not null")
            .bind(access.tenant)
            .fetch_all(&mut *tx)
            .await?;
    tx.commit().await?;
    let closed: std::collections::BTreeMap<_, _> = closed.into_iter().collect();
    let day = |t: Option<OffsetDateTime>| t.map(|t| t.to_offset(turkey()).date().to_string()).unwrap_or_default();
    Ok(WorkbookSheet {
        name: "Dosyalar".into(),
        columns: vec![
            col("Dosya no", CellKind::Text),
            col("Yön", CellKind::Text),
            col("Tür", CellKind::Text),
            col("Başlık", CellKind::Text),
            col("Meslek mensubu", CellKind::Text),
            col("Sorumlu", CellKind::Text),
            col("Gönderildi", CellKind::Date),
            col("Durum", CellKind::Text),
            col("Kapandı", CellKind::Date),
            col("Süre (gün)", CellKind::Number),
            col("Tur", CellKind::Number),
        ],
        rows: rows
            .iter()
            .map(|r| {
                let end = closed.get(&r.id).copied().flatten();
                let span = match (r.sent_at, end) {
                    (Some(a), Some(b)) => format!("{:.1}", (b - a).as_seconds_f64() / 86400.0),
                    _ => String::new(),
                };
                vec![
                    r.number().unwrap_or_default(),
                    if r.direction() == DossierDirection::Application { "Başvuru" } else { "Aktarım" }.into(),
                    catalog::kind_label(r.direction(), &r.kind),
                    r.title.clone(),
                    r.professional_name.clone().unwrap_or_default(),
                    r.assignee_name.clone().unwrap_or_default(),
                    day(r.sent_at),
                    catalog::status_label(r.status()).into(),
                    day(end),
                    span,
                    r.round.to_string(),
                ]
            })
            .collect(),
    })
}

/// `GET …/portal/statistics/export?format=xlsx|csv`: the figures and the period's dossiers as a workbook (XLSX), or
/// the dossiers alone as CSV; the bytes, a file name and its media type.
pub async fn export(
    db: &Db,
    access: &Access,
    from: Date,
    to: Date,
    format: WorkbookFormat,
) -> AppResult<(Vec<u8>, String, &'static str)> {
    let s = statistics(db, access, from, to).await?;
    let files = dossiers_sheet(db, access, from, to).await?;
    let summary = WorkbookSheet {
        name: "Özet".into(),
        columns: vec![col("Gösterge", CellKind::Text), col("Değer", CellKind::Auto)],
        rows: [
            ("Dönem", format!("{} – {}", s.from, s.to)),
            ("Gelen başvuru", s.received.to_string()),
            ("Karara bağlanan", s.decided.to_string()),
            ("Onaylanan", s.approved.to_string()),
            ("Reddedilen", s.rejected.to_string()),
            ("Geri çekilen", s.withdrawn.to_string()),
            ("Açık (şimdi)", s.open_now.to_string()),
            ("Süresi geçmiş (şimdi)", s.overdue_now.to_string()),
            ("Ortalama karar süresi (gün)", num(s.average_decision_days)),
            ("Ortanca karar süresi (gün)", num(s.median_decision_days)),
            ("Ortalama tur", num(s.average_rounds)),
            ("Gönderilen aktarım", s.transfers_sent.to_string()),
            ("Tamamlanan aktarım", s.transfers_completed.to_string()),
            ("Tahakkuk (TL)", lira(s.fees_assessed)),
            ("Tahsil (TL)", lira(s.fees_paid)),
        ]
        .into_iter()
        .map(|(k, v)| vec![k.to_owned(), v])
        .collect(),
    };
    let by_kind = WorkbookSheet {
        name: "Türlere göre".into(),
        columns: vec![
            col("Tür", CellKind::Text),
            col("Gelen", CellKind::Number),
            col("Onay", CellKind::Number),
            col("Ret", CellKind::Number),
            col("Ortalama karar (gün)", CellKind::Number),
        ],
        rows: s
            .by_kind
            .iter()
            .map(|k| {
                vec![
                    k.label.clone(),
                    k.received.to_string(),
                    k.approved.to_string(),
                    k.rejected.to_string(),
                    num(k.average_decision_days),
                ]
            })
            .collect(),
    };
    let by_month = WorkbookSheet {
        name: "Aylara göre".into(),
        columns: vec![col("Ay", CellKind::Text), col("Gelen", CellKind::Number), col("Karara bağlanan", CellKind::Number)],
        rows: s.by_month.iter().map(|m| vec![m.month.clone(), m.received.to_string(), m.decided.to_string()]).collect(),
    };
    let by_person = WorkbookSheet {
        name: "Sorumlulara göre".into(),
        columns: vec![col("Sorumlu", CellKind::Text), col("Açık", CellKind::Number), col("Karara bağlanan", CellKind::Number)],
        rows: s.by_assignee.iter().map(|p| vec![p.name.clone(), p.open.to_string(), p.decided.to_string()]).collect(),
    };
    let (sheets, ext, media) = match format {
        WorkbookFormat::Csv => (vec![files], "csv", "text/csv; charset=utf-8"),
        WorkbookFormat::Ods => (
            vec![summary, by_kind, by_month, by_person, files],
            "ods",
            "application/vnd.oasis.opendocument.spreadsheet",
        ),
        WorkbookFormat::Xlsx => (
            vec![summary, by_kind, by_month, by_person, files],
            "xlsx",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),
    };
    let (bytes, _) = kentos_formats::workbook::write(&WorkbookWriteInput { format, sheets })
        .map_err(AppError::invalid)?;
    Ok((bytes, format!("istatistik-{}-{}.{ext}", s.from, s.to), media))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_period_is_this_year_by_default_and_at_most_three_years() {
        let (from, to) = period(None, Some("2026-11-18")).unwrap();
        assert_eq!((from.to_string(), to.to_string()), ("2026-01-01".into(), "2026-11-18".into()));
        assert_eq!(period(Some("2026-12-01"), Some("2026-11-01")).unwrap_err().path(), Some("from"));
        assert!(period(Some("2020-01-01"), Some("2026-01-01")).is_err());
        assert_eq!(lira(751_063), "7510.63");
        assert_eq!(num(Some(3.5)), "3.5");
        assert_eq!(kentos_contracts::DossierStatus::Approved, catalog::status_of("approved").unwrap());
    }
}
