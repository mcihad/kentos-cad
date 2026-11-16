//! The project's coordinate reference system as Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması
//! Hakkında Yönetmelik EK-7 asks it (Tablo 7.6 `IfcProjectedCRS`, Tablo 7.7 `IfcMapConversion`; docs/adr/8047 §8): the
//! system and the conversion of the model's coordinates into it. TUREF's 3° Transverse Mercator zones (EPSG:5253 …
//! 5259, their central meridians 27° … 45°; ITRF96's realisation in Türkiye on the GRS80 ellipsoid) are named in full,
//! another system by its name. Every IFC file KentOS writes (the building's, the road's alignments) says its system
//! through here.
//!
//! A model in a local system stands on its base by the local plane (docs/adr/8072 §3): turned counter-clockwise, scaled
//! and shifted, as IFC 4.3's map conversion does it. Its `Scale` converts units only and scales the heights too, so a
//! local system's scale is written as `IfcMapConversionScaled`'s FactorX and FactorY, the heights unscaled.

use crate::math::{cos, hypot, sin};
use crate::step::{Step, real, text};

/// A projected system and where the model's origin stands in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Crs {
    /// Its name: an EPSG code ("EPSG:5254"), or a project's own system's name.
    pub name: String,
    pub description: Option<String>,
    /// The datum its plane coordinates stand on (TUREF).
    pub geodetic_datum: Option<String>,
    /// The datum its heights are measured from ("Antalya height (EPSG:5775)", "ITRF96").
    pub vertical_datum: Option<String>,
    pub map_projection: Option<String>,
    /// A TUREF zone's central meridian, degrees ("30").
    pub map_zone: Option<String>,
    /// The model's origin in it: eastings, northings and orthogonal height, metres.
    pub origin: [f64; 3],
    /// The model's x axis in it, a unit vector east and north (`XAxisAbscissa`, `XAxisOrdinate`): [1, 0] unturned.
    pub x_axis: [f64; 2],
    /// How long the model's metre is in it along the model's x and y axes (`IfcMapConversionScaled` FactorX and
    /// FactorY; heights are not scaled): [1, 1] for none.
    pub factors: [f64; 2],
}

impl Crs {
    /// The model in a local system on this one by a similarity (docs/adr/8072 §3): turned `rotation` degrees
    /// counter-clockwise, scaled by `scale`, its origin shifted to (`east`, `north`); the origin's height kept. Refused,
    /// with the reason, for values that are no transform.
    pub fn turned(mut self, east: f64, north: f64, rotation: f64, scale: f64) -> Result<Crs, String> {
        if ![east, north, rotation, scale].iter().all(|v| v.is_finite()) {
            return Err("dönüşümün değerleri sonlu sayılar değil".into());
        }
        if scale <= 0.0 {
            return Err(format!("dönüşümün ölçeği {scale}; sıfırdan büyük olmalı"));
        }
        let th = rotation * (std::f64::consts::PI / 180.0);
        self.origin = [east, north, self.origin[2]];
        self.x_axis = [cos(th), sin(th)];
        self.factors = [scale, scale];
        Ok(self)
    }

    /// The model in a local system on this one by an affine transform `[a, b, c, d, e, f]` (x = a·x' + b·y' + c,
    /// y = d·x' + e·y' + f): its origin at (c, f), its x axis along (a, d), its metres the columns' lengths. A map
    /// conversion turns, scales each axis and shifts; an affine transform that shears or mirrors the plane is refused,
    /// with the reason (no similarity is fitted in its place).
    pub fn on_plane(mut self, [a, b, c, d, e, f]: [f64; 6]) -> Result<Crs, String> {
        if ![a, b, c, d, e, f].iter().all(|v| v.is_finite()) {
            return Err("dönüşümün katsayıları sonlu sayılar değil".into());
        }
        let (fx, fy) = (hypot(a, d), hypot(b, e));
        if !(fx > 0.0 && fy > 0.0) || a * e - b * d == 0.0 {
            return Err("dönüşüm düzlemi bir doğruya indiriyor".into());
        }
        if a * e - b * d < 0.0 {
            return Err("dönüşüm düzlemi aynalıyor".into());
        }
        // The columns square to each other: a shear's cosine within the arithmetic's rounding.
        if (a * b + d * e).abs() > 1e-12 * fx * fy {
            return Err("dönüşüm eksenleri dik tutmuyor (kayma var)".into());
        }
        self.origin = [c, f, self.origin[2]];
        self.x_axis = [a / fx, d / fx];
        self.factors = [fx, fy];
        Ok(self)
    }
}

/// The central meridian of TUREF's 3° zone an EPSG code names, degrees: EPSG:5253 27 … EPSG:5259 45 (EK-7 Tablo 7.6).
pub fn turef_meridian(code: &str) -> Option<u32> {
    let n: u32 = code.strip_prefix("EPSG:")?.parse().ok()?;
    (5253..=5259).contains(&n).then(|| 27 + 3 * (n - 5253))
}

/// The system `name` ("EPSG:5254", or a project's own) with its description, the datum its heights are measured from
/// and the model's origin in it: a TUREF zone with its datum, its projection and its zone (EK-7 Tablo 7.6), another
/// system by its name.
pub fn projected(
    name: &str,
    description: Option<String>,
    vertical_datum: Option<String>,
    origin: [f64; 3],
) -> Crs {
    let meridian = turef_meridian(name);
    Crs {
        name: name.to_owned(),
        description,
        geodetic_datum: meridian.map(|_| "TUREF".to_owned()),
        vertical_datum,
        map_projection: meridian.map(|_| "Transverse Mercator".to_owned()),
        map_zone: meridian.map(|m| m.to_string()),
        origin,
        x_axis: [1.0, 0.0],
        factors: [1.0, 1.0],
    }
}

/// Writes `crs`: its `IfcProjectedCRS` (its unit the file's metre, `length`) and the map conversion of the
/// representation context `context` into it: `IfcMapConversion` turned and shifted, `IfcMapConversionScaled` when the
/// model's metre is scaled; its `Scale` 1, the units the same.
pub(crate) fn write(st: &mut Step, crs: &Crs, context: usize, length: usize) {
    let opt = |s: &Option<String>| s.as_deref().map_or_else(|| "$".to_owned(), text);
    let c = st.add(format!(
        "IFCPROJECTEDCRS({},{},{},{},{},{},#{length})",
        text(&crs.name),
        opt(&crs.description),
        opt(&crs.geodetic_datum),
        opt(&crs.vertical_datum),
        opt(&crs.map_projection),
        opt(&crs.map_zone)
    ));
    let [e, n, h] = crs.origin;
    let [xa, xo] = crs.x_axis;
    let head = format!(
        "#{context},#{c},{},{},{},{},{},1.",
        real(e),
        real(n),
        real(h),
        real(xa),
        real(xo)
    );
    if crs.factors == [1.0, 1.0] {
        st.add(format!("IFCMAPCONVERSION({head})"));
    } else {
        let [fx, fy] = crs.factors;
        st.add(format!(
            "IFCMAPCONVERSIONSCALED({head},{},{},1.)",
            real(fx),
            real(fy)
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turef_s_zones_are_named_in_full_and_another_system_by_its_name() {
        assert_eq!(turef_meridian("EPSG:5253"), Some(27));
        assert_eq!(turef_meridian("EPSG:5254"), Some(30));
        assert_eq!(turef_meridian("EPSG:5259"), Some(45));
        assert_eq!(turef_meridian("EPSG:5260"), None);
        assert_eq!(turef_meridian("EPSG:32636"), None);
        let tm30 = projected(
            "EPSG:5254",
            Some("TUREF / TM30".into()),
            None,
            [0.0, 0.0, 1285.4],
        );
        assert_eq!(
            (
                tm30.geodetic_datum.as_deref(),
                tm30.map_projection.as_deref(),
                tm30.map_zone.as_deref()
            ),
            (Some("TUREF"), Some("Transverse Mercator"), Some("30"))
        );
        let utm = projected("EPSG:32636", None, None, [0.0; 3]);
        assert_eq!(
            (utm.geodetic_datum, utm.map_projection, utm.map_zone),
            (None, None, None)
        );
    }

    #[test]
    fn it_is_written_as_a_projected_system_and_its_conversion() {
        let mut st = Step::new("t".into());
        let length = st.add("IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)");
        let model = st.add("IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,$,$)");
        write(
            &mut st,
            &projected(
                "EPSG:5255",
                Some("TUREF / TM33".into()),
                Some("Antalya height (EPSG:5775)".into()),
                [0.0, 0.0, 1285.4],
            ),
            model,
            length,
        );
        let text = st.file("v", "t.ifc", "2026-11-11T12:00:00", "KentOS");
        assert!(text.contains(
            "IFCPROJECTEDCRS('EPSG:5255','TUREF / TM33','TUREF','Antalya height (EPSG:5775)','Transverse Mercator','33',#1)"
        ));
        assert!(text.contains("IFCMAPCONVERSION(#2,#3,0.,0.,1285.4,1.,0.,1.)"));
    }

    /// Where IFC 4.3's map conversion puts a model point (`IfcMapConversion.md`: the axes scaled, turned
    /// counter-clockwise by atan(XAxisOrdinate / XAxisAbscissa), shifted; `IfcMapConversionScaled` each axis by its
    /// factor).
    fn mapped(c: &Crs, [x, y]: [f64; 2]) -> [f64; 2] {
        // The angle's cosine and sine are the axis vector's, normalised.
        let r = hypot(c.x_axis[0], c.x_axis[1]);
        let (ct, st) = (c.x_axis[0] / r, c.x_axis[1] / r);
        let (fx, fy) = (c.factors[0] * x, c.factors[1] * y);
        [c.origin[0] + ct * fx - st * fy, c.origin[1] + st * fx + ct * fy]
    }

    /// A local system turned 23.5° and scaled 0.99989 onto TUREF / TM36: the conversion is the scaled one, its axis the
    /// turn's, its factors the scale, its height the origin's; three points land where the project's own plane transform
    /// puts them (docs/adr/8072 §3).
    #[test]
    fn a_local_system_is_turned_scaled_and_shifted_onto_its_base() {
        let base = projected("EPSG:5256", Some("TUREF / TM36".into()), None, [0.0, 0.0, 1285.4]);
        let c = base.clone().turned(587_312.345, 4_400_612.789, 23.5, 0.99989).expect("a similarity");
        let plane = kentos_geometry_core::crs::Plane::Similarity {
            east: 587_312.345,
            north: 4_400_612.789,
            rotation: 23.5,
            scale: 0.99989,
        };
        for p in [[0.0, 0.0], [60.0, 5.0], [10.0, 45.0]] {
            let want = plane.forward(kentos_geometry_core::Vec2::new(p[0], p[1]));
            let got = mapped(&c, p);
            assert!((got[0] - want.x).abs() < 1e-8 && (got[1] - want.y).abs() < 1e-8, "{p:?}: {got:?} {want:?}");
        }
        let mut st = Step::new("t".into());
        let length = st.add("IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)");
        let model = st.add("IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,$,$)");
        write(&mut st, &c, model, length);
        let text = st.file("v", "t.ifc", "2026-11-16T12:00:00", "KentOS");
        let th = 23.5_f64.to_radians();
        assert!(
            text.contains(&format!(
                "IFCMAPCONVERSIONSCALED(#2,#3,587312.345,4400612.789,1285.4,{},{},1.,0.99989,0.99989,1.)",
                real(cos(th)),
                real(sin(th))
            )),
            "{text}"
        );
        // Unturned and unscaled: the plain conversion.
        let plain = base.turned(0.0, 0.0, 0.0, 1.0).expect("the identity");
        assert_eq!((plain.x_axis, plain.factors), ([1.0, 0.0], [1.0, 1.0]));
        assert!(base_text(&plain).contains("IFCMAPCONVERSION(#2,#3,0.,0.,1285.4,1.,0.,1.)"));
        assert!(projected("EPSG:5256", None, None, [0.0; 3]).turned(0.0, 0.0, 10.0, 0.0).is_err());
    }

    fn base_text(c: &Crs) -> String {
        let mut st = Step::new("t".into());
        let length = st.add("IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)");
        let model = st.add("IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,$,$)");
        write(&mut st, c, model, length);
        st.file("v", "t.ifc", "2026-11-16T12:00:00", "KentOS")
    }

    /// An affine transform with square axes is written with a factor for each; one that shears, mirrors or flattens the
    /// plane is refused with its reason.
    #[test]
    fn an_affine_transform_is_written_only_with_square_axes() {
        let base = projected("EPSG:5256", None, None, [0.0, 0.0, 0.0]);
        // Turned 90°: x along north, 2 m a metre; y along west, 3 m a metre.
        let c = base.clone().on_plane([0.0, -3.0, 500.0, 2.0, 0.0, 700.0]).expect("square axes");
        assert_eq!((c.origin, c.x_axis, c.factors), ([500.0, 700.0, 0.0], [0.0, 1.0], [2.0, 3.0]));
        assert_eq!(mapped(&c, [1.0, 1.0]).map(|v| (v * 1e9).round() / 1e9), [497.0, 702.0]);
        assert!(base_text(&c).contains("IFCMAPCONVERSIONSCALED(#2,#3,500.,700.,0.,0.,1.,1.,2.,3.,1.)"));
        let why = |p| base.clone().on_plane(p).expect_err("refused");
        assert!(why([1.0, 0.1, 0.0, 0.0, 1.0, 0.0]).contains("kayma"));
        assert!(why([1.0, 0.0, 0.0, 0.0, -1.0, 0.0]).contains("aynalıyor"));
        assert!(why([1.0, 2.0, 0.0, 2.0, 4.0, 0.0]).contains("doğruya"));
        assert!(why([f64::NAN, 0.0, 0.0, 0.0, 1.0, 0.0]).contains("sonlu"));
    }
}
