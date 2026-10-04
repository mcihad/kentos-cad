//! Schema 13's coordinate system definitions and datum choices in the
//! settings (docs/specs/kcad-v2.md §6.4.1, docs/adr/0168): maps whose keys
//! are fixed, written in their encoded order; a definition or a choice the
//! reader would refuse stops the writer (`kentos_contracts::crs`'s rules).

use kentos_contracts::crs::choices_problem;
use kentos_contracts::{
    Convention, CrsBase, CrsDefinition, CrsPlane, CrsSystem, CustomDatum, DatumTransform,
    GridChoice, Helmert, RegistryDatum,
};

use super::Encoder;
use crate::cbor::Seg;
use crate::error::{Code, KcadError};

pub(super) fn registry(d: RegistryDatum) -> &'static str {
    match d {
        RegistryDatum::Turef => "TUREF",
        RegistryDatum::Ed50 => "ED50",
        RegistryDatum::Wgs84 => "WGS84",
    }
}

impl<'d> Encoder<'d> {
    /// A definition: `name`, `system`.
    pub(super) fn crs_definition(&mut self, d: &'d CrsDefinition) -> Result<(), KcadError> {
        if let Some(problem) = d.problem() {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.definition(d)
    }

    fn definition(&mut self, d: &'d CrsDefinition) -> Result<(), KcadError> {
        self.open(2, true)?;
        self.key("name");
        self.text(&d.name)?;
        self.key("system");
        self.at(Seg::Name("system"), |e| e.crs_system(&d.system))?;
        self.close();
        Ok(())
    }

    fn crs_system(&mut self, s: &'d CrsSystem) -> Result<(), KcadError> {
        match s {
            CrsSystem::Tm(t) => {
                self.open(6 + usize::from(t.latitude_of_origin.is_some()), true)?;
                self.key("kind");
                self.text("tm")?;
                self.datum(t.datum, t.custom_datum.as_deref())?;
                self.key("scaleFactor");
                self.float(t.scale_factor)?;
                self.key("falseEasting");
                self.float(t.false_easting)?;
                self.key("falseNorthing");
                self.float(t.false_northing)?;
                self.key("centralMeridian");
                self.float(t.central_meridian)?;
                if let Some(lat0) = t.latitude_of_origin {
                    self.key("latitudeOfOrigin");
                    self.float(lat0)?;
                }
                self.close();
            }
            CrsSystem::Geographic(g) => {
                self.open(2, true)?;
                self.key("kind");
                self.text("geographic")?;
                self.datum(g.datum, g.custom_datum.as_deref())?;
                self.close();
            }
            CrsSystem::Local(l) => {
                self.open(3, true)?;
                self.key("base");
                self.at(Seg::Name("base"), |e| e.crs_base(&l.base))?;
                self.key("kind");
                self.text("local")?;
                self.key("plane");
                self.at(Seg::Name("plane"), |e| e.crs_plane(&l.plane))?;
                self.close();
            }
        }
        Ok(())
    }

    /// A definition's datum: the registry's name (`datum`) or the project's
    /// own (`customDatum`); the definition's rules made sure there is one.
    fn datum(
        &mut self,
        registry_datum: Option<RegistryDatum>,
        custom: Option<&'d CustomDatum>,
    ) -> Result<(), KcadError> {
        match (registry_datum, custom) {
            (Some(r), _) => {
                self.key("datum");
                self.text(registry(r))
            }
            (None, Some(c)) => {
                self.key("customDatum");
                self.at(Seg::Name("customDatum"), |e| e.custom_datum(c))
            }
            (None, None) => Err(self.fail(Code::BadValue, "datum yok")),
        }
    }

    fn custom_datum(&mut self, c: &'d CustomDatum) -> Result<(), KcadError> {
        self.open(2 + usize::from(c.to_wgs84.is_some()), true)?;
        self.key("name");
        self.text(&c.name)?;
        if let Some(h) = &c.to_wgs84 {
            self.key("toWgs84");
            self.at(Seg::Name("toWgs84"), |e| e.helmert(h))?;
        }
        self.key("ellipsoid");
        self.at(Seg::Name("ellipsoid"), |e| {
            e.open(3, true)?;
            e.key("name");
            e.text(&c.ellipsoid.name)?;
            e.key("semiMajor");
            e.float(c.ellipsoid.semi_major)?;
            e.key("inverseFlattening");
            e.float(c.ellipsoid.inverse_flattening)?;
            e.close();
            Ok(())
        })?;
        self.close();
        Ok(())
    }

    fn helmert(&mut self, h: &'d Helmert) -> Result<(), KcadError> {
        self.open(4 + usize::from(h.accuracy.is_some()), true)?;
        self.key("scale");
        self.float(h.scale)?;
        if let Some(a) = h.accuracy {
            self.key("accuracy");
            self.float(a)?;
        }
        self.key("rotation");
        self.at(Seg::Name("rotation"), |e| e.floats(&h.rotation))?;
        self.key("convention");
        self.text(match h.convention {
            Convention::PositionVector => "positionVector",
            Convention::CoordinateFrame => "coordinateFrame",
        })?;
        self.key("translation");
        self.at(Seg::Name("translation"), |e| e.floats(&h.translation))?;
        self.close();
        Ok(())
    }

    fn crs_base(&mut self, b: &'d CrsBase) -> Result<(), KcadError> {
        self.open(1, true)?;
        match (&b.srid, &b.definition) {
            (Some(srid), _) => {
                self.key("srid");
                self.w.uint(u64::from(*srid));
            }
            (None, Some(d)) => {
                self.key("definition");
                self.at(Seg::Name("definition"), |e| e.definition(d))?;
            }
            (None, None) => return Err(self.fail(Code::BadValue, "taban yok")),
        }
        self.close();
        Ok(())
    }

    fn crs_plane(&mut self, p: &'d CrsPlane) -> Result<(), KcadError> {
        match *p {
            CrsPlane::Similarity {
                east,
                north,
                rotation,
                scale,
            } => {
                self.open(5, true)?;
                self.key("east");
                self.float(east)?;
                self.key("kind");
                self.text("similarity")?;
                self.key("north");
                self.float(north)?;
                self.key("scale");
                self.float(scale)?;
                self.key("rotation");
                self.float(rotation)?;
            }
            CrsPlane::Affine { a, b, c, d, e, f } => {
                self.open(7, true)?;
                for (k, v) in [("a", a), ("b", b), ("c", c), ("d", d), ("e", e), ("f", f)] {
                    self.key(k);
                    self.float(v)?;
                }
                self.key("kind");
                self.text("affine")?;
            }
        }
        self.close();
        Ok(())
    }

    /// The datum choices: each pair at most once (§6.4.1).
    pub(super) fn datum_transforms(&mut self, list: &'d [DatumTransform]) -> Result<(), KcadError> {
        if let Some(problem) = choices_problem(list) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(list.len(), false)?;
        for (i, t) in list.iter().enumerate() {
            self.at(Seg::Index(i), |e| e.datum_transform(t))?;
        }
        self.close();
        Ok(())
    }

    fn datum_transform(&mut self, t: &'d DatumTransform) -> Result<(), KcadError> {
        self.open(4, true)?;
        self.key("to");
        self.text(registry(t.to))?;
        self.key("from");
        self.text(registry(t.from))?;
        match (&t.helmert, &t.grid) {
            (None, Some(g)) => {
                self.key("grid");
                self.at(Seg::Name("grid"), |e| e.grid_choice(g))?;
                self.key("name");
                self.text(&t.name)?;
            }
            (Some(h), None) => {
                self.key("name");
                self.text(&t.name)?;
                self.key("helmert");
                self.at(Seg::Name("helmert"), |e| e.helmert(h))?;
            }
            _ => return Err(self.fail(Code::BadValue, "yedi parametre ya da ızgara, yalnız biri")),
        }
        self.close();
        Ok(())
    }

    fn grid_choice(&mut self, g: &'d GridChoice) -> Result<(), KcadError> {
        self.open(3 + usize::from(g.accuracy.is_some()), true)?;
        self.key("id");
        self.text(&g.id)?;
        self.key("file");
        self.text(&g.file)?;
        self.key("size");
        self.w.uint(g.size);
        if let Some(a) = g.accuracy {
            self.key("accuracy");
            self.float(a)?;
        }
        self.close();
        Ok(())
    }
}
