//! The legal deadline a dossier runs under while the municipality has it (docs/adr/9000 §4): İmar Kanunu (3194)
//! m.22 for a building permit (the deficiencies within 15 days, all at once in writing; a complete one decided within
//! 30; a corrected one within 15 of its new application), m.30 for an occupancy permit (30 days, else it counts as
//! given). Counted in calendar days on Turkey's calendar; the law's own rules for a last day that falls on a holiday
//! are not applied here.

use kentos_contracts::{DossierDirection, DossierStatus, LegalClock};
use time::Date;

use super::catalog::{Deadline, kind};

/// The legal deadline running for the municipality on `today`, if the kind has one and the dossier waits for the
/// municipality's answer: `round` packages sent, the last on `round_sent`.
pub fn clock(
    direction: DossierDirection,
    kind_code: &str,
    status: DossierStatus,
    round: u32,
    round_sent: Option<Date>,
    today: Date,
) -> Option<LegalClock> {
    let deadline = kind(direction, kind_code)?.deadline?;
    if !matches!(status, DossierStatus::Submitted | DossierStatus::InReview) {
        return None;
    }
    let sent = round_sent?;
    let (label, days, basis) = match deadline {
        Deadline::BuildingPermit if round <= 1 && today <= sent + time::Duration::days(15) => (
            "Eksikliklerin bildirimi",
            15,
            "Eksik ve yanlışlar başvurudan itibaren 15 gün içinde bütün olarak yazıyla bildirilir (İmar Kanunu m.22).",
        ),
        Deadline::BuildingPermit if round <= 1 => (
            "Karar",
            30,
            "Eksiksiz başvuruda yapı ruhsatı başvurudan itibaren en geç 30 günde verilir (İmar Kanunu m.22).",
        ),
        Deadline::BuildingPermit => (
            "Karar",
            15,
            "Eksikler giderildikten sonraki başvurudan itibaren en geç 15 günde karar verilir (İmar Kanunu m.22).",
        ),
        Deadline::OccupancyPermit => (
            "Karar",
            30,
            "Başvuru en geç 30 günde sonuçlandırılır; aksi hâlde izin verilmiş sayılır (İmar Kanunu m.30).",
        ),
    };
    let due = sent + time::Duration::days(days);
    Some(LegalClock {
        label: label.into(),
        due_on: due.to_string(),
        days_left: i32::try_from((due - today).whole_days()).unwrap_or(i32::MIN),
        basis: basis.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u8, d: u8) -> Date {
        Date::from_calendar_date(y, time::Month::try_from(m).unwrap(), d).unwrap()
    }

    #[test]
    fn a_building_permit_runs_under_article_22() {
        let sent = date(2026, 3, 2);
        let at = |status, round, today| {
            clock(
                DossierDirection::Application,
                "building_permit",
                status,
                round,
                Some(sent),
                today,
            )
        };
        // The first fifteen days: the deficiencies are due.
        let c = at(DossierStatus::Submitted, 1, date(2026, 3, 10)).unwrap();
        assert_eq!(
            (c.label.as_str(), c.due_on.as_str(), c.days_left),
            ("Eksikliklerin bildirimi", "2026-03-17", 7)
        );
        // After them, the decision, thirty days from the application.
        let c = at(DossierStatus::InReview, 1, date(2026, 3, 20)).unwrap();
        assert_eq!(
            (c.label.as_str(), c.due_on.as_str(), c.days_left),
            ("Karar", "2026-04-01", 12)
        );
        // Passed: negative.
        let c = at(DossierStatus::InReview, 1, date(2026, 4, 3)).unwrap();
        assert_eq!(c.days_left, -2);
        // Sent again after its deficiencies: fifteen days from then.
        let c = clock(
            DossierDirection::Application,
            "building_permit",
            DossierStatus::Submitted,
            2,
            Some(date(2026, 5, 4)),
            date(2026, 5, 5),
        )
        .unwrap();
        assert_eq!(
            (c.label.as_str(), c.due_on.as_str()),
            ("Karar", "2026-05-19")
        );
        // Nothing runs while it waits for the professional, or once it is decided.
        assert!(at(DossierStatus::ChangesRequested, 1, date(2026, 3, 10)).is_none());
        assert!(at(DossierStatus::Approved, 1, date(2026, 3, 10)).is_none());
    }

    #[test]
    fn an_occupancy_permit_runs_thirty_days_and_other_kinds_none() {
        let sent = date(2026, 6, 1);
        let c = clock(
            DossierDirection::Application,
            "occupancy_permit",
            DossierStatus::Submitted,
            1,
            Some(sent),
            sent,
        )
        .unwrap();
        assert_eq!((c.due_on.as_str(), c.days_left), ("2026-07-01", 30));
        assert!(
            clock(
                DossierDirection::Application,
                "architectural_project",
                DossierStatus::Submitted,
                1,
                Some(sent),
                sent
            )
            .is_none()
        );
        assert!(
            clock(
                DossierDirection::Transfer,
                "survey",
                DossierStatus::Sent,
                1,
                Some(sent),
                sent
            )
            .is_none()
        );
    }
}
