//! The calendar a legal deadline is counted on (docs/adr/9000 §19; plan BUL-16): the days the offices do not work. A
//! deadline whose last day is one of them runs to the first working day after it — the general rule of reckoning time,
//! as 2577 sayılı İYUK m.8 writes it for a last day that falls on an official holiday.
//!
//! - The weekend and the fixed holidays of 2429 sayılı Kanun are this file: they do not change from year to year.
//! - The religious holidays move with the lunar calendar: they are `kentos.holiday` (migration 0050), kept by the
//!   platform's administrators, held here in memory and read again when they change.
//! - A half day (the arife) is a working day for a deadline: the offices work its morning. The stricter reading keeps
//!   a municipality from missing a deadline the law did not extend.
//! - An administrative leave (idari izin) is not a holiday: it does not extend a deadline, and it is not in the table.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock, RwLock};

use kentos_contracts::{HolidayCreate, HolidayView};
use kentos_postgres::{Db, Scope};
use time::{Date, Month, Weekday};

use super::accounts::user_scope;
use super::{date_at, text_at};
use crate::error::{AppError, AppResult};
use crate::identity::Actor;

/// The religious holidays as the table has them: a day's name and whether only half of it is off.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Holidays {
    days: BTreeMap<Date, (String, bool)>,
}

/// The fixed holidays of 2429 sayılı Kanun (m.1 the national day, m.2 the general holidays).
fn fixed(d: Date) -> Option<&'static str> {
    match (d.month(), d.day()) {
        (Month::January, 1) => Some("Yılbaşı"),
        (Month::April, 23) => Some("Ulusal Egemenlik ve Çocuk Bayramı"),
        (Month::May, 1) => Some("Emek ve Dayanışma Günü"),
        (Month::May, 19) => Some("Atatürk'ü Anma, Gençlik ve Spor Bayramı"),
        (Month::July, 15) => Some("Demokrasi ve Millî Birlik Günü"),
        (Month::August, 30) => Some("Zafer Bayramı"),
        (Month::October, 29) => Some("Cumhuriyet Bayramı"),
        _ => None,
    }
}

const WEEKDAYS: [&str; 7] = [
    "Pazartesi",
    "Salı",
    "Çarşamba",
    "Perşembe",
    "Cuma",
    "Cumartesi",
    "Pazar",
];
const MONTHS: [&str; 12] = [
    "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim",
    "Kasım", "Aralık",
];

/// `1 Haziran 2026 Pazartesi`.
pub fn day_text(d: Date) -> String {
    format!(
        "{} {} {} {}",
        d.day(),
        MONTHS[usize::from(u8::from(d.month())) - 1],
        d.year(),
        WEEKDAYS[usize::from(d.weekday().number_days_from_monday())]
    )
}

impl Holidays {
    pub fn new(days: impl IntoIterator<Item = (Date, String, bool)>) -> Self {
        Self {
            days: days
                .into_iter()
                .map(|(d, name, half)| (d, (name, half)))
                .collect(),
        }
    }

    /// Why the offices do not work on `d`, if they do not: the weekend, a fixed holiday or a full religious one.
    pub fn closed(&self, d: Date) -> Option<String> {
        if let Some(name) = fixed(d) {
            return Some(name.to_owned());
        }
        if let Some((name, false)) = self.days.get(&d) {
            return Some(name.clone());
        }
        match d.weekday() {
            Weekday::Saturday => Some("Cumartesi".into()),
            Weekday::Sunday => Some("Pazar".into()),
            _ => None,
        }
    }

    /// The first working day on or after `d`.
    pub fn working_day_from(&self, d: Date) -> Date {
        let mut day = d;
        while self.closed(day).is_some() {
            match day.next_day() {
                Some(next) => day = next,
                None => break,
            }
        }
        day
    }

    /// A deadline's last day as the law counts it: `due`, or the first working day after it, with why.
    pub fn extended(&self, due: Date) -> (Date, Option<String>) {
        match self.closed(due) {
            None => (due, None),
            Some(why) => {
                let to = self.working_day_from(due);
                (
                    to,
                    Some(format!(
                        "Son gün {} ({why}) iş günü olmadığından süre izleyen ilk iş gününe uzar: {} (2577 sayılı İYUK m.8; tatiller 2429 sayılı Kanun).",
                        day_text(due),
                        day_text(to)
                    )),
                )
            }
        }
    }
}

static CURRENT: OnceLock<RwLock<Arc<Holidays>>> = OnceLock::new();

fn cell() -> &'static RwLock<Arc<Holidays>> {
    CURRENT.get_or_init(|| RwLock::new(Arc::new(Holidays::default())))
}

/// The calendar as last read from the table (the fixed holidays and the weekend alone before the first read).
pub fn current() -> Arc<Holidays> {
    cell().read().map(|g| Arc::clone(&g)).unwrap_or_default()
}

/// Reads the table again (at start and after every change).
pub async fn load(db: &Db) -> AppResult<()> {
    let mut tx = db.scoped(Scope::default()).await?;
    let rows: Vec<(Date, String, bool)> =
        sqlx::query_as("select day, name, half_day from kentos.holiday order by day")
            .fetch_all(&mut *tx)
            .await?;
    tx.commit().await?;
    if let Ok(mut g) = cell().write() {
        *g = Arc::new(Holidays::new(rows));
    }
    Ok(())
}

/// `GET /v1/holidays?year=`: the year's holidays, fixed and religious, in order; anyone signed in.
pub async fn list(db: &Db, actor: &Actor, year: i32) -> AppResult<Vec<HolidayView>> {
    if !(2000..=2200).contains(&year) {
        return Err(AppError::invalid_at("year", "Yıl geçerli değil."));
    }
    let mut tx = db.scoped(user_scope(actor)).await?;
    let rows: Vec<(Date, String, bool)> = sqlx::query_as(
        "select day, name, half_day from kentos.holiday where extract(year from day) = $1 order by day",
    )
    .bind(year)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let mut out: Vec<HolidayView> = rows
        .into_iter()
        .map(|(d, name, half_day)| HolidayView {
            day: d.to_string(),
            name,
            half_day,
            fixed: false,
        })
        .collect();
    for (month, day) in [(1, 1), (4, 23), (5, 1), (5, 19), (7, 15), (8, 30), (10, 29)] {
        if let Ok(d) =
            Date::from_calendar_date(year, Month::try_from(month).unwrap_or(Month::January), day)
            && let Some(name) = fixed(d)
        {
            out.push(HolidayView {
                day: d.to_string(),
                name: name.into(),
                half_day: false,
                fixed: true,
            });
        }
    }
    out.sort_by(|a, b| a.day.cmp(&b.day).then(b.fixed.cmp(&a.fixed)));
    Ok(out)
}

/// `POST /v1/platform/holidays`: a religious holiday's day (a year's, when its dates are announced); the platform's
/// administrators', through `kentos.holiday_set`, which keeps the platform's record.
pub async fn add(db: &Db, actor: &Actor, input: &HolidayCreate) -> AppResult<HolidayView> {
    let day = date_at("day", Some(&input.day))?
        .ok_or_else(|| AppError::invalid_at("day", "Günü yazın."))?;
    let name = text_at("name", "Adı", &input.name, 2, 80)?;
    if let Some(f) = fixed(day) {
        return Err(AppError::invalid_at(
            "day",
            format!("Bu gün zaten tatil: {f} (2429 sayılı Kanun)."),
        ));
    }
    let mut tx = db.scoped(user_scope(actor)).await?;
    let code: Option<String> = sqlx::query_scalar("select kentos.holiday_set($1, $2, $3)")
        .bind(day)
        .bind(&name)
        .bind(input.half_day)
        .fetch_one(&mut *tx)
        .await?;
    refused(code)?;
    tx.commit().await?;
    load(db).await?;
    Ok(HolidayView {
        day: day.to_string(),
        name,
        half_day: input.half_day,
        fixed: false,
    })
}

/// `DELETE /v1/platform/holidays/{day}`.
pub async fn remove(db: &Db, actor: &Actor, day: &str) -> AppResult<()> {
    let day =
        date_at("day", Some(day))?.ok_or_else(|| AppError::not_found("Böyle bir tatil yok."))?;
    let mut tx = db.scoped(user_scope(actor)).await?;
    let code: Option<String> = sqlx::query_scalar("select kentos.holiday_remove($1)")
        .bind(day)
        .fetch_one(&mut *tx)
        .await?;
    refused(code)?;
    tx.commit().await?;
    load(db).await
}

fn refused(code: Option<String>) -> AppResult<()> {
    match code.as_deref() {
        None => Ok(()),
        Some("forbidden") => Err(AppError::forbidden(
            "Tatil takvimini sistem yöneticileri yazar.",
        )),
        Some(_) => Err(AppError::not_found(
            "Böyle bir tatil yok (sabit tatiller Kanun'dandır, silinmez).",
        )),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn date(y: i32, m: u8, d: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap()
    }

    pub(crate) fn year_2026() -> Holidays {
        let mut days = vec![(
            date(2026, 3, 19),
            "Ramazan Bayramı arifesi".to_owned(),
            true,
        )];
        for d in 20..=22 {
            days.push((
                date(2026, 3, d),
                format!("Ramazan Bayramı {}. gün", d - 19),
                false,
            ));
        }
        days.push((date(2026, 5, 26), "Kurban Bayramı arifesi".to_owned(), true));
        for d in 27..=30 {
            days.push((
                date(2026, 5, d),
                format!("Kurban Bayramı {}. gün", d - 26),
                false,
            ));
        }
        Holidays::new(days)
    }

    #[test]
    fn the_offices_close_on_the_weekend_the_law_s_days_and_a_bayram_but_not_on_its_arife() {
        let h = year_2026();
        assert_eq!(h.closed(date(2026, 11, 21)).as_deref(), Some("Cumartesi"));
        assert_eq!(
            h.closed(date(2026, 10, 29)).as_deref(),
            Some("Cumhuriyet Bayramı")
        );
        assert_eq!(
            h.closed(date(2026, 5, 28)).as_deref(),
            Some("Kurban Bayramı 2. gün")
        );
        assert_eq!(
            h.closed(date(2026, 5, 26)),
            None,
            "the arife's morning is worked"
        );
        assert_eq!(h.closed(date(2026, 11, 18)), None);
        // Without the table the fixed days and the weekend still close.
        assert_eq!(
            Holidays::default().closed(date(2026, 5, 19)).as_deref(),
            Some("Atatürk'ü Anma, Gençlik ve Spor Bayramı")
        );
    }

    #[test]
    fn a_last_day_off_runs_to_the_first_working_day() {
        let h = year_2026();
        // Kurban bayramı's last day is a Saturday: through Sunday to Monday 1 June.
        let (to, why) = h.extended(date(2026, 5, 27));
        assert_eq!(to, date(2026, 6, 1));
        let why = why.unwrap();
        assert!(
            why.starts_with("Son gün 27 Mayıs 2026 Çarşamba (Kurban Bayramı 1. gün)"),
            "{why}"
        );
        assert!(why.contains("1 Haziran 2026 Pazartesi"), "{why}");
        assert_eq!(h.extended(date(2026, 3, 21)).0, date(2026, 3, 23));
        assert_eq!(h.extended(date(2026, 10, 29)).0, date(2026, 10, 30));
        assert_eq!(h.extended(date(2026, 11, 18)), (date(2026, 11, 18), None));
        assert_eq!(day_text(date(2026, 6, 1)), "1 Haziran 2026 Pazartesi");
    }
}
