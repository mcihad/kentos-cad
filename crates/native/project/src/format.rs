//! Numbers as the user reads them, from the project's settings: the web's
//! `Formatter` (`apps/web/src/app/format.ts`). Display only (CLAUDE.md §5,
//! §23.2): nothing here rounds a coordinate that is kept. The decimal
//! separator is the point, as typed input takes it.

use kentos_contracts::{AngleUnit, AreaUnit, DrawingUnit, ProjectSettings, Workspace};
use kentos_geometry_core::tools::point_input::{AngleFrom, Angles};

use kentos_geometry_core::Vec2;
use kentos_geometry_core::tools::imperial::{Style, format_inches};

/// Grads per degree.
const GRAD_PER_DEG: f64 = 400.0 / 360.0;

/// The unit settings formatting depends on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Format {
    pub length_decimals: usize,
    pub area_decimals: usize,
    pub area_unit: AreaUnit,
    pub angle_unit: AngleUnit,
    /// The unit lengths are typed and read in: metres, or a local project's
    /// drawing unit (docs/adr/0165 §2). Geometry stays in metres.
    pub unit: DrawingUnit,
    /// The project's axes and the way its angles run (docs/adr/0165 §4).
    pub axes: Axes,
    /// The drawing's user coordinate system, its origin's x and y and its
    /// angle (docs/adr/0261): where the status bar reads the pointer and how
    /// the axes' icon turns. Points are given in the world all the same.
    pub ucs: Option<[f64; 3]>,
    /// How lengths and coordinates are written (docs/adr/0262 §2): decimal
    /// in the unit, or feet and inches with their marks.
    pub style: Style,
}

/// A project's axes and the way its angles run (docs/adr/0165 §4): a CBS
/// project's Y east and X north, its directions bearings (semt) from north,
/// clockwise; a CAD project's X east and Y north, its angles from east,
/// counter-clockwise. A project not asked its type is shown as CBS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Axes {
    #[default]
    Gis,
    Cad,
}

impl Default for Format {
    /// A new project's settings (web: `DEFAULT_PROJECT_SETTINGS`).
    fn default() -> Self {
        Self {
            length_decimals: 3,
            area_decimals: 2,
            area_unit: AreaUnit::M2,
            angle_unit: AngleUnit::Grad,
            unit: DrawingUnit::M,
            axes: Axes::Gis,
            ucs: None,
            style: Style::Decimal,
        }
    }
}

impl Format {
    pub fn of(settings: &ProjectSettings) -> Self {
        Self {
            length_decimals: (settings.length_decimals as usize).min(12),
            area_decimals: (settings.area_decimals as usize).min(12),
            area_unit: settings.area_unit,
            angle_unit: settings.angle_unit,
            unit: settings.unit(),
            axes: if settings.project_type() == Some(Workspace::Cad) {
                Axes::Cad
            } else {
                Axes::Gis
            },
            ucs: settings.ucs.map(|u| [u.origin.x, u.origin.y, u.angle]),
            style: match settings.length_style {
                Some(kentos_contracts::LengthStyle::Architectural)
                    if settings.unit().imperial() =>
                {
                    Style::Architectural
                }
                Some(kentos_contracts::LengthStyle::Engineering) if settings.unit().imperial() => {
                    Style::Engineering
                }
                Some(kentos_contracts::LengthStyle::Fractional) if settings.unit().imperial() => {
                    Style::Fractional
                }
                _ => Style::Decimal,
            },
        }
    }

    /// A world point in the drawing's UCS (docs/adr/0261 §4); itself in the world's.
    pub fn in_ucs(&self, p: Vec2) -> Vec2 {
        match self.ucs {
            Some([ox, oy, a]) => {
                let (c, s) = (
                    kentos_geometry_core::jsmath::cos(a),
                    kentos_geometry_core::jsmath::sin(a),
                );
                let (dx, dy) = (p.x - ox, p.y - oy);
                Vec2::new(dx * c + dy * s, -dx * s + dy * c)
            }
            None => p,
        }
    }

    /// How a typed polar angle runs and its unit: the project's type and
    /// angle unit (docs/adr/0165 §4).
    pub fn angles(&self) -> Angles {
        Angles {
            from: match self.axes {
                Axes::Cad => AngleFrom::East,
                Axes::Gis => AngleFrom::North,
            },
            grads: self.angle_unit == AngleUnit::Grad,
        }
    }

    /// The same settings in the surveyor's terms: the Hesap windows' (field
    /// measurements are metres whatever a local project's unit, read as Y, X
    /// and semt whatever its type; docs/adr/0165 §2, §4).
    pub fn metric(self) -> Self {
        Self {
            unit: DrawingUnit::M,
            axes: Axes::Gis,
            ..self
        }
    }

    /// The east axis's name: Y in a CBS project, X in a CAD one (docs/adr/0165 §4).
    pub fn east_label(&self) -> &'static str {
        match self.axes {
            Axes::Gis => "Y",
            Axes::Cad => "X",
        }
    }

    /// The north axis's name: X in a CBS project, Y in a CAD one.
    pub fn north_label(&self) -> &'static str {
        match self.axes {
            Axes::Gis => "X",
            Axes::Cad => "Y",
        }
    }

    /// A point as it is typed: `Y,X` in a CBS project, `X,Y` in a CAD one;
    /// east first in both.
    pub fn pair_label(&self) -> String {
        format!("{},{}", self.east_label(), self.north_label())
    }

    /// A relative point as it is typed: `@dY,dX` or `@dX,dY`.
    pub fn relative_label(&self) -> String {
        format!("@d{},d{}", self.east_label(), self.north_label())
    }

    /// A polar point as it is typed: a CBS project's `@mesafe<semt`, a CAD
    /// project's `@mesafe<açı`.
    pub fn polar_label(&self) -> &'static str {
        match self.axes {
            Axes::Gis => "@mesafe<semt",
            Axes::Cad => "@mesafe<açı",
        }
    }

    /// The value field's hint (the web's `CursorInput`): “mesafe · Y,X ·
    /// @dY,dX · @mesafe<semt” in the project's axes.
    pub fn input_hint(&self) -> String {
        format!(
            "mesafe · {} · {} · {}",
            self.pair_label(),
            self.relative_label(),
            self.polar_label()
        )
    }

    /// Words written in a CBS project's terms (`Y,X`, `@dY,dX`, `Y (sağa)`,
    /// `X (yukarı)`) in the project's axes: a tool's steps, a column's
    /// heading (the web's `axesText`).
    pub fn axes_text(&self, text: &str) -> String {
        if self.axes == Axes::Gis {
            return text.to_owned();
        }
        text.replace("dY,dX", "dX,dY")
            .replace("Y,X", "X,Y")
            .replace("Y (sağa)", "X (sağa)")
            .replace("X (yukarı)", "Y (yukarı)")
    }

    /// A length or coordinate typed in the unit, in metres (what the geometry
    /// keeps): the unit's ratio of whole numbers, as one product and one
    /// division (an inch × 254 ÷ 10000; a millimetre ÷ 1000, docs/adr/0262 §1).
    pub fn to_metres(&self, typed: f64) -> f64 {
        let (mul, div) = self.unit.metres();
        if mul == 1.0 {
            typed / div
        } else {
            typed * mul / div
        }
    }

    /// An area typed in the project's area unit (m², dönüm, ha; a local
    /// project's mm² or cm²), in square metres (the web's `areaToSquareMetres`).
    pub fn area_to_square_metres(&self, typed: f64) -> f64 {
        if self.unit != DrawingUnit::M {
            let (mul, div) = self.unit.metres();
            return typed * (mul * mul) / (div * div);
        }
        match self.area_unit {
            AreaUnit::Donum => typed * 1000.0,
            AreaUnit::Ha => typed * 10_000.0,
            AreaUnit::M2 => typed,
        }
    }

    /// A length or coordinate in metres, in the unit (a field's starting value).
    pub fn from_metres(&self, metres: f64) -> f64 {
        let (mul, div) = self.unit.metres();
        if mul == 1.0 {
            metres * div
        } else {
            metres * div / mul
        }
    }

    /// A length in metres as inches, for the feet-and-inches styles (docs/adr/0262 §2).
    fn inches(metres: f64) -> f64 {
        metres * 10_000.0 / 254.0
    }

    /// A length in the project's style: feet and inches with their marks, or
    /// decimal in the unit without it.
    fn styled(&self, metres: f64) -> String {
        match self.style {
            Style::Decimal => fixed(self.from_metres(metres), self.length_decimals),
            style => format_inches(Self::inches(metres), style, self.length_decimals as u32),
        }
    }

    /// A length in metres in the unit with the digits it needs, not rounded to
    /// the length decimals: a limit in a message, a kept value in a prompt
    /// (`0.000001` m, `0.001` mm). Without the unit; the web's `plain`.
    pub fn plain(&self, metres: f64) -> String {
        let v = self.from_metres(metres);
        js_number(format!("{v:.14e}").parse().unwrap_or(v))
    }

    /// The length unit in words, for prompts (the web's `lengthUnitName`):
    /// `metre`, or a local project's `milimetre` or `santimetre`.
    pub fn length_unit_name(&self) -> &'static str {
        match self.unit {
            DrawingUnit::Mm => "milimetre",
            DrawingUnit::Cm => "santimetre",
            DrawingUnit::M => "metre",
            DrawingUnit::In => "inç",
            DrawingUnit::Ft => "ayak",
        }
    }

    /// The length unit's mark (the web's `lengthUnitLabel`): `m`, or a local project's `mm` or `cm`.
    pub fn length_unit_label(&self) -> &'static str {
        self.unit.mark()
    }

    /// A grid coordinate (Y or X) without a unit.
    pub fn coord(&self, v: f64) -> String {
        self.styled(v)
    }

    /// A length with its unit; feet and inches carry their own marks (`12'-6 1/2"`).
    pub fn length(&self, metres: f64) -> String {
        match self.style {
            Style::Decimal => format!("{} {}", self.styled(metres), self.unit.mark()),
            _ => self.styled(metres),
        }
    }

    /// A length without its unit (the web's `length(m, false)`): `20.000 × 10.000 m`.
    pub fn length_bare(&self, metres: f64) -> String {
        self.styled(metres)
    }

    /// An area in the unit squared, for a local project in millimetres or centimetres.
    fn square_units(&self, square_metres: f64) -> Option<String> {
        let (mul, div) = self.unit.metres();
        (self.unit != DrawingUnit::M).then(|| {
            fixed(
                square_metres * (div * div) / (mul * mul),
                self.area_decimals,
            )
        })
    }

    /// An area in the project's unit without the unit (the web's `area(m2, false)`).
    pub fn area_bare(&self, square_metres: f64) -> String {
        if let Some(s) = self.square_units(square_metres) {
            return s;
        }
        let d = self.area_decimals;
        match self.area_unit {
            AreaUnit::Donum => fixed(square_metres / 1000.0, d),
            AreaUnit::Ha => fixed(square_metres / 10_000.0, d),
            AreaUnit::M2 => fixed(square_metres, d),
        }
    }

    /// The area unit's name (the web's `areaUnitLabel`): `m²`, `dönüm` or `ha`;
    /// a local project's `mm²` or `cm²`.
    pub fn area_unit_label(&self) -> &'static str {
        match self.unit {
            DrawingUnit::Mm => return "mm²",
            DrawingUnit::Cm => return "cm²",
            DrawingUnit::In => return "in²",
            DrawingUnit::Ft => return "ft²",
            DrawingUnit::M => {}
        }
        match self.area_unit {
            AreaUnit::Donum => "dönüm",
            AreaUnit::Ha => "ha",
            AreaUnit::M2 => "m²",
        }
    }

    pub fn area(&self, square_metres: f64) -> String {
        if let Some(s) = self.square_units(square_metres) {
            return format!("{s} {}", self.area_unit_label());
        }
        let d = self.area_decimals;
        match self.area_unit {
            AreaUnit::Donum => format!("{} dönüm", fixed(square_metres / 1000.0, d)),
            AreaUnit::Ha => format!("{} ha", fixed(square_metres / 10_000.0, d)),
            AreaUnit::M2 => format!("{} m²", fixed(square_metres, d)),
        }
    }

    /// A bearing given in grads (the surveying semt), in the project's angle unit.
    pub fn bearing(&self, grad: f64) -> String {
        match self.angle_unit {
            AngleUnit::Deg => format!("{}°", fixed(grad / GRAD_PER_DEG, 4)),
            AngleUnit::Grad => format!("{} g", fixed(grad, 4)),
        }
    }

    /// A bearing without its unit (the web's `bearing(g, false)`), for a
    /// column whose heading names the unit ([`Format::angle_unit_label`]).
    pub fn bearing_bare(&self, grad: f64) -> String {
        match self.angle_unit {
            AngleUnit::Deg => fixed(grad / GRAD_PER_DEG, 4),
            AngleUnit::Grad => fixed(grad, 4),
        }
    }

    /// A direction's reading's name: a CBS project's `Semt`, a CAD project's
    /// `Açı` (docs/adr/0165 §4).
    pub fn direction_name(&self) -> &'static str {
        match self.axes {
            Axes::Gis => "Semt",
            Axes::Cad => "Açı",
        }
    }

    /// A direction given as its semt in grads, as the project's type reads
    /// it: the semt, or a CAD project's angle from east, counter-clockwise
    /// (100 − semt grads), in the project's angle unit (the web's `direction`).
    pub fn direction(&self, semt_grad: f64) -> String {
        self.bearing(self.direction_grads(semt_grad))
    }

    /// The same without its unit, for a column whose heading names it.
    pub fn direction_bare(&self, semt_grad: f64) -> String {
        self.bearing_bare(self.direction_grads(semt_grad))
    }

    fn direction_grads(&self, semt_grad: f64) -> f64 {
        if self.axes == Axes::Gis {
            return semt_grad;
        }
        let mut g = 100.0 - semt_grad;
        if g < 0.0 {
            g += 400.0;
        }
        if g >= 400.0 {
            g -= 400.0;
        }
        g
    }

    /// The angle unit's mark (the web's `angleUnitLabel`): `°` or `g`.
    pub fn angle_unit_label(&self) -> &'static str {
        match self.angle_unit {
            AngleUnit::Deg => "°",
            AngleUnit::Grad => "g",
        }
    }

    /// An angle in radians (an angular dimension's), in the project's angle
    /// unit: the web's `angle`, a bearing of `rad × 200 / π` grads.
    pub fn angle(&self, rad: f64) -> String {
        self.bearing((rad * 200.0) / std::f64::consts::PI)
    }

    /// An angle typed in the project's angle unit (a direction, a bearing), in radians.
    pub fn angle_from_typed(&self, typed: f64) -> f64 {
        match self.angle_unit {
            AngleUnit::Deg => (typed * std::f64::consts::PI) / 180.0,
            AngleUnit::Grad => (typed * std::f64::consts::PI) / 200.0,
        }
    }

    /// A turn typed in the project's angle unit, in radians counter-clockwise: as typed in a CAD project,
    /// clockwise in a CBS one, its directions running that way (docs/adr/0291, as Görünümü döndür's and UCS's).
    pub fn turn_from_typed(&self, typed: f64) -> f64 {
        let rad = self.angle_from_typed(typed);
        match self.axes {
            Axes::Cad => rad,
            Axes::Gis => -rad,
        }
    }

    /// A direction typed in the project's way, as a geometry angle in radians (counter-clockwise from east): a
    /// CAD project's from east counter-clockwise, a CBS project's a semt, from north clockwise (docs/adr/0291).
    pub fn direction_from_typed(&self, typed: f64) -> f64 {
        let rad = self.angle_from_typed(typed);
        match self.axes {
            Axes::Cad => rad,
            Axes::Gis => std::f64::consts::FRAC_PI_2 - rad,
        }
    }

    /// A turn (radians counter-clockwise) as the project reads it, four decimals and the unit: degrees or grads,
    /// counter-clockwise in a CAD project, clockwise in a CBS one (docs/adr/0291).
    pub fn turn(&self, rad: f64) -> String {
        let way = match self.axes {
            Axes::Cad => rad,
            Axes::Gis => -rad,
        };
        let v = match self.angle_unit {
            AngleUnit::Deg => (way * 180.0) / std::f64::consts::PI,
            AngleUnit::Grad => (way * 200.0) / std::f64::consts::PI,
        };
        format!("{}{}", fixed(v, 4), self.angle_unit_label())
    }

    /// How a typed turn runs, as a prompt says it: “derece, saat yönü tersine” or “grad, saat yönünde”.
    pub fn turn_words(&self) -> String {
        let way = match self.axes {
            Axes::Cad => "saat yönü tersine",
            Axes::Gis => "saat yönünde",
        };
        format!("{}, {way}", self.angle_unit_name())
    }

    /// The project's angle unit as a prompt says it: `derece` or `grad`.
    pub fn angle_unit_name(&self) -> &'static str {
        match self.angle_unit {
            AngleUnit::Deg => "derece",
            AngleUnit::Grad => "grad",
        }
    }

    /// The same without its unit (the web's `angle(rad, false)`).
    pub fn angle_bare(&self, rad: f64) -> String {
        self.bearing_bare((rad * 200.0) / std::f64::consts::PI)
    }

    /// A slope in percent, two decimals (docs/adr/0147 §2; the web's `percent`).
    pub fn percent(&self, v: f64) -> String {
        fixed(v, 2)
    }

    /// A dimension's measured value as drawn (the web's `dimensionText`): its
    /// prefix (“R ”, “Ø ”, “Y=”, “X=”, “t=”, “%”), then a length or a
    /// coordinate without its unit, an angle in the project's angle unit or a
    /// percentage.
    pub fn dimension(&self, prefix: &str, unit: &str, value: f64) -> String {
        self.dimension_in(
            prefix,
            unit,
            value,
            &kentos_contracts::DimensionLook::default(),
        )
    }

    /// A dimension's value as written in its look (docs/adr/0183 §3): the
    /// look's prefix, the kind's prefix, the number, the look's suffix; a
    /// length or a coordinate in the look's unit and decimals (else the
    /// project's) without the unit, a slope's percentage with the look's
    /// decimals (else 2), an angle in the project's angle unit. The web's
    /// `dimensionValue`; both pass the `dimensionValue` cases of
    /// fixtures/text/v1/styles.json.
    pub fn dimension_in(
        &self,
        prefix: &str,
        unit: &str,
        value: f64,
        look: &kentos_contracts::DimensionLook,
    ) -> String {
        if matches!(unit, "length" | "coordinate") {
            return self.dimension_value(prefix, unit, value, look, None);
        }
        let number = match unit {
            "none" => return String::new(),
            "level" => level(value, look),
            "angle" => self.angle(value),
            "percent" => fixed(value, look.decimals.map_or(2, |d| d as usize)),
            _ => {
                let per_metre = look.unit.unwrap_or(self.unit).per_metre();
                let decimals = look.decimals.map_or(self.length_decimals, |d| d as usize);
                fixed(value * per_metre, decimals)
            }
        };
        format!(
            "{}{prefix}{number}{}",
            look.prefix.as_deref().unwrap_or_default(),
            look.suffix.as_deref().unwrap_or_default()
        )
    }

    /// A dimension's value as written (docs/adr/0321 §2, §3; the cases of fixtures/text/v1/dimension-values.json): a
    /// length or a coordinate rounded to its increment, in its unit and style (feet and inches in an inch or foot
    /// unit, else its decimals with its zeros left out), its tolerance, its prefixes and suffix and its alternate unit;
    /// an angle and a slope as `dimension_in` writes them. A text written over it is shown with `<>` as the value; one
    /// without `<>` is shown alone.
    pub fn dimension_value(
        &self,
        prefix: &str,
        unit: &str,
        value: f64,
        look: &kentos_contracts::DimensionLook,
        text: Option<&str>,
    ) -> String {
        let written = if matches!(unit, "length" | "coordinate") {
            self.length_value(prefix, value, look)
        } else {
            self.dimension_in(prefix, unit, value, look)
        };
        match text {
            Some(t) => t.replace("<>", &written),
            None => written,
        }
    }

    fn length_value(
        &self,
        prefix: &str,
        value: f64,
        look: &kentos_contracts::DimensionLook,
    ) -> String {
        use kentos_contracts::ToleranceKind;
        let options = look.options.as_deref().cloned().unwrap_or_default();
        let rounded = match options.round {
            Some(r) => (value / r).round() * r,
            None => value,
        };
        let number =
            |v: f64, decimals: Option<u32>| self.dimension_number(v, look, &options, decimals);
        let (body, tolerance) = match options.tolerance {
            Some(t) if t.kind == ToleranceKind::Limits => (
                format!(
                    "{}/{}",
                    number(rounded + t.upper, t.decimals),
                    number(rounded - t.lower.unwrap_or(0.0), t.decimals)
                ),
                String::new(),
            ),
            Some(t) if t.kind == ToleranceKind::Symmetric => (
                number(rounded, None),
                format!(" ±{}", number(t.upper, t.decimals)),
            ),
            Some(t) => (
                number(rounded, None),
                format!(
                    " +{} -{}",
                    number(t.upper, t.decimals),
                    number(t.lower.unwrap_or(0.0), t.decimals)
                ),
            ),
            None => (number(rounded, None), String::new()),
        };
        let mut out = format!(
            "{}{prefix}{body}{tolerance}{}",
            look.prefix.as_deref().unwrap_or_default(),
            look.suffix.as_deref().unwrap_or_default()
        );
        if let Some(alt) = options.alt {
            let decimals = alt.decimals.map_or(self.length_decimals, |d| d as usize);
            out.push_str(&format!(
                " [{}]",
                fixed(value * alt.unit.per_metre(), decimals)
            ));
        }
        out
    }

    /// A dimension's length in its unit and style: feet and inches with their marks, else decimal with its zeros.
    fn dimension_number(
        &self,
        metres: f64,
        look: &kentos_contracts::DimensionLook,
        options: &kentos_contracts::DimensionOptions,
        decimals: Option<u32>,
    ) -> String {
        use kentos_contracts::{DimensionZeros, DrawingUnit, LengthStyle};
        let unit = look.unit.unwrap_or(self.unit);
        let style = match (options.length_style, look.unit) {
            (Some(LengthStyle::Architectural), _) => Style::Architectural,
            (Some(LengthStyle::Engineering), _) => Style::Engineering,
            (Some(LengthStyle::Fractional), _) => Style::Fractional,
            (Some(LengthStyle::Decimal), _) | (None, Some(_)) => Style::Decimal,
            (None, None) => self.style,
        };
        let d = decimals
            .or(look.decimals)
            .map_or(self.length_decimals, |d| d as usize);
        if matches!(unit, DrawingUnit::In | DrawingUnit::Ft) && style != Style::Decimal {
            return format_inches(Self::inches(metres), style, d as u32);
        }
        let mut text = fixed(metres * unit.per_metre(), d);
        let zeros = options.zeros;
        if matches!(zeros, Some(DimensionZeros::Trailing | DimensionZeros::Both))
            && text.contains('.')
        {
            text = text.trim_end_matches('0').trim_end_matches('.').to_owned();
        }
        if matches!(zeros, Some(DimensionZeros::Leading | DimensionZeros::Both)) {
            if let Some(rest) = text.strip_prefix("0.") {
                text = format!(".{rest}");
            } else if let Some(rest) = text.strip_prefix("-0.") {
                text = format!("-.{rest}");
            }
        }
        text
    }

    /// `Y 487012.000  X 4420000.000`: east first (CLAUDE.md §5), a CAD
    /// project's `X 120.000  Y 45.500` (docs/adr/0165 §4).
    pub fn point(&self, p: Vec2) -> String {
        format!(
            "{} {}  {} {}",
            self.east_label(),
            self.coord(p.x),
            self.north_label(),
            self.coord(p.y)
        )
    }
}

/// A level's elevation (docs/adr/0322): metres rounded to its look's increment, its decimals (absent: 2), signed:
/// `+3.20`, `-1.50`, `±0.00` when the digits shown are all zero.
fn level(metres: f64, look: &kentos_contracts::DimensionLook) -> String {
    let rounded = match look.options.as_deref().and_then(|o| o.round) {
        Some(r) => (metres / r).round() * r,
        None => metres,
    };
    let decimals =
        look.decimals
            .unwrap_or(kentos_geometry_core::geom::dimension::LEVEL_DECIMALS) as usize;
    let digits = fixed(rounded.abs(), decimals);
    let sign = if digits.chars().all(|c| c == '0' || c == '.') {
        "±"
    } else if rounded < 0.0 {
        "-"
    } else {
        "+"
    };
    format!("{sign}{digits}")
}

/// `v` with `d` decimals by the display rule (docs/adr/0149): seven
/// decimals first, then the digits shown, a half away from zero; no sign on
/// zero. The shared core's `display::fixed`, the web's `core/displayNumber.ts`.
pub fn fixed(v: f64, d: usize) -> String {
    kentos_geometry_core::display::fixed(v, d)
}

/// A number as JavaScript writes it (`String(n)`), for the values the tools
/// show after `+x.toFixed(d)`: the shortest digits, no trailing zeros, no
/// sign on zero. Numbers this small in magnitude are never written with an
/// exponent by either.
pub fn js_number(v: f64) -> String {
    if v == 0.0 {
        return "0".to_owned();
    }
    format!("{v}")
}

/// An angle in radians as the rectangle tool writes its rotation (the web's
/// `fmtDeg`: `+((rad / DEG) % 360).toFixed(4)` and a degree sign): `30°`, `12.3457°`.
pub fn short_degrees(rad: f64) -> String {
    let deg = (rad / (kentos_geometry_core::jsmath::PI / 180.0)) % 360.0;
    let rounded = fixed(deg, 4).parse::<f64>().unwrap_or(f64::NAN);
    format!("{}°", js_number(rounded))
}

/// Radians in degrees as the arc tool writes them (the web's `deg`: `rad * 180 / π`).
pub fn degrees(rad: f64) -> f64 {
    (rad * 180.0) / kentos_geometry_core::jsmath::PI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degrees_read_as_the_web_writes_them() {
        use kentos_geometry_core::jsmath::PI;
        assert_eq!(short_degrees(0.0), "0°");
        assert_eq!(short_degrees(PI / 6.0), "30°");
        assert_eq!(short_degrees(-PI / 2.0), "-90°");
        assert_eq!(short_degrees(1.0), "57.2958°");
        // A tiny negative angle rounds to zero without its sign.
        assert_eq!(short_degrees(-1e-9), "0°");
        assert_eq!(fixed(degrees(PI / 2.0), 4), "90.0000");
    }

    #[test]
    fn numbers_read_as_on_the_web() {
        let f = Format::default();
        assert_eq!(
            f.point(Vec2::new(487012.0, 4420000.0)),
            "Y 487012.000  X 4420000.000"
        );
        assert_eq!(f.length(12.0), "12.000 m");
        assert_eq!(f.area(96.0), "96.00 m²");
        assert_eq!(f.bearing(100.0), "100.0000 g");
        assert_eq!(f.coord(-0.0), "0.000");
        let other = Format {
            area_unit: AreaUnit::Donum,
            angle_unit: AngleUnit::Deg,
            ..f
        };
        assert_eq!(other.area(1500.0), "1.50 dönüm");
        assert_eq!(other.bearing(100.0), "90.0000°");
    }

    /// The type's axes (docs/adr/0165 §4): a CAD project names east X and
    /// north Y, its polar angles from east; a CBS project and one not asked
    /// its type the surveyor's Y and X and semt. The Hesap windows' stay the
    /// surveyor's.
    #[test]
    fn a_projects_type_names_its_axes() {
        let mut settings = ProjectSettings {
            leader_styles: Vec::new(),
            wall_styles: Vec::new(),
            arch: None,
            line_type_scale: None,
            line_types: Vec::new(),
            length_style: None,
            xref_paths: Vec::new(),
            named_ucs: Vec::new(),
            named_views: Vec::new(),
            forms: Vec::new(),
            relations: Vec::new(),
            ucs: None,
            srid: 5256,
            length_decimals: 3,
            area_decimals: 2,
            area_unit: AreaUnit::M2,
            angle_unit: AngleUnit::Deg,
            plot_scale: 1000.0,
            workspace: Some(Workspace::Cad),
            drawing_font: None,
            drawing_unit: None,
            second_srid: None,
            custom_crs: None,
            second_custom_crs: None,
            datum_transforms: Vec::new(),
            layer_states: Vec::new(),
            survey: None,
            dimension_styles: Vec::new(),
            topology: None,
            annotation: None,
            text_styles: Vec::new(),
            connections: Vec::new(),
            networks: Vec::new(),
            variables: Vec::new(),
            address_locators: Vec::new(),
            domains: Vec::new(),
            hatch_patterns: Vec::new(),
            sources: Vec::new(),
        };
        let cad = Format::of(&settings);
        assert_eq!(cad.point(Vec2::new(120.0, 45.5)), "X 120.000  Y 45.500");
        assert_eq!(cad.input_hint(), "mesafe · X,Y · @dX,dY · @mesafe<açı");
        assert_eq!(
            cad.axes_text(
                "Taban noktasına tıklayın ya da Y,X yazın; aralık dY,dX. Y (sağa), X (yukarı)"
            ),
            "Taban noktasına tıklayın ya da X,Y yazın; aralık dX,dY. X (sağa), Y (yukarı)"
        );
        assert_eq!(
            cad.angles(),
            Angles {
                from: AngleFrom::East,
                grads: false
            }
        );
        assert_eq!(cad.metric().pair_label(), "Y,X", "the surveyor's");
        // A direction from east, counter-clockwise: semt 0 g (north) is 90°, 300 g (west) 180°.
        assert_eq!(
            (
                cad.direction_name(),
                cad.direction(0.0),
                cad.direction(300.0)
            ),
            ("Açı", "90.0000°".to_owned(), "180.0000°".to_owned())
        );
        assert_eq!(cad.direction_bare(100.0), "0.0000");
        settings.workspace = Some(Workspace::Gis);
        settings.angle_unit = AngleUnit::Grad;
        let gis = Format::of(&settings);
        assert_eq!(
            (gis.direction_name(), gis.direction(300.0)),
            ("Semt", "300.0000 g".to_owned())
        );
        assert_eq!(gis.input_hint(), "mesafe · Y,X · @dY,dX · @mesafe<semt");
        assert_eq!(gis.axes_text("Y,X"), "Y,X");
        assert_eq!(
            gis.angles(),
            Angles {
                from: AngleFrom::North,
                grads: true
            }
        );
        settings.workspace = Some(Workspace::LegacyHybrid);
        assert_eq!(Format::of(&settings).axes, Axes::Gis, "not asked: CBS");
    }

    /// A local project's drawing unit (docs/adr/0165 §2): lengths, coordinates
    /// and areas read and typed in it, the geometry in metres; a project with
    /// a coordinate system is in metres whatever unit it once had.
    #[test]
    fn a_local_project_reads_in_its_unit() {
        let mut settings = ProjectSettings {
            leader_styles: Vec::new(),
            wall_styles: Vec::new(),
            arch: None,
            line_type_scale: None,
            line_types: Vec::new(),
            length_style: None,
            xref_paths: Vec::new(),
            named_ucs: Vec::new(),
            named_views: Vec::new(),
            forms: Vec::new(),
            relations: Vec::new(),
            ucs: None,
            srid: 0,
            length_decimals: 3,
            area_decimals: 2,
            area_unit: AreaUnit::M2,
            angle_unit: AngleUnit::Grad,
            plot_scale: 100.0,
            workspace: None,
            drawing_font: None,
            drawing_unit: Some(DrawingUnit::Mm),
            second_srid: None,
            custom_crs: None,
            second_custom_crs: None,
            datum_transforms: Vec::new(),
            layer_states: Vec::new(),
            survey: None,
            dimension_styles: Vec::new(),
            topology: None,
            annotation: None,
            text_styles: Vec::new(),
            connections: Vec::new(),
            networks: Vec::new(),
            variables: Vec::new(),
            address_locators: Vec::new(),
            domains: Vec::new(),
            hatch_patterns: Vec::new(),
            sources: Vec::new(),
        };
        let f = Format::of(&settings);
        assert_eq!(f.coord(0.1), "100.000");
        assert_eq!(f.length(0.12), "120.000 mm");
        assert_eq!(f.area(0.0001), "100.00 mm²");
        assert_eq!((f.length_unit_label(), f.area_unit_label()), ("mm", "mm²"));
        assert_eq!((f.to_metres(250.0), f.from_metres(0.25)), (0.25, 250.0));
        assert_eq!(f.dimension("", "length", 0.12), "120.000");
        // A limit with the digits it needs, and the unit in words.
        assert_eq!(
            (f.plain(1e-6), f.length_unit_name()),
            ("0.001".to_owned(), "milimetre")
        );
        // The Hesap windows stay in metres.
        assert_eq!(f.metric().length(0.12), "0.120 m");
        settings.drawing_unit = Some(DrawingUnit::Cm);
        assert_eq!(Format::of(&settings).area(0.0001), "1.00 cm²");
        settings.srid = 5254;
        let f = Format::of(&settings);
        assert_eq!(
            (f.length(0.12), f.to_metres(250.0)),
            ("0.120 m".to_owned(), 250.0)
        );
    }

    /// Feet and inches (docs/adr/0262): typed and shown by their exact
    /// ratio, written in the project's style with their marks.
    #[test]
    fn an_inch_project_writes_feet_and_inches() {
        let mut settings: ProjectSettings =
            serde_json::from_value(serde_json::json!({
                "srid": 0, "lengthDecimals": 4, "areaDecimals": 2, "areaUnit": "m2",
                "angleUnit": "deg", "plotScale": 48.0, "drawingUnit": "in", "lengthStyle": "architectural"
            }))
            .expect("settings");
        let f = Format::of(&settings);
        assert_eq!(f.to_metres(10.0), 0.254);
        assert_eq!(f.length(150.5 * 0.0254), "12'-6 1/2\"");
        assert_eq!(f.length_unit_name(), "inç");
        assert_eq!(f.area_unit_label(), "in²");
        settings.length_style = Some(kentos_contracts::LengthStyle::Engineering);
        settings.length_decimals = 2;
        assert_eq!(Format::of(&settings).length(150.5 * 0.0254), "12'-6.50\"");
        // Decimal in feet carries the unit's mark.
        settings.length_style = None;
        settings.drawing_unit = Some(DrawingUnit::Ft);
        assert_eq!(Format::of(&settings).length(0.3048 * 12.5), "12.50 ft");
        // A metric unit is decimal whatever the setting says.
        settings.length_style = Some(kentos_contracts::LengthStyle::Fractional);
        settings.drawing_unit = Some(DrawingUnit::Mm);
        assert_eq!(Format::of(&settings).length(0.25), "250.00 mm");
    }

    /// The display rule (docs/adr/0149); every case of it is the core's
    /// `fixtures/numeric/v1/display.json`.
    #[test]
    fn fixed_writes_by_the_display_rule() {
        for (v, d, shown) in [
            (487012.0625, 3, "487012.063"),
            (0.125, 2, "0.13"),
            (-0.125, 2, "-0.13"),
            (2.5, 0, "3"),
            (0.5, 0, "1"),
            // 9.995 and 1.005 are just under a half in binary; written as typed, rounded as on paper.
            (9.995, 2, "10.00"),
            (99.5, 0, "100"),
            (9.9995, 3, "10.000"),
            (1.005, 2, "1.01"),
            // A value that rounds to zero has no sign.
            (-0.001, 2, "0.00"),
            (-0.0, 3, "0.000"),
            (12.0, 3, "12.000"),
            (1234.5678, 2, "1234.57"),
            (5e-324, 3, "0.000"),
        ] {
            assert_eq!(fixed(v, d), shown, "{v} with {d}");
        }
        assert_eq!(fixed(f64::NAN, 2), "NaN");
        assert_eq!(fixed(f64::NEG_INFINITY, 2), "-Infinity");
    }
}
