//! A curve's traveled way widening for a design vehicle (docs/adr/0932, plan/road.md Y28; ROAD-DEVER-05): the AASHTO
//! Green Book's width on the curve from the vehicle's steady-state off-tracking, its front overhang, the lateral
//! clearance and the extra width allowance, less the width on the tangent.
//!
//! **Steady-state off-tracking.** Turning steadily, every fixed axle's extension passes through the curve's centre
//! (Heald, TRR 1052, the WHI formula): the front axle's centre runs at R, the rear axle's at `√(R² − L₁²)`, a point d
//! behind the rear axle on the axis (a trailer's kingpin) at `√(r² + d²)`, the trailer's axle L₂ behind its kingpin at
//! `√(r_k² − L₂²)`. So the rearmost axle runs at `√(R² − ΣLᵢ²)` with `ΣLᵢ² = L₁² − d² + L₂²` (the kingpin's offset counts
//! negative: the "sum of squares" is not the sum of the wheelbases' squares). The off-tracking `R − √(R² − Σ)` is
//! computed as `Σ/(R + √(R² − Σ))`: the same number without the cancellation of two nearly equal radii (at R = 1000 m
//! the difference form loses five digits). It is the maximum; a curve too short to reach the steady state off-tracks
//! less. Where `R² ≤ Σ` the vehicle cannot turn steadily on the curve (the centre falls inside its rearmost axle's path).
//!
//! **The width on the curve** (AASHTO, metric): `W_c = N(U + C) + (N − 1)·F_A + Z` with the track `U = u + off-tracking`
//! (u the track width on the tangent, out to out of the tires), the front overhang's width `F_A = √(R² + A(2L + A)) − R`
//! (A the front overhang, L the tractor's wheelbase; also without cancellation), the extra width `Z = 0.1·V/√R` (V in
//! km/h) and C the lateral clearance per lane. The widening is `W_c − W_n` (W_n the traveled way's width on the tangent);
//! AASHTO's simpler form (Civil 3D's "Simple") keeps `N(U + C)` only. None of u, A, L, C comes from KentOS: they are the
//! agency's design vehicle and standard. The classical `n·L²/(2R)` is the off-tracking's first term; it is short by
//! about `L⁴/(8R³)`.

/// A design vehicle as the widening reads it: the track width out to out of the tires, the front overhang ahead of the
/// front axle, the tractor's wheelbase, m; a trailer's kingpin behind the tractor's rear axle (negative: ahead of it)
/// and its wheelbase from the kingpin to its axle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WideningVehicle {
    pub track: f64,
    pub front: f64,
    pub wheelbase: f64,
    pub trailer: Option<(f64, f64)>,
}

impl WideningVehicle {
    /// `ΣLᵢ²`: the tractor's wheelbase², less the kingpin's offset², plus the trailer's wheelbase².
    pub fn sum_of_squares(&self) -> f64 {
        let (k, l2) = self.trailer.unwrap_or((0.0, 0.0));
        self.wheelbase * self.wheelbase - k * k + l2 * l2
    }
}

/// AASHTO's two forms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WideningMethod {
    /// `N(U + C) + (N − 1)·F_A + Z`.
    #[default]
    Standard,
    /// `N(U + C)`.
    Simple,
}

/// What the road gives: its lanes and their width on the tangent (m), the lateral clearance per lane (m), the design
/// speed (km/h; none: no extra width), the method.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WideningRoad {
    pub lanes: u32,
    pub tangent: f64,
    pub clearance: f64,
    pub speed: Option<f64>,
    pub method: WideningMethod,
}

/// A curve's widening: the off-tracking, the track U, the front overhang's width, the extra width, the width on the
/// curve and the widening (negative: the tangent's width is more than the curve needs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Widening {
    pub offtracking: f64,
    pub track: f64,
    pub overhang: f64,
    pub allowance: f64,
    pub width: f64,
    pub widening: f64,
}

/// Why a widening is not read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WideningError {
    /// A size not finite, a track or wheelbase not positive, an overhang, clearance or tangent width negative, no lane,
    /// a speed not positive, a radius not positive.
    Input,
    /// `R² ≤ ΣLᵢ²`: the vehicle cannot turn steadily on a curve of this radius.
    Unturnable,
}

impl std::fmt::Display for WideningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            WideningError::Input => {
                "iz genişliği ve dingil aralığı sıfırdan büyük; ön çıkıntı, açıklık ve teğet genişliği eksi olmayan; \
                 şerit sayısı, hız ve yarıçap sıfırdan büyük sayılar olmalı"
            }
            WideningError::Unturnable => "taşıt bu yarıçapta kararlı dönemez: yarıçap dingil aralıklarının karelerinin toplamının kökünden küçük",
        })
    }
}

/// The steady-state off-tracking of a vehicle whose `ΣLᵢ²` is `sum` on a radius `radius` (m): `R − √(R² − Σ)`.
pub fn offtracking(radius: f64, sum: f64) -> Option<f64> {
    let inner = radius * radius - sum;
    (inner > 0.0).then(|| sum / (radius + inner.sqrt()))
}

/// The front overhang's width: `√(R² + A(2L + A)) − R`.
pub fn front_overhang(radius: f64, front: f64, wheelbase: f64) -> f64 {
    let add = front * (2.0 * wheelbase + front);
    add / ((radius * radius + add).sqrt() + radius)
}

/// The extra width for the difficulty of driving a curve: `0.1·V/√R` (V km/h, R m).
pub fn allowance(speed: f64, radius: f64) -> f64 {
    0.1 * speed / radius.sqrt()
}

/// The widening of a curve of radius `radius` (m, the traveled way's centreline) for `vehicle` on `road`.
pub fn widening(radius: f64, vehicle: &WideningVehicle, road: &WideningRoad) -> Result<Widening, WideningError> {
    let finite = [radius, vehicle.track, vehicle.front, vehicle.wheelbase, road.tangent, road.clearance]
        .iter()
        .chain(vehicle.trailer.as_ref().map(|t| [t.0, t.1]).iter().flatten())
        .chain(road.speed.iter())
        .all(|v| v.is_finite());
    let trailer_ok = vehicle.trailer.is_none_or(|(_, l2)| l2 > 0.0);
    if !finite
        || !trailer_ok
        || radius <= 0.0
        || vehicle.track <= 0.0
        || vehicle.wheelbase <= 0.0
        || vehicle.front < 0.0
        || road.tangent < 0.0
        || road.clearance < 0.0
        || road.lanes == 0
        || road.speed.is_some_and(|v| v <= 0.0)
    {
        return Err(WideningError::Input);
    }
    let off = offtracking(radius, vehicle.sum_of_squares()).ok_or(WideningError::Unturnable)?;
    let track = vehicle.track + off;
    let n = f64::from(road.lanes);
    let (overhang, extra) = match road.method {
        WideningMethod::Standard => {
            (front_overhang(radius, vehicle.front, vehicle.wheelbase), road.speed.map_or(0.0, |v| allowance(v, radius)))
        }
        WideningMethod::Simple => (0.0, 0.0),
    };
    let width = n * (track + road.clearance) + (n - 1.0) * overhang + extra;
    Ok(Widening { offtracking: off, track, overhang, allowance: extra, width, widening: width - road.tangent })
}

/// The widening as written in a schedule: none below `least` (m; an agency disregards small amounts), else rounded up
/// to a multiple of `step` (m). None of the two given: the widening as it is, none when not positive.
pub fn rounded(widening: f64, step: Option<f64>, least: Option<f64>) -> f64 {
    if widening.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) || least.is_some_and(|l| widening < l) {
        return 0.0;
    }
    match step.filter(|s| *s > 0.0) {
        // A widening that is a multiple within a hair stays (0.6 is not rounded to 0.7 by 0.6000000000000001).
        Some(s) => (widening / s - 1e-9).ceil() * s,
        None => widening,
    }
}
