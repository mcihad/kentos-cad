//! KentOS: the float methods whose results the platforms' maths libraries
//! may round differently, through `libm` on every target, so that a native
//! build and the browser's WASM answer alike bit for bit (docs/adr/0171;
//! written by scripts/vendor/geographiclib.py).

pub(crate) trait Lm: Sized {
    fn lm_sin(self) -> Self;
    fn lm_cos(self) -> Self;
    fn lm_sin_cos(self) -> (Self, Self);
    fn lm_atan2(self, x: Self) -> Self;
    fn lm_hypot(self, y: Self) -> Self;
    fn lm_cbrt(self) -> Self;
    fn lm_atanh(self) -> Self;
    fn lm_atan(self) -> Self;
}

impl Lm for f64 {
    fn lm_sin(self) -> f64 {
        libm::sin(self)
    }
    fn lm_cos(self) -> f64 {
        libm::cos(self)
    }
    fn lm_sin_cos(self) -> (f64, f64) {
        libm::sincos(self)
    }
    fn lm_atan2(self, x: f64) -> f64 {
        libm::atan2(self, x)
    }
    fn lm_hypot(self, y: f64) -> f64 {
        libm::hypot(self, y)
    }
    fn lm_cbrt(self) -> f64 {
        libm::cbrt(self)
    }
    fn lm_atanh(self) -> f64 {
        libm::atanh(self)
    }
    fn lm_atan(self) -> f64 {
        libm::atan(self)
    }
}
