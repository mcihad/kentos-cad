//! A node's points made ready to draw (docs/adr/0207 §6): positions as
//! float32 differences from the node's centre (the GPU never sees a world
//! coordinate), their returns, and of their classes, intensities and colours
//! what the look reads ([`Needs`]): a COPC's other layers stay compressed,
//! which halves a node's decoding for every look but the file's colours.

use kentos_contracts::{CloudRender, PointCloudStyle};

use crate::record::Layout;

/// What a look reads of a point besides its position and returns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Needs {
    pub class: bool,
    pub intensity: bool,
    pub rgb: bool,
}

impl Needs {
    /// Everything a look may read.
    pub const ALL: Needs = Needs {
        class: true,
        intensity: true,
        rgb: true,
    };

    /// What `style` reads: classes by class or to hide some, intensities or colours by their looks.
    pub fn of(style: &PointCloudStyle) -> Needs {
        Needs {
            class: style.render == CloudRender::Classification || !style.hidden.is_empty(),
            intensity: style.render == CloudRender::Intensity,
            rgb: style.render == CloudRender::Rgb,
        }
    }

    /// Whether points decoded for `self` hold what `other` reads.
    pub fn covers(self, other: Needs) -> bool {
        (self.class || !other.class)
            && (self.intensity || !other.intensity)
            && (self.rgb || !other.rgb)
    }

    /// The needs as one small number (a cache's key).
    pub fn bits(self) -> u8 {
        u8::from(self.class) | u8::from(self.intensity) << 1 | u8::from(self.rgb) << 2
    }
}

/// A node's points for the GPU and its looks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NodePoints {
    /// The node's centre in the world.
    pub center: [f64; 3],
    /// x, y, z per point, from the centre.
    pub xyz: Vec<f32>,
    /// One per point when decoded for it ([`Needs::class`]), else empty.
    pub class: Vec<u8>,
    /// One per point when decoded for it, else empty.
    pub intensity: Vec<u16>,
    /// Return number in the low four bits, number of returns in the high.
    pub returns: Vec<u8>,
    /// Red, green and blue per point; empty when the format has none or they were not decoded.
    pub rgb: Vec<u16>,
}

impl NodePoints {
    pub fn len(&self) -> usize {
        self.returns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.returns.is_empty()
    }

    /// A point's height in the world.
    #[inline]
    pub fn z(&self, i: usize) -> f64 {
        self.center[2] + f64::from(self.xyz[3 * i + 2])
    }

    /// Bytes it holds.
    pub fn bytes(&self) -> usize {
        self.xyz.len() * 4
            + self.class.len()
            + self.intensity.len() * 2
            + self.returns.len()
            + self.rgb.len() * 2
    }

    /// A point's class; 0 when classes were not decoded.
    #[inline]
    pub fn class(&self, i: usize) -> u8 {
        self.class.get(i).copied().unwrap_or(0)
    }

    /// A point's intensity; 0 when intensities were not decoded.
    #[inline]
    pub fn intensity(&self, i: usize) -> u16 {
        self.intensity.get(i).copied().unwrap_or(0)
    }
}

/// The points of `records` (of `layout`, coordinates by `scale` and `offset`)
/// about `center`, with what `needs` names: the records' other fields are not
/// read (a picture's records leave them undefined, `chunks::selection_for`).
pub fn decode(
    layout: &Layout,
    records: &[u8],
    scale: [f64; 3],
    offset: [f64; 3],
    center: [f64; 3],
    needs: Needs,
) -> NodePoints {
    let n = records.len() / layout.len.max(1);
    let rgb = needs.rgb && layout.rgb.is_some();
    let mut p = NodePoints {
        center,
        xyz: Vec::with_capacity(3 * n),
        class: Vec::with_capacity(if needs.class { n } else { 0 }),
        intensity: Vec::with_capacity(if needs.intensity { n } else { 0 }),
        returns: Vec::with_capacity(n),
        rgb: Vec::with_capacity(if rgb { 3 * n } else { 0 }),
    };
    for r in records.chunks_exact(layout.len) {
        let x = f64::from(layout.x(r)) * scale[0] + offset[0];
        let y = f64::from(layout.y(r)) * scale[1] + offset[1];
        let z = f64::from(layout.z(r)) * scale[2] + offset[2];
        p.xyz.push((x - center[0]) as f32);
        p.xyz.push((y - center[1]) as f32);
        p.xyz.push((z - center[2]) as f32);
        if needs.class {
            p.class.push(layout.class(r));
        }
        if needs.intensity {
            p.intensity.push(layout.intensity(r));
        }
        let (ret, nret) = layout.returns(r);
        p.returns.push((ret & 0x0F) | (nret << 4));
        if rgb && let Some(c) = layout.rgb(r) {
            p.rgb.extend_from_slice(&c);
        }
    }
    p
}
