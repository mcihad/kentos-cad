//! Tarih ve saat türleri.
//!
//! Harici bağımlılık olmadan Gregoryen takvim hesabı yapar. Türkçe yazım:
//! `24.09.2026`, `14:30`, `24.09.2026 14:30`. Gün sayıları için Howard
//! Hinnant'ın `days_from_civil` algoritması kullanılır.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Türkçe ay adları.
pub const MONTHS: [&str; 12] = [
    "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim",
    "Kasım", "Aralık",
];

/// Kısaltılmış Türkçe ay adları.
pub const MONTHS_SHORT: [&str; 12] = [
    "Oca", "Şub", "Mar", "Nis", "May", "Haz", "Tem", "Ağu", "Eyl", "Eki", "Kas", "Ara",
];

/// Takvim başlıkları için kısaltılmış gün adları, pazartesiden başlayarak.
pub const WEEKDAYS: [&str; 7] = ["Pt", "Sa", "Ça", "Pe", "Cu", "Ct", "Pz"];

/// Takvim tarihi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    year: i32,
    month: u8,
    day: u8,
}

impl Date {
    /// Geçerli bir tarih; 30 Şubat gibi tarihlerde `None`.
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        ((1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month))
            .then_some(Self { year, month, day })
    }

    pub fn year(self) -> i32 {
        self.year
    }

    /// 1 (Ocak) … 12 (Aralık).
    pub fn month(self) -> u8 {
        self.month
    }

    pub fn day(self) -> u8 {
        self.day
    }

    /// Haftanın günü: 0 pazartesi … 6 pazar.
    pub fn weekday(self) -> u8 {
        // 1 Ocak 1970 perşembeydi (3).
        (self.days_since_epoch() + 3).rem_euclid(7) as u8
    }

    /// 1 Ocak 1970'ten bu yana geçen gün sayısı.
    pub fn days_since_epoch(self) -> i64 {
        days_from_civil(self.year, self.month, self.day)
    }

    pub fn from_days_since_epoch(days: i64) -> Self {
        let (year, month, day) = civil_from_days(days);
        Self { year, month, day }
    }

    pub fn add_days(self, days: i64) -> Self {
        Self::from_days_since_epoch(self.days_since_epoch() + days)
    }

    /// Ay ekler; gün yeni ayda yoksa ayın son gününe çekilir (31 Ocak + 1 ay
    /// → 28 ya da 29 Şubat).
    pub fn add_months(self, months: i32) -> Self {
        let total = self.year * 12 + i32::from(self.month) - 1 + months;
        let year = total.div_euclid(12);
        let month = (total.rem_euclid(12) + 1) as u8;

        Self {
            year,
            month,
            day: self.day.min(days_in_month(year, month)),
        }
    }

    pub fn first_of_month(self) -> Self {
        Self { day: 1, ..self }
    }

    /// "24.09.2026" gibi yazımları çözümler; ayırıcı olarak nokta, eğik çizgi
    /// ve tire kabul edilir. ISO biçimi "2026-09-24" de kabul edilir.
    pub fn parse(text: &str) -> Option<Self> {
        let parts: Vec<&str> = text.trim().split(['.', '/', '-']).collect();
        let [first, second, third] = parts.as_slice() else {
            return None;
        };

        let (year, month, day) = if first.len() == 4 {
            (first, second, third)
        } else {
            (third, second, first)
        };

        Self::new(year.parse().ok()?, month.parse().ok()?, day.parse().ok()?)
    }

    /// Ayın Türkçe adıyla: "Eylül 2026".
    pub fn month_title(self) -> String {
        format!("{} {}", MONTHS[usize::from(self.month) - 1], self.year)
    }

    /// ISO 8601 hafta numarası (1–53): hafta pazartesi başlar, yılın ilk
    /// haftası ilk perşembeyi içeren haftadır.
    pub fn iso_week(self) -> u8 {
        let thursday = self.add_days(3 - i64::from(self.weekday()));
        let first = Date::new(thursday.year(), 1, 1).unwrap_or(thursday);

        ((thursday.days_since_epoch() - first.days_since_epoch()) / 7 + 1) as u8
    }

    /// Hafta sonu mu (cumartesi ya da pazar).
    pub fn is_weekend(self) -> bool {
        self.weekday() >= 5
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}.{:02}.{:04}", self.day, self.month, self.year)
    }
}

/// Artık yıl mı.
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Ayın gün sayısı.
pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Günün saati.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time {
    hour: u8,
    minute: u8,
    second: u8,
}

impl Time {
    pub const MIDNIGHT: Self = Self {
        hour: 0,
        minute: 0,
        second: 0,
    };

    pub fn new(hour: u8, minute: u8, second: u8) -> Option<Self> {
        (hour < 24 && minute < 60 && second < 60).then_some(Self {
            hour,
            minute,
            second,
        })
    }

    pub fn hour(self) -> u8 {
        self.hour
    }

    pub fn minute(self) -> u8 {
        self.minute
    }

    pub fn second(self) -> u8 {
        self.second
    }

    /// "14:30", "14:30:05" ya da "14.30" yazımlarını çözümler.
    pub fn parse(text: &str) -> Option<Self> {
        let parts: Vec<&str> = text.trim().split([':', '.']).collect();

        match parts.as_slice() {
            [hour, minute] => Self::new(hour.parse().ok()?, minute.parse().ok()?, 0),
            [hour, minute, second] => Self::new(
                hour.parse().ok()?,
                minute.parse().ok()?,
                second.parse().ok()?,
            ),
            _ => None,
        }
    }

    fn from_seconds(seconds: i64) -> Self {
        let seconds = seconds.rem_euclid(86_400);

        Self {
            hour: (seconds / 3_600) as u8,
            minute: (seconds / 60 % 60) as u8,
            second: (seconds % 60) as u8,
        }
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.second == 0 {
            write!(f, "{:02}:{:02}", self.hour, self.minute)
        } else {
            write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
        }
    }
}

/// Tarih ve saat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime {
    pub date: Date,
    pub time: Time,
}

impl DateTime {
    pub fn new(date: Date, time: Time) -> Self {
        Self { date, time }
    }

    /// "24.09.2026 14:30" yazımını çözümler; saat yazılmazsa gece yarısıdır.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();

        match text.split_once(char::is_whitespace) {
            Some((date, time)) => Some(Self::new(Date::parse(date)?, Time::parse(time)?)),
            None => Some(Self::new(Date::parse(text)?, Time::MIDNIGHT)),
        }
    }

    /// Unix zaman damgasından (saniye) UTC tarih ve saat.
    pub fn from_unix(seconds: i64) -> Self {
        Self {
            date: Date::from_days_since_epoch(seconds.div_euclid(86_400)),
            time: Time::from_seconds(seconds),
        }
    }

    /// Sistem saatine göre şu an; UTC'ye `offset_minutes` dakika eklenir.
    /// Türkiye için +180.
    pub fn now(offset_minutes: i32) -> Self {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs() as i64);

        Self::from_unix(seconds + i64::from(offset_minutes) * 60)
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.date, self.time)
    }
}

fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day_of_year =
        (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i32, u8, u8) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);

    (year as i32, month as u8, day as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::new(year, month, day).expect("geçerli tarih")
    }

    #[test]
    fn iso_weeks_follow_the_first_thursday() {
        let week = |year, month, day| Date::new(year, month, day).map(Date::iso_week);

        // 1 Ocak 2026 perşembe: yılın ilk haftası.
        assert_eq!(week(2026, 1, 1), Some(1));
        // 24 Eylül 2026.
        assert_eq!(week(2026, 9, 24), Some(39));
        // 1 Ocak 2027 cuma: 2026'nın 53. haftası.
        assert_eq!(week(2027, 1, 1), Some(53));
        // 29 Aralık 2025 pazartesi: 2026'nın ilk haftası.
        assert_eq!(week(2025, 12, 29), Some(1));
        assert!(Date::new(2026, 9, 26).is_some_and(Date::is_weekend));
    }

    #[test]
    fn invalid_dates_are_rejected() {
        assert!(Date::new(2026, 2, 29).is_none());
        assert!(Date::new(2024, 2, 29).is_some());
        assert!(Date::new(1900, 2, 29).is_none());
        assert!(Date::new(2000, 2, 29).is_some());
        assert!(Date::new(2026, 13, 1).is_none());
        assert!(Date::new(2026, 4, 31).is_none());
    }

    #[test]
    fn epoch_and_weekdays() {
        assert_eq!(date(1970, 1, 1).days_since_epoch(), 0);
        assert_eq!(date(1970, 1, 1).weekday(), 3);
        assert_eq!(date(2026, 9, 24).weekday(), 3);
        assert_eq!(date(2023, 10, 29).weekday(), 6);
    }

    #[test]
    fn day_counts_round_trip() {
        for days in [-800_000, -1, 0, 59, 60, 11_016, 20_000, 2_932_896] {
            assert_eq!(Date::from_days_since_epoch(days).days_since_epoch(), days);
        }
    }

    #[test]
    fn months_are_added_with_day_clamping() {
        assert_eq!(date(2026, 1, 31).add_months(1), date(2026, 2, 28));
        assert_eq!(date(2024, 1, 31).add_months(1), date(2024, 2, 29));
        assert_eq!(date(2026, 12, 15).add_months(1), date(2027, 1, 15));
        assert_eq!(date(2026, 1, 15).add_months(-1), date(2025, 12, 15));
    }

    #[test]
    fn turkish_formats_are_parsed_and_written() {
        assert_eq!(Date::parse("24.09.2026"), Some(date(2026, 9, 24)));
        assert_eq!(Date::parse("24/9/2026"), Some(date(2026, 9, 24)));
        assert_eq!(Date::parse("2026-09-24"), Some(date(2026, 9, 24)));
        assert_eq!(Date::parse("31.02.2026"), None);
        assert_eq!(date(2026, 9, 4).to_string(), "04.09.2026");

        assert_eq!(Time::parse("14:30"), Time::new(14, 30, 0));
        assert_eq!(Time::parse("9.05"), Time::new(9, 5, 0));
        assert_eq!(Time::parse("24:00"), None);
        assert_eq!(
            Time::new(9, 5, 0).map(|time| time.to_string()),
            Some("09:05".into())
        );

        let moment = DateTime::parse("24.09.2026 14:30").expect("tarih ve saat");
        assert_eq!(moment.to_string(), "24.09.2026 14:30");
        assert_eq!(
            DateTime::parse("24.09.2026").map(|moment| moment.time),
            Some(Time::MIDNIGHT)
        );
    }

    #[test]
    fn unix_timestamps_are_converted() {
        let moment = DateTime::from_unix(1_790_000_000);
        assert_eq!(moment.to_string(), "21.09.2026 14:13:20");
        assert_eq!(date(2026, 9, 1).month_title(), "Eylül 2026");
    }
}
