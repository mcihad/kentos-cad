//! The parts of an AutoCAD 2000+ DXF that do not depend on the drawing:
//! the class list, the symbol tables' heads and fixed records, the model
//! and paper space blocks, and the objects AutoCAD expects (the root
//! dictionary, layouts, plot style, materials, multiline and multileader
//! styles). Handles are fixed below `FIRST_FREE`; everything the drawing
//! adds is numbered from there. The content follows the minimal document
//! ezdxf writes and AutoCAD opens (checked with ezdxf's auditor, ADR 0009).

use super::Out;

pub const LAYER_TABLE: u64 = 0x1;
pub const LTYPE_TABLE: u64 = 0x2;
pub const APPID_TABLE: u64 = 0x3;
pub const DIMSTYLE_TABLE: u64 = 0x4;
pub const STYLE_TABLE: u64 = 0x5;
pub const UCS_TABLE: u64 = 0x6;
pub const VIEW_TABLE: u64 = 0x7;
pub const VPORT_TABLE: u64 = 0x8;
pub const BLOCK_RECORD_TABLE: u64 = 0x9;
/// The *Model_Space block record: the owner of every entity.
pub const MODEL_SPACE: u64 = 0x17;
const PAPER_SPACE: u64 = 0x1B;
/// The "Normal" plot style every layer points to.
pub const PLOT_STYLE: u64 = 0x13;
/// The "Global" material every layer points to.
pub const MATERIAL: u64 = 0x21;
const VPORT: u64 = 0x23;
pub const LTYPE_BYBLOCK: u64 = 0x24;
pub const LTYPE_BYLAYER: u64 = 0x25;
pub const LTYPE_CONTINUOUS: u64 = 0x26;
pub const LAYER_ZERO: u64 = 0x27;
const STYLE: u64 = 0x29;
const APPID_ACAD: u64 = 0x2A;
const DIMSTYLE: u64 = 0x2B;
pub const APPID_KENTOS: u64 = 0x2D;
/// The first handle the drawing's own records and entities take.
pub const FIRST_FREE: u64 = 0x2E;

/// Out of line, so the fixed lists stay data (unrolled, each group would be code).
#[inline(never)]
fn fixed(out: &mut Out, groups: &[(i32, &str)]) {
    for (code, v) in groups {
        out.str(*code, v);
    }
}

/// The classes of the objects below (and those AutoCAD lists in every drawing).
pub fn classes(out: &mut Out) {
    const CLASSES: &[(&str, &str, &str, &str)] = &[
        (
            "ACDBDICTIONARYWDFLT",
            "AcDbDictionaryWithDefault",
            "ObjectDBX Classes",
            "0",
        ),
        ("SUN", "AcDbSun", "SCENEOE", "1153"),
        (
            "VISUALSTYLE",
            "AcDbVisualStyle",
            "ObjectDBX Classes",
            "4095",
        ),
        ("MATERIAL", "AcDbMaterial", "ObjectDBX Classes", "1153"),
        ("SCALE", "AcDbScale", "ObjectDBX Classes", "1153"),
        ("TABLESTYLE", "AcDbTableStyle", "ObjectDBX Classes", "4095"),
        (
            "MLEADERSTYLE",
            "AcDbMLeaderStyle",
            "ACDB_MLEADERSTYLE_CLASS",
            "4095",
        ),
        (
            "DICTIONARYVAR",
            "AcDbDictionaryVar",
            "ObjectDBX Classes",
            "0",
        ),
        (
            "CELLSTYLEMAP",
            "AcDbCellStyleMap",
            "ObjectDBX Classes",
            "1152",
        ),
        (
            "MENTALRAYRENDERSETTINGS",
            "AcDbMentalRayRenderSettings",
            "SCENEOE",
            "1024",
        ),
        (
            "ACDBDETAILVIEWSTYLE",
            "AcDbDetailViewStyle",
            "ObjectDBX Classes",
            "1025",
        ),
        (
            "ACDBSECTIONVIEWSTYLE",
            "AcDbSectionViewStyle",
            "ObjectDBX Classes",
            "1025",
        ),
        ("RASTERVARIABLES", "AcDbRasterVariables", "ISM", "0"),
        (
            "ACDBPLACEHOLDER",
            "AcDbPlaceHolder",
            "ObjectDBX Classes",
            "0",
        ),
        ("LAYOUT", "AcDbLayout", "ObjectDBX Classes", "0"),
    ];
    out.section("CLASSES");
    for (record, class, app, flags) in CLASSES {
        fixed(
            out,
            &[
                (0, "CLASS"),
                (1, record),
                (2, class),
                (3, app),
                (90, flags),
                (91, "0"),
                (280, "0"),
                (281, "0"),
            ],
        );
    }
    out.str(0, "ENDSEC");
}

/// A symbol table's head with its record count.
pub fn table(out: &mut Out, name: &str, handle: u64, count: usize) {
    out.str(0, "TABLE");
    out.str(2, name);
    out.handle(5, handle);
    out.str(330, "0");
    out.str(100, "AcDbSymbolTable");
    out.int(70, count as i64);
}

/// A symbol table record's head (name and flags follow).
pub fn record(out: &mut Out, kind: &str, handle: u64, owner: u64, class: &str) {
    out.str(0, kind);
    out.handle(if kind == "DIMSTYLE" { 105 } else { 5 }, handle);
    out.handle(330, owner);
    out.str(100, "AcDbSymbolTableRecord");
    out.str(100, class);
}

/// The *Active viewport, looking at `centre` with `height` of the drawing on screen.
pub fn vport_table(out: &mut Out, centre: (f64, f64), height: f64) {
    table(out, "VPORT", VPORT_TABLE, 1);
    record(out, "VPORT", VPORT, VPORT_TABLE, "AcDbViewportTableRecord");
    fixed(
        out,
        &[
            (2, "*Active"),
            (70, "0"),
            (10, "0.0"),
            (20, "0.0"),
            (11, "1.0"),
            (21, "1.0"),
        ],
    );
    out.real(12, centre.0);
    out.real(22, centre.1);
    fixed(
        out,
        &[
            (13, "0.0"),
            (23, "0.0"),
            (14, "0.5"),
            (24, "0.5"),
            (15, "0.5"),
            (25, "0.5"),
            (16, "0.0"),
            (26, "0.0"),
            (36, "1.0"),
            (17, "0.0"),
            (27, "0.0"),
            (37, "0.0"),
        ],
    );
    out.real(40, height);
    fixed(
        out,
        &[
            (41, "1.34"),
            (42, "50.0"),
            (43, "0.0"),
            (44, "0.0"),
            (50, "0.0"),
            (51, "0.0"),
            (71, "0"),
            (72, "1000"),
            (73, "1"),
            (74, "3"),
            (75, "0"),
            (76, "0"),
            (77, "0"),
            (78, "0"),
            (281, "0"),
            (65, "0"),
            (146, "0.0"),
        ],
    );
    out.str(0, "ENDTAB");
}

/// The line types every drawing has: ByBlock, ByLayer, Continuous.
pub fn builtin_ltypes(out: &mut Out) {
    for (handle, name, description) in [
        (LTYPE_BYBLOCK, "ByBlock", ""),
        (LTYPE_BYLAYER, "ByLayer", ""),
        (LTYPE_CONTINUOUS, "Continuous", "Solid line"),
    ] {
        record(out, "LTYPE", handle, LTYPE_TABLE, "AcDbLinetypeTableRecord");
        fixed(
            out,
            &[
                (2, name),
                (70, "0"),
                (3, description),
                (72, "65"),
                (73, "0"),
                (40, "0.0"),
            ],
        );
    }
}

/// STYLE (Standard, Arial: it has every Turkish letter), VIEW and UCS (empty).
pub fn style_view_ucs(out: &mut Out) {
    table(out, "STYLE", STYLE_TABLE, 1);
    record(out, "STYLE", STYLE, STYLE_TABLE, "AcDbTextStyleTableRecord");
    fixed(
        out,
        &[
            (2, "Standard"),
            (70, "0"),
            (40, "0.0"),
            (41, "1.0"),
            (50, "0.0"),
            (71, "0"),
            (42, "2.5"),
            (3, "arial.ttf"),
            (4, ""),
        ],
    );
    out.str(0, "ENDTAB");
    table(out, "VIEW", VIEW_TABLE, 0);
    out.str(0, "ENDTAB");
    table(out, "UCS", UCS_TABLE, 0);
    out.str(0, "ENDTAB");
}

/// APPID: AutoCAD's own and KentOS's (its extended data needs it registered).
pub fn appid_table(out: &mut Out) {
    table(out, "APPID", APPID_TABLE, 2);
    for (handle, name) in [
        (APPID_ACAD, "ACAD"),
        (APPID_KENTOS, super::super::xdata::APP),
    ] {
        record(out, "APPID", handle, APPID_TABLE, "AcDbRegAppTableRecord");
        fixed(out, &[(2, name), (70, "0")]);
    }
    out.str(0, "ENDTAB");
}

/// DIMSTYLE: the Standard style (the file has no dimension entities; AutoCAD needs the table).
pub fn dimstyle_table(out: &mut Out) {
    table(out, "DIMSTYLE", DIMSTYLE_TABLE, 1);
    out.str(100, "AcDbDimStyleTable");
    record(
        out,
        "DIMSTYLE",
        DIMSTYLE,
        DIMSTYLE_TABLE,
        "AcDbDimStyleTableRecord",
    );
    fixed(
        out,
        &[
            (2, "Standard"),
            (70, "0"),
            (40, "1.0"),
            (41, "2.5"),
            (42, "0.625"),
            (43, "3.75"),
            (44, "1.25"),
            (45, "0.0"),
            (46, "0.0"),
            (47, "0.0"),
            (48, "0.0"),
            (49, "2.5"),
            (140, "2.5"),
            (141, "2.5"),
            (142, "0.0"),
            (143, "0.03937007874"),
            (144, "1.0"),
            (145, "0.0"),
            (146, "1.0"),
            (147, "0.625"),
            (148, "0.0"),
            (69, "0"),
            (70, "0"),
            (71, "0"),
            (72, "0"),
            (73, "0"),
            (74, "0"),
            (75, "0"),
            (76, "0"),
            (77, "1"),
            (78, "8"),
            (79, "3"),
            (170, "0"),
            (171, "3"),
            (172, "1"),
            (173, "0"),
            (174, "0"),
            (175, "0"),
            (176, "0"),
            (177, "0"),
            (178, "0"),
            (179, "2"),
            (271, "2"),
            (272, "2"),
            (273, "2"),
            (274, "3"),
            (275, "0"),
            (276, "0"),
            (277, "2"),
            (278, "44"),
            (279, "0"),
            (280, "0"),
            (281, "0"),
            (282, "0"),
            (283, "0"),
            (284, "8"),
            (285, "0"),
            (286, "0"),
            (288, "0"),
            (289, "3"),
            (290, "0"),
            (371, "-2"),
            (372, "-2"),
        ],
    );
    out.str(0, "ENDTAB");
}

/// BLOCK_RECORD: model space and paper space, each pointing to its layout.
pub fn block_record_table(out: &mut Out) {
    table(out, "BLOCK_RECORD", BLOCK_RECORD_TABLE, 2);
    for (handle, name, layout) in [
        (MODEL_SPACE, "*Model_Space", "1A"),
        (PAPER_SPACE, "*Paper_Space", "1E"),
    ] {
        record(
            out,
            "BLOCK_RECORD",
            handle,
            BLOCK_RECORD_TABLE,
            "AcDbBlockTableRecord",
        );
        fixed(
            out,
            &[(2, name), (340, layout), (70, "0"), (280, "1"), (281, "0")],
        );
    }
    out.str(0, "ENDTAB");
}

/// BLOCKS: the model and paper space blocks (the drawing itself is in ENTITIES).
pub fn blocks(out: &mut Out) {
    out.section("BLOCKS");
    for (begin, end, owner, name) in [
        (0x18, 0x19, "17", "*Model_Space"),
        (0x1C, 0x1D, "1B", "*Paper_Space"),
    ] {
        out.str(0, "BLOCK");
        out.handle(5, begin);
        fixed(
            out,
            &[
                (330, owner),
                (100, "AcDbEntity"),
                (8, "0"),
                (100, "AcDbBlockBegin"),
                (2, name),
                (70, "0"),
                (10, "0.0"),
                (20, "0.0"),
                (30, "0.0"),
                (3, name),
                (1, ""),
                (0, "ENDBLK"),
            ],
        );
        out.handle(5, end);
        fixed(
            out,
            &[
                (330, owner),
                (100, "AcDbEntity"),
                (8, "0"),
                (100, "AcDbBlockEnd"),
            ],
        );
    }
    out.str(0, "ENDSEC");
}

fn dictionary(out: &mut Out, handle: &str, entries: &[(&str, &str)]) {
    fixed(
        out,
        &[
            (0, "DICTIONARY"),
            (5, handle),
            (330, "A"),
            (100, "AcDbDictionary"),
            (281, "1"),
        ],
    );
    for (name, target) in entries {
        out.str(3, name);
        out.str(350, target);
    }
}

fn layout(
    out: &mut Out,
    handle: &str,
    name: &str,
    plot_flags: &str,
    tab: &str,
    block_record: &str,
) {
    fixed(
        out,
        &[
            (0, "LAYOUT"),
            (5, handle),
            (330, "D"),
            (100, "AcDbPlotSettings"),
            (1, ""),
            (4, "A3"),
            (6, ""),
            (40, "7.5"),
            (41, "20.0"),
            (42, "7.5"),
            (43, "20.0"),
            (44, "420.0"),
            (45, "297.0"),
            (46, "0.0"),
            (47, "0.0"),
            (48, "0.0"),
            (49, "0.0"),
            (140, "0.0"),
            (141, "0.0"),
            (142, "1.0"),
            (143, "1.0"),
            (70, plot_flags),
            (72, "1"),
            (73, "0"),
            (74, "5"),
            (7, ""),
            (75, "16"),
            (76, "0"),
            (77, "2"),
            (78, "300"),
            (147, "1.0"),
            (148, "0.0"),
            (149, "0.0"),
            (100, "AcDbLayout"),
            (1, name),
            (70, "1"),
            (71, tab),
            (10, "0.0"),
            (20, "0.0"),
            (11, "420.0"),
            (21, "297.0"),
            (12, "0.0"),
            (22, "0.0"),
            (32, "0.0"),
            (14, "1.0E+20"),
            (24, "1.0E+20"),
            (34, "1.0E+20"),
            (15, "-1.0E+20"),
            (25, "-1.0E+20"),
            (35, "-1.0E+20"),
            (146, "0.0"),
            (13, "0.0"),
            (23, "0.0"),
            (33, "0.0"),
            (16, "1.0"),
            (26, "0.0"),
            (36, "0.0"),
            (17, "0.0"),
            (27, "1.0"),
            (37, "0.0"),
            (76, "1"),
            (330, block_record),
        ],
    );
}

fn material(out: &mut Out, handle: &str, name: &str) {
    fixed(
        out,
        &[
            (0, "MATERIAL"),
            (5, handle),
            (102, "{ACAD_REACTORS"),
            (330, "E"),
            (102, "}"),
            (330, "E"),
            (100, "AcDbMaterial"),
            (1, name),
            (2, ""),
            (70, "0"),
            (40, "1.0"),
            (71, "1"),
            (41, "1.0"),
            (91, "-1023410177"),
            (42, "1.0"),
            (72, "1"),
            (3, ""),
            (73, "1"),
            (74, "1"),
            (75, "1"),
            (44, "0.5"),
            (73, "0"),
            (45, "1.0"),
            (46, "1.0"),
            (77, "1"),
            (4, ""),
            (78, "1"),
            (79, "1"),
            (170, "1"),
            (48, "1.0"),
            (171, "1"),
            (6, ""),
            (172, "1"),
            (173, "1"),
            (174, "1"),
            (140, "1.0"),
            (141, "1.0"),
            (175, "1"),
            (7, ""),
            (176, "1"),
            (177, "1"),
            (178, "1"),
            (143, "1.0"),
            (179, "1"),
            (8, ""),
            (270, "1"),
            (271, "1"),
            (272, "1"),
            (145, "1.0"),
            (146, "1.0"),
            (273, "1"),
            (9, ""),
            (274, "1"),
            (275, "1"),
            (276, "1"),
            (42, "1.0"),
            (72, "1"),
            (3, ""),
            (73, "1"),
            (74, "1"),
            (75, "1"),
            (94, "63"),
        ],
    );
}

/// OBJECTS: the root dictionary and what it names.
pub fn objects(out: &mut Out) {
    out.section("OBJECTS");
    fixed(
        out,
        &[
            (0, "DICTIONARY"),
            (5, "A"),
            (330, "0"),
            (100, "AcDbDictionary"),
            (281, "1"),
        ],
    );
    for (name, target) in [
        ("ACAD_COLOR", "B"),
        ("ACAD_GROUP", "C"),
        ("ACAD_LAYOUT", "D"),
        ("ACAD_MATERIAL", "E"),
        ("ACAD_MLEADERSTYLE", "F"),
        ("ACAD_MLINESTYLE", "10"),
        ("ACAD_PLOTSETTINGS", "11"),
        ("ACAD_PLOTSTYLENAME", "12"),
        ("ACAD_SCALELIST", "14"),
        ("ACAD_TABLESTYLE", "15"),
        ("ACAD_VISUALSTYLE", "16"),
    ] {
        out.str(3, name);
        out.str(350, target);
    }
    dictionary(out, "B", &[]);
    dictionary(out, "C", &[]);
    dictionary(out, "D", &[("Model", "1A"), ("Layout1", "1E")]);
    dictionary(
        out,
        "E",
        &[("ByBlock", "1F"), ("ByLayer", "20"), ("Global", "21")],
    );
    dictionary(out, "F", &[("Standard", "2C")]);
    dictionary(out, "10", &[("Standard", "22")]);
    dictionary(out, "11", &[]);
    fixed(
        out,
        &[
            (0, "ACDBDICTIONARYWDFLT"),
            (5, "12"),
            (330, "A"),
            (100, "AcDbDictionary"),
            (281, "1"),
            (3, "Normal"),
            (350, "13"),
            (100, "AcDbDictionaryWithDefault"),
            (340, "13"),
            (0, "ACDBPLACEHOLDER"),
            (5, "13"),
            (330, "12"),
        ],
    );
    dictionary(out, "14", &[]);
    dictionary(out, "15", &[]);
    dictionary(out, "16", &[]);
    layout(out, "1A", "Model", "1024", "0", "17");
    layout(out, "1E", "Layout1", "0", "1", "1B");
    material(out, "1F", "ByBlock");
    material(out, "20", "ByLayer");
    material(out, "21", "Global");
    fixed(
        out,
        &[
            (0, "MLINESTYLE"),
            (5, "22"),
            (102, "{ACAD_REACTORS"),
            (330, "10"),
            (102, "}"),
            (330, "10"),
            (100, "AcDbMlineStyle"),
            (2, "Standard"),
            (70, "0"),
            (3, ""),
            (62, "256"),
            (51, "90.0"),
            (52, "90.0"),
            (71, "2"),
            (49, "0.5"),
            (62, "256"),
            (6, "BYLAYER"),
            (49, "-0.5"),
            (62, "256"),
            (6, "BYLAYER"),
            (0, "MLEADERSTYLE"),
            (5, "2C"),
            (102, "{ACAD_REACTORS"),
            (330, "F"),
            (102, "}"),
            (330, "F"),
            (100, "AcDbMLeaderStyle"),
            (179, "2"),
            (170, "2"),
            (171, "1"),
            (172, "0"),
            (90, "2"),
            (40, "0.0"),
            (41, "0.0"),
            (173, "1"),
            (91, "-1056964608"),
            (92, "-2"),
            (290, "1"),
            (42, "2.0"),
            (291, "1"),
            (43, "8.0"),
            (3, "Standard"),
            (44, "4.0"),
            (300, ""),
            (342, "29"),
            (174, "1"),
            (175, "1"),
            (176, "0"),
            (178, "1"),
            (93, "-1056964608"),
            (45, "4.0"),
            (292, "0"),
            (297, "0"),
            (46, "4.0"),
            (94, "-1056964608"),
            (47, "1.0"),
            (49, "1.0"),
            (140, "1.0"),
            (294, "1"),
            (141, "0.0"),
            (177, "0"),
            (142, "1.0"),
            (295, "0"),
            (296, "0"),
            (143, "3.75"),
            (271, "0"),
            (272, "9"),
            (273, "9"),
        ],
    );
    out.str(0, "ENDSEC");
}
