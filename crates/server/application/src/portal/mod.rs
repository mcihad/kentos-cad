//! The cloud portal (docs/adr/9000): the platform's administrators, organisations (municipalities first) and the
//! professionals who work with them, and the dossiers between a municipality and a professional: applications the
//! professional sends, projects the municipality transfers.
//!
//! - [`accounts`]: who the signed-in person is on the portal; signing up as a professional; one's own profile.
//! - [`platform`]: the platform administrators' use cases (organisations, accounts, professionals, settings, record).
//! - [`organization`]: an organisation's portal settings, members and directory of professionals.
//! - [`dossiers`]: opening, listing, reading, editing and removing dossiers; [`steps`] their steps and messages;
//!   [`files`] their files (uploads, a cloud project's state, a KCAD file opened as a project).
//! - [`lists`]: the lists of dossiers and the overviews' figures.
//! - [`flow`] is the steps' rule, [`catalog`] the names, [`clock`] the legal deadlines; all three pure.
//!
//! Row-level security draws the line between the sides (migration 0043): a professional's transactions never carry an
//! organisation's scope, so nothing of a municipality but its dossiers with them is visible to them.

pub mod accounts;
pub mod catalog;
pub mod clock;
pub mod detail;
pub mod dossiers;
pub mod fees;
pub mod files;
pub mod flow;
pub mod lists;
pub mod notifications;
pub mod organization;
pub mod platform;
mod rows;
pub mod steps;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::error::{AppError, AppResult};

/// A security-definer function's refusal (migration 0043), as the person reads it.
pub(crate) fn refused(code: &str) -> AppError {
    match code {
        "forbidden" => AppError::forbidden("Bu işlem için yetkiniz yok."),
        "login_taken" => {
            AppError::invalid_at("login", "Bu giriş adı kullanılıyor; başka bir ad seçin.")
        }
        "slug_taken" => AppError::invalid_at(
            "slug",
            "Bu kısa ad başka bir kurumda kullanılıyor; başka bir ad seçin.",
        ),
        "no_seat" => AppError::invalid(
            "Kurumun koltukları dolu: bir üyenin koltuğunu boşaltın ya da sistem yönetiminden koltuk isteyin.",
        ),
        "last_owner" => AppError::invalid(
            "Kurumun son sahibinin rolü düşürülemez ya da üyeliği kapatılamaz: önce başka birini sahip yapın.",
        ),
        "not_found" => AppError::not_found("Kayıt bulunamadı."),
        "signup_closed" => {
            AppError::forbidden("Şu anda yeni kayıt alınmıyor; sistem yöneticisine başvurun.")
        }
        "seats_below_used" => AppError::invalid_at(
            "seats",
            "Koltuk sayısı kullanılan koltuklardan az olamaz: önce üyelerden koltuk boşaltın.",
        ),
        "self" => AppError::invalid("Kendi hesabınızın durumunu buradan değiştiremezsiniz."),
        "not_local" => {
            AppError::invalid("Bu hesap parolayla değil, kurum girişiyle oturum açıyor.")
        }
        "wrong_password" => AppError::invalid_at("current", "Şimdiki parolanız doğru değil."),
        "invalid" => AppError::invalid("Verilen değer geçerli değil."),
        other => AppError::invalid(format!("İşlem yapılamadı ({other}).")),
    }
}

/// A definer function's answer: done, or refused.
pub(crate) fn answer(code: Option<String>) -> AppResult<()> {
    match code {
        None => Ok(()),
        Some(code) => Err(refused(&code)),
    }
}

pub(crate) fn rfc3339(t: OffsetDateTime) -> String {
    t.format(&Rfc3339).unwrap_or_default()
}

/// Turkey's time (UTC+3 all year since 2016): legal days are counted on its calendar.
pub(crate) fn turkey() -> time::UtcOffset {
    time::UtcOffset::from_hms(3, 0, 0).unwrap_or(time::UtcOffset::UTC)
}

pub(crate) fn today() -> time::Date {
    OffsetDateTime::now_utc().to_offset(turkey()).date()
}

/// Text as given, without the spaces around it; empty is none.
pub(crate) fn tidy(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
}

/// A required text of `min..=max` characters at `field`, trimmed.
pub(crate) fn text_at(
    field: &str,
    label: &str,
    value: &str,
    min: usize,
    max: usize,
) -> AppResult<String> {
    let value = value.trim();
    let n = value.chars().count();
    if n < min {
        return Err(AppError::invalid_at(
            field,
            if min <= 1 {
                format!("{label} boş olamaz.")
            } else {
                format!("{label} en az {min} karakter olmalı.")
            },
        ));
    }
    if n > max {
        return Err(AppError::invalid_at(
            field,
            format!("{label} en çok {max} karakter olabilir."),
        ));
    }
    Ok(value.to_owned())
}

/// An optional text of at most `max` characters at `field`, trimmed; empty is none.
pub(crate) fn optional_at(
    field: &str,
    label: &str,
    value: Option<&str>,
    max: usize,
) -> AppResult<Option<String>> {
    match tidy(value) {
        Some(v) if v.chars().count() > max => Err(AppError::invalid_at(
            field,
            format!("{label} en çok {max} karakter olabilir."),
        )),
        v => Ok(v),
    }
}

/// An e-mail address as far as the portal checks one: something@something.something, no spaces.
pub(crate) fn email_at(
    field: &str,
    value: Option<&str>,
    required: bool,
) -> AppResult<Option<String>> {
    let Some(v) = tidy(value) else {
        return if required {
            Err(AppError::invalid_at(field, "E-posta adresi gerekli."))
        } else {
            Ok(None)
        };
    };
    let ok = v.len() <= 200
        && !v.contains(char::is_whitespace)
        && v.split_once('@').is_some_and(|(user, host)| {
            !user.is_empty() && host.contains('.') && !host.starts_with('.') && !host.ends_with('.')
        });
    if ok {
        Ok(Some(v))
    } else {
        Err(AppError::invalid_at(
            field,
            format!("“{v}” bir e-posta adresi değil."),
        ))
    }
}

/// A UUID given as text at `field`.
pub(crate) fn id_at(field: &str, value: &str) -> AppResult<uuid::Uuid> {
    uuid::Uuid::parse_str(value.trim())
        .map_err(|_| AppError::invalid_at(field, format!("{field} bir kimlik (UUID) olmalı.")))
}

/// A day given as YYYY-MM-DD at `field`.
pub(crate) fn date_at(field: &str, value: Option<&str>) -> AppResult<Option<time::Date>> {
    let Some(v) = tidy(value) else {
        return Ok(None);
    };
    let bad = || AppError::invalid_at(field, format!("“{v}” bir tarih değil (YYYY-AA-GG)."));
    let mut parts = v.split('-');
    let (Some(y), Some(m), Some(d), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(bad());
    };
    let (Ok(y), Ok(m), Ok(d)) = (y.parse::<i32>(), m.parse::<u8>(), d.parse::<u8>()) else {
        return Err(bad());
    };
    let month = time::Month::try_from(m).map_err(|_| bad())?;
    time::Date::from_calendar_date(y, month, d)
        .map(Some)
        .map_err(|_| bad())
}

/// The first `max` characters of `text`, with an ellipsis when cut.
pub(crate) fn excerpt(text: &str, max: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let cut: String = text.chars().take(max).collect();
    format!("{}…", cut.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_dates_and_addresses_are_checked_where_they_are_given() {
        assert_eq!(
            text_at("title", "Başlık", "  Ada 412  ", 3, 200).unwrap(),
            "Ada 412"
        );
        assert_eq!(
            text_at("title", "Başlık", "ab", 3, 200).unwrap_err().path(),
            Some("title")
        );
        assert_eq!(optional_at("x", "X", Some("   "), 10).unwrap(), None);
        assert!(email_at("email", Some("ayse@ornek.gov.tr"), true).is_ok());
        assert!(email_at("email", Some("ayse@ornek"), true).is_err());
        assert!(
            email_at("email", None, true).is_err()
                && email_at("email", None, false).unwrap().is_none()
        );
        assert_eq!(
            date_at("dueOn", Some("2026-12-31"))
                .unwrap()
                .unwrap()
                .to_string(),
            "2026-12-31"
        );
        assert!(
            date_at("dueOn", Some("2026-02-30")).is_err()
                && date_at("dueOn", Some("31.12.2026")).is_err()
        );
        assert_eq!(excerpt("Statik proje eksik.", 6), "Statik…");
    }
}
