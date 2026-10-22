//! Allocated paper block/layout owners and standard 2D viewport records.
use super::{Handles, Out};
use kentos_contracts::{CadLayout, CadViewport};
pub(super) struct Paper<'a> {
    pub data: &'a CadLayout,
    pub record: u64,
    pub layout: u64,
    pub begin: u64,
    pub end: u64,
    pub block: String,
}
pub(super) fn allocate<'a>(data: &'a [CadLayout], handles: &mut Handles) -> Vec<Paper<'a>> {
    data.iter()
        .enumerate()
        .map(|(i, data)| Paper {
            data,
            record: if i == 0 { 0x1B } else { handles.take() },
            layout: if i == 0 { 0x1E } else { handles.take() },
            begin: if i == 0 { 0x1C } else { handles.take() },
            end: if i == 0 { 0x1D } else { handles.take() },
            block: if i == 0 {
                "*Paper_Space".into()
            } else {
                format!("*Paper_Space{}", i - 1)
            },
        })
        .collect()
}
pub(super) fn viewport(
    out: &mut Out,
    handles: &mut Handles,
    paper: &Paper<'_>,
    view: &CadViewport,
    id: usize,
    per_metre: f64,
) {
    out.str(0, "VIEWPORT");
    out.handle(5, handles.take());
    out.handle(330, paper.record);
    out.str(100, "AcDbEntity");
    out.int(67, 1);
    out.str(410, &super::blocks::one_line(&paper.data.name));
    out.str(8, "0");
    out.str(100, "AcDbViewport");
    out.xyz(10, view.paper_center_mm);
    out.real(40, view.width_mm);
    out.real(41, view.height_mm);
    out.int(68, 1);
    out.int(69, id as i64);
    // DCS origin is chosen at WCS zero; rotate the WCS view centre into DCS.
    let (sn, cs) = (crate::math::sin(view.twist), crate::math::cos(view.twist));
    out.real(
        12,
        (cs * view.view_center.x + sn * view.view_center.y) * per_metre,
    );
    out.real(
        22,
        (-sn * view.view_center.x + cs * view.view_center.y) * per_metre,
    );
    out.real(16, 0.);
    out.real(26, 0.);
    out.real(36, 1.);
    out.real(17, 0.);
    out.real(27, 0.);
    out.real(37, 0.);
    out.real(45, view.view_height * per_metre);
    out.real(51, view.twist.to_degrees());
    // DXF 90: 0x8000 is enabled; 0x20000 would turn the viewport off.
    out.int(90, 32768 | if view.locked {16384}else{0});
}
