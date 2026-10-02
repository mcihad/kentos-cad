//! Vektör oturtma's words (the web's `FitDialog.ts`): the summary under
//! the table (or under Parametrelerle's fields), the parameters in words,
//! the report, the kinds' names and hints, and the reasons there is no
//! solution.

use std::fmt;

use kentos_domain::Document as Model;
use kentos_geometry_core::ops::fit::{self as solver, Derived, FitError, FitKind};
use kentos_interaction::{Format, fixed, js_trim};
use kentos_processing::text::js_number;

use super::{Form, Method, NAME, Scope, TITLE, USE, pair_of};
use crate::calc::traverse::mm_text;
use crate::exchange::words::Kind as Line;

impl Form {
    /// The summary now: the control points' solution, or Parametrelerle's
    /// numbers (at most six problems with them, as the Hesap windows say).
    pub(super) fn summary_lines(&self, model: &Model, format: &Format) -> Vec<(Line, String)> {
        match self.method {
            Method::Points => self.points_summary(format),
            Method::Parameters => match self.params.read(model, format) {
                Ok(p) => p.summary(format),
                Err(problems) => problems.into_iter().take(6).collect(),
            },
        }
    }

    /// The report to copy: Parametrelerle's while its numbers read, else the
    /// control points'.
    pub fn report(&self, model: &Model, format: &Format) -> Option<Vec<Vec<String>>> {
        match self.method {
            Method::Points => Some(self.points_report(format)),
            Method::Parameters => {
                let p = self.params.read(model, format).ok()?;
                Some(p.report(&self.params, format))
            }
        }
    }

    /// The summary under the table (the web's `solvePoints`' lines).
    fn points_summary(&self, format: &Format) -> Vec<(Line, String)> {
        let need = self.kind.need();
        let fit = match &self.solution {
            None | Some(Err(FitError::TooFew(_))) => {
                return vec![(
                    Line::Info,
                    format!(
                        "{} için en az {need} kullanılan çift gerekir; şimdi {}. Koordinatları yazın, yapıştırın ya da çizimden seçin.",
                        kind_title(self.kind),
                        self.used
                    ),
                )];
            }
            Some(Err(e)) => return vec![(Line::Warn, failure(*e).to_owned())],
            Some(Ok(fit)) => fit,
        };
        let dof = 2 * self.used.saturating_sub(need);
        let mut lines = vec![(
            Line::Ok,
            match fit.m0 {
                None => format!(
                    "{} çift tam geçer; m0 için en az bir fazla çift gerekir (serbestlik 0).",
                    self.used
                ),
                Some(m0) => format!(
                    "m0 = ±{} ({} çift, serbestlik {dof}).",
                    mm_text(m0),
                    self.used
                ),
            },
        )];
        lines.push((Line::Info, parameters(fit, format)));
        if let (Some(worst), Some(_)) = (self.worst(), fit.m0) {
            let r = self.pair_rows[worst];
            let name = js_trim(&self.rows[r][NAME]);
            let who = if name.is_empty() {
                format!("{}. satır", r + 1)
            } else {
                name.to_owned()
            };
            lines.push((
                Line::Info,
                format!(
                    "En büyük artık {}: {who}. Kötü bir çifti Kullan'dan çıkarın; çözüm hemen yenilenir.",
                    mm_text(fit.residuals[worst][2])
                ),
            ));
        }
        lines
    }

    /// The control points' report (the web's `report`): the transform, the
    /// pairs with their residuals, m0 and the parameters, tab-separated.
    pub(super) fn points_report(&self, format: &Format) -> Vec<Vec<String>> {
        let strings = |words: &[&str]| words.iter().map(|w| (*w).to_owned()).collect::<Vec<_>>();
        let mut lines = vec![
            strings(&[TITLE, kind_name(self.kind)]),
            strings(&[
                "Kullan", "Ad", "Kaynak Y", "Kaynak X", "Hedef Y", "Hedef X", "vY (mm)", "vX (mm)",
                "v (mm)",
            ]),
        ];
        for row in &self.rows {
            if pair_of(row).is_none() {
                continue;
            }
            let mut line = vec![if row[USE] == "0" { "hayır" } else { "evet" }.to_owned()];
            line.extend(row[NAME..].iter().cloned());
            lines.push(line);
        }
        if let Some(fit) = self.fit() {
            lines.push(vec![
                "m0 (mm)".to_owned(),
                fit.m0
                    .map_or_else(|| "—".to_owned(), |m0| fixed(m0 * 1000.0, 2)),
            ]);
            lines.push(vec!["Parametreler".to_owned(), parameters(fit, format)]);
            lines.push(vec![
                "Kaynak merkezi Y".to_owned(),
                js_number(fit.from.x),
                "Kaynak merkezi X".to_owned(),
                js_number(fit.from.y),
            ]);
            lines.push(vec![
                "Hedef merkezi Y".to_owned(),
                js_number(fit.to.x),
                "Hedef merkezi X".to_owned(),
                js_number(fit.to.y),
            ]);
            let mut numbers = vec!["Merkezli sayılar".to_owned()];
            numbers.extend(fit.params.iter().map(|p| js_number(*p)));
            lines.push(numbers);
        }
        lines
    }
}

/// The parameters in words (the web's `parameters`): Helmert's scale and
/// turn, the affine's scales, turn and shear, the projective's numbers. The
/// affine's scales by the surveyor's axes (CLAUDE.md §5): Y is east (the
/// core's x scale), X north.
fn parameters(fit: &solver::Fit, format: &Format) -> String {
    match fit.derived() {
        Derived::Helmert { scale, rotation } => format!(
            "Ölçek {} ({} ppm), dönüklük {}.",
            fixed(scale, 8),
            fixed((scale - 1.0) * 1e6, 1),
            format.angle(rotation)
        ),
        Derived::Affine {
            scale_x,
            scale_y,
            rotation,
            shear,
        } => format!(
            "Y ölçeği {}, X ölçeği {}, dönüklük {}, kayma {}.",
            fixed(scale_x, 8),
            fixed(scale_y, 8),
            format.angle(rotation),
            format.angle(shear)
        ),
        Derived::Projective => format!(
            "Merkezli sayılar (kaynak merkezi {}): {}.",
            format.point(fit.from),
            fit.params
                .iter()
                .map(|p| fixed(*p, 9))
                .collect::<Vec<_>>()
                .join("; ")
        ),
    }
}

/// A kind as the segmented control names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Named(pub(super) FitKind);

pub(super) const KINDS: [Named; 3] = [
    Named(FitKind::Helmert),
    Named(FitKind::Affine),
    Named(FitKind::Projective),
];

impl fmt::Display for Named {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(kind_title(self.0))
    }
}

/// A scope with its count, as the segmented control shows it: “Seçili (3)”, “Katman”, “Tümü (120)”.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Counted(pub(super) Scope, pub(super) usize);

impl fmt::Display for Counted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Scope::Selection => write!(f, "Seçili ({})", self.1),
            Scope::Layer => f.write_str("Katman"),
            Scope::All => write!(f, "Tümü ({})", self.1),
        }
    }
}

/// A kind as a sentence names it.
pub(super) fn kind_name(kind: FitKind) -> &'static str {
    match kind {
        FitKind::Helmert => "Helmert",
        FitKind::Affine => "afin",
        FitKind::Projective => "projektif",
    }
}

/// A kind at the start of a sentence, and on its segment.
fn kind_title(kind: FitKind) -> &'static str {
    match kind {
        FitKind::Helmert => "Helmert",
        FitKind::Affine => "Afin",
        FitKind::Projective => "Projektif",
    }
}

pub(super) fn kind_hint(kind: FitKind) -> &'static str {
    match kind {
        FitKind::Helmert => "Benzerlik: öteleme, dönüklük ve tek ölçek; en az 2 çift.",
        FitKind::Affine => {
            "X ve Y'ye ayrı ölçek ve kayma; en az 3 çift, bir doğru üstünde olmayan. Daireler ve yaylar elips olur."
        }
        FitKind::Projective => {
            "Perspektif; en az 4 çift, üçü bir doğru üstünde olmayan. Eğriler 0,1 mm'lik köşelere açılır."
        }
    }
}

fn failure(e: FitError) -> &'static str {
    match e {
        FitError::TooFew(_) => "",
        FitError::Coincident => "Kaynak noktaların hepsi aynı yerde; dönüşüm bulunamaz.",
        FitError::Collinear => {
            "Kaynak noktalar bir doğru üstünde; afin dönüşüm bulunamaz. Doğrunun dışında bir çift ekleyin."
        }
        FitError::Singular => {
            "Denklemlerin tek çözümü yok: kaynak noktaların üçü bir doğru üstünde olmamalı."
        }
    }
}
