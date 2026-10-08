//! The records an operation writes (docs/adr/0207 §7): a result of one file
//! keeps that file's version, point format, scale, offset, VLRs and extra
//! bytes; of several (a virtual cloud, Birleştir) it is LAS 1.4 in the
//! widest format they need (6, 7 or 8), at the finest scale, at the first
//! file's offset, with the first's system, and with their extra bytes only
//! when every file has the same ones. A coordinate that does not land on the
//! result's grid is rounded (half away from zero) and counted.

use crate::las::{Header, Vlr};
use crate::record::{Layout, standard_len, widen};
use crate::text;
use crate::write::Spec;
use crate::{PcError, Result};

/// A file's records as the host reads them: its header, layout and records.
#[derive(Clone, Debug)]
pub struct Input {
    pub head: Header,
    pub layout: Layout,
    pub vlrs: Vec<Vlr>,
    pub evlrs: Vec<Vlr>,
}

impl Input {
    /// A text cloud's records as its parse gives them (LAS 1.4, format 6 or 7, its own scale and offset).
    pub fn text(plan: &text::Plan) -> Input {
        let format = plan.columns.format();
        let len = standard_len(format);
        let head = Header {
            major: 1,
            minor: 4,
            source_id: 0,
            global_encoding: 0,
            guid: [0; 16],
            system: String::new(),
            software: String::new(),
            day: 0,
            year: 0,
            header_size: crate::las::HEADER_1_4 as u16,
            data_offset: crate::las::HEADER_1_4 as u32,
            vlr_count: 0,
            format,
            compressed: false,
            record_len: len as u16,
            count: plan.count,
            by_return: [0; 15],
            scale: plan.scale,
            offset: plan.offset,
            min: [plan.bounds[0], plan.bounds[1], plan.bounds[2]],
            max: [plan.bounds[3], plan.bounds[4], plan.bounds[5]],
            waveform_start: 0,
            evlr_start: 0,
            evlr_count: 0,
        };
        Input {
            head,
            layout: Layout::new(format, len),
            vlrs: Vec::new(),
            evlrs: Vec::new(),
        }
    }

    fn extra_vlr(&self) -> Option<&Vlr> {
        self.vlrs
            .iter()
            .chain(&self.evlrs)
            .find(|v| v.is("LASF_Spec", 4))
    }
}

/// The result's file and what it gave up.
#[derive(Clone, Debug)]
pub struct Plan {
    pub spec: Spec,
    /// The files' extra bytes differed and were left out.
    pub dropped_extra: bool,
}

/// The result's file for `inputs` (one or more), compressed or not.
pub fn plan(inputs: &[Input], compressed: bool) -> Result<Plan> {
    let first = inputs
        .first()
        .ok_or_else(|| PcError::new("İşlenecek nokta bulutu dosyası yok."))?;
    if let [one] = inputs {
        return Ok(Plan {
            spec: Spec::like(&one.head, &one.vlrs, &one.evlrs, compressed),
            dropped_extra: false,
        });
    }
    let nir = inputs.iter().any(|i| i.layout.nir.is_some());
    let rgb = inputs.iter().any(|i| i.layout.rgb.is_some());
    let format = if nir {
        8
    } else if rgb {
        7
    } else {
        6
    };
    let extras_same = inputs.iter().all(|i| {
        i.layout.extra() == first.layout.extra()
            && i.extra_vlr().map(|v| &v.data) == first.extra_vlr().map(|v| &v.data)
    });
    let extra = if extras_same { first.layout.extra() } else { 0 };
    let mut scale = first.head.scale;
    for i in inputs {
        for (s, &v) in scale.iter_mut().zip(&i.head.scale) {
            *s = s.min(v);
        }
    }
    let mut vlrs: Vec<Vlr> = first
        .vlrs
        .iter()
        .filter(|v| v.user == "LASF_Projection")
        .cloned()
        .collect();
    if extras_same && let Some(e) = first.extra_vlr() {
        let mut e = e.clone();
        e.extended = false;
        vlrs.push(e);
    }
    let wkt = vlrs.iter().any(|v| v.is("LASF_Projection", 2112));
    let gps_standard = inputs.iter().all(|i| i.head.gps_standard());
    let record_len =
        u16::try_from(standard_len(format) + extra).map_err(|_| PcError::new("Kayıt çok uzun."))?;
    Ok(Plan {
        spec: Spec {
            minor: 4,
            format,
            record_len,
            scale,
            offset: first.head.offset,
            vlrs,
            evlrs: Vec::new(),
            compressed,
            source_id: first.head.source_id,
            global_encoding: (if gps_standard {
                crate::las::ENCODING_GPS_STANDARD
            } else {
                0
            }) | if wkt { crate::las::ENCODING_WKT } else { 0 },
            software: "KentOS CAD".to_owned(),
        },
        dropped_extra: !extras_same && inputs.iter().any(|i| i.layout.extra() > 0),
    })
}

/// A file's records written as the result's.
#[derive(Clone, Debug)]
pub struct Converter {
    from: Layout,
    from_scale: [f64; 3],
    from_offset: [f64; 3],
    to: Layout,
    to_scale: [f64; 3],
    to_offset: [f64; 3],
    /// Same layout, scale and offset: the records as they are.
    same: bool,
    /// Only the coordinates change.
    same_layout: bool,
    widen: bool,
}

/// How close to the result's grid a moved coordinate must be to count as kept (in its steps).
const ON_GRID: f64 = 1e-6;

impl Converter {
    pub fn new(input: &Input, spec: &Spec) -> Converter {
        let to = spec.layout();
        let same_layout = input.layout == to;
        let same_grid = input.head.scale == spec.scale && input.head.offset == spec.offset;
        Converter {
            from: input.layout,
            from_scale: input.head.scale,
            from_offset: input.head.offset,
            to,
            to_scale: spec.scale,
            to_offset: spec.offset,
            same: same_layout && same_grid,
            same_layout,
            widen: !same_layout,
        }
    }

    /// The result's layout.
    pub fn layout(&self) -> Layout {
        self.to
    }

    /// `records` as the result's, appended to `out`: the number of points rounded onto its grid.
    pub fn convert(&self, records: &[u8], out: &mut Vec<u8>) -> Result<u64> {
        if self.same {
            out.extend_from_slice(records);
            return Ok(0);
        }
        let mut rounded = 0;
        let start = out.len();
        let n = records.len() / self.from.len.max(1);
        out.resize(start + n * self.to.len, 0);
        for (i, r) in records.chunks_exact(self.from.len).enumerate() {
            let o = &mut out[start + i * self.to.len..start + (i + 1) * self.to.len];
            if self.widen {
                widen(&self.from, r, &self.to, o);
            } else if self.same_layout {
                o.copy_from_slice(r);
            }
            let ints = [self.from.x(r), self.from.y(r), self.from.z(r)];
            let mut moved = [0i32; 3];
            // The point counts once however many of its coordinates leave the grid.
            let mut off = false;
            for k in 0..3 {
                let v = f64::from(ints[k]) * self.from_scale[k] + self.from_offset[k];
                let n = (v - self.to_offset[k]) / self.to_scale[k];
                let m = n.round();
                if !(m >= f64::from(i32::MIN) && m <= f64::from(i32::MAX)) {
                    return Err(PcError::new(
                        "Bulutların kapsamı sonucun ölçeğine sığmıyor; daha kaba bir ölçek gerekir.",
                    ));
                }
                off |= (n - m).abs() > ON_GRID;
                moved[k] = m as i32;
            }
            if off {
                rounded += 1;
            }
            self.to.set_xyz(o, moved[0], moved[1], moved[2]);
        }
        Ok(rounded)
    }
}
