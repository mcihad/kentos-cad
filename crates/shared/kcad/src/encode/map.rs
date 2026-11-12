//! The surveying settings as a payload keeps them: the survey and annotation settings and the heights (docs/adr/0169,
//! 0519).

use super::*;

impl<'d> Encoder<'d> {
    /// The survey settings (docs/adr/0169 §3), checked whole as a reader checks them.
    /// The annotation heights (docs/adr/0205 §1), each kind given, in the
    /// canonical order of their keys; checked as a reader checks them.
    pub(super) fn annotation(&mut self, a: &AnnotationHeights) -> Result<(), KcadError> {
        if let Some((key, problem)) = a.problem() {
            return self.at(Seg::Name(key), |e| Err(e.fail(Code::BadValue, &problem)));
        }
        let order = [
            AnnotationKind::Text,
            AnnotationKind::Table,
            AnnotationKind::Leader,
            AnnotationKind::Measure,
            AnnotationKind::Station,
            AnnotationKind::Dimension,
            AnnotationKind::Coordinate,
        ];
        let given: Vec<(AnnotationKind, f64)> = order
            .into_iter()
            .filter_map(|k| a.given(k).map(|mm| (k, mm)))
            .collect();
        self.open(given.len(), true)?;
        for (kind, mm) in given {
            self.key(kind.key());
            self.at(Seg::Name(kind.key()), |e| e.float(mm))?;
        }
        self.close();
        Ok(())
    }

    /// Schema 85's heights (docs/adr/0519 §1), checked: their rules, a kind a project keeps, an ellipsoidal height
    /// only with a system.
    pub(super) fn heights(
        &mut self,
        s: &ProjectSettings,
        h: &HeightReference,
    ) -> Result<(), KcadError> {
        if let Some((_, why)) = h.problem() {
            return Err(self.fail(Code::BadValue, &why));
        }
        let kind = match h.kind {
            HeightKind::Ellipsoidal => "ellipsoidal",
            HeightKind::Orthometric => "orthometric",
            HeightKind::Unknown => {
                return Err(self.fail(
                    Code::BadValue,
                    "bilinmeyen yükseklik yazılmaz; alan yazılmaz",
                ));
            }
        };
        if h.kind == HeightKind::Ellipsoidal && !s.has_system() {
            return Err(self.fail(
                Code::BadValue,
                "elipsoit yüksekliği bir koordinat sistemi ister; yerel projede yalnız ortometrik yükseklik olur",
            ));
        }
        self.open(
            1 + usize::from(h.geoid.is_some()) + usize::from(h.vertical_srid.is_some()),
            true,
        )?;
        self.key("kind");
        self.w.text(kind);
        if let Some(g) = &h.geoid {
            self.key("geoid");
            self.at(Seg::Name("geoid"), |e| {
                e.open(1 + usize::from(g.sha256.is_some()), true)?;
                e.key("name");
                e.text(&g.name)?;
                if let Some(sha) = &g.sha256 {
                    e.key("sha256");
                    e.text(sha)?;
                }
                Ok(())
            })?;
        }
        if let Some(v) = h.vertical_srid {
            self.key("verticalSrid");
            self.w.uint(u64::from(v));
        }
        Ok(())
    }

    pub(super) fn survey(&mut self, v: &SurveySettings) -> Result<(), KcadError> {
        if let Some(problem) = v.problem() {
            return Err(self.fail(Code::BadValue, &problem));
        }
        // Floats and the one bool, in the canonical order of their keys.
        let fields = [
            ("index", v.index.map(Ok)),
            ("faceHz", v.face_hz.map(Ok)),
            ("twoWay", v.two_way.map(Ok)),
            ("sigmaPpm", v.sigma_ppm.map(Ok)),
            ("faceSlope", v.face_slope.map(Ok)),
            ("refraction", v.refraction.map(Ok)),
            ("sigmaZenith", v.sigma_zenith.map(Ok)),
            ("groundHeight", v.ground_height.map(Ok)),
            ("reduceToGrid", v.reduce_to_grid.map(Err)),
            ("sigmaDistance", v.sigma_distance.map(Ok)),
            ("traverseAngle", v.traverse_angle.map(Ok)),
            ("traverseCoord", v.traverse_coord.map(Ok)),
            ("sigmaCentering", v.sigma_centering.map(Ok)),
            ("sigmaDirection", v.sigma_direction.map(Ok)),
            ("sigmaLevelling", v.sigma_levelling.map(Ok)),
        ];
        self.open(fields.iter().filter(|(_, x)| x.is_some()).count(), true)?;
        for (key, x) in fields {
            match x {
                Some(Ok(x)) => {
                    self.key(key);
                    self.at(Seg::Name(key), |e| e.float(x))?;
                }
                Some(Err(b)) => {
                    self.key(key);
                    self.w.bool(b);
                }
                None => {}
            }
        }
        self.close();
        Ok(())
    }
}
