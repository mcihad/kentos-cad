//! Which modules offer each İşlemler tool (docs/adr/9310, CLAUDE.md §25 rule 2). Every built-in tool is placed by id
//! in one of six sets ([`PLACED`]), each module's yes or no stated by name; an extension's tool by its descriptor's
//! `modules` or, without them, its manifest's; a model where all its steps are. The window, the command line, Python,
//! MCP, an extension and the agent ask the same set: a project of another module lists none of the others and refuses
//! a run (`tool_not_in_module`), naming the projects that have the tool.

use kentos_domain::Document;
use kentos_launch::{Module, ModuleSet};

use crate::types::Tool;

const fn set(architecture: bool, map: bool, gis: bool, road: bool) -> ModuleSet {
    ModuleSet {
        architecture,
        map,
        gis,
        road,
    }
}

/// Every module: Mimari has no toolbox, its ribbon holds these two.
pub const EVERY: ModuleSet = set(true, true, true, true);
/// CBS, Harita and Yol: what works on real coordinates, a data table's objects or a drawing's alike. Mimari draws a
/// building on its own coordinates.
pub const SHARED: ModuleSet = set(false, true, true, true);
/// CBS and Harita: the works both do on a city's map (addresses, scenarios, topology rules, the zoning check).
pub const GIS_MAP: ModuleSet = set(false, true, true, false);
/// CBS and Yol: the water's way and the cost of crossing over a DEM, a road's drainage and corridor.
pub const GIS_ROAD: ModuleSet = set(false, false, true, true);
/// Harita and Yol: a CAD drawing's own writing, put down as text objects; a GIS labels its features instead.
pub const CAD_DRAWING: ModuleSet = set(false, true, false, true);
/// CBS alone: analyses of tables and images (networks, statistics, suitability, remote sensing, generalization).
pub const GIS: ModuleSet = set(false, false, true, false);

/// Each built-in tool's modules, by id: the ADR's table. `every_builtin_tool_is_placed_once` holds it to the
/// registry: a tool missing here, or placed twice, or an id here no tool has, fails.
pub const PLACED: &[(&str, ModuleSet)] = &[
    // Every module: choosing objects by an expression or by where they are.
    ("selection.byExpression", EVERY),
    ("selection.byLocation", EVERY),
    // CBS, Harita and Yol: attributes, conversions, geometry, layers, linear referencing, point clouds, surfaces and
    // the rasters a drawing lies on, on real coordinates in a data table and a drawing alike.
    ("attributes.calculate", SHARED),
    ("attributes.fromEnclosing", SHARED),
    ("attributes.fromInside", SHARED),
    ("attributes.joinByField", SHARED),
    ("attributes.transfer", SHARED),
    ("convert.bearingLine", SHARED),
    ("convert.densify", SHARED),
    ("convert.linesToPolygons", SHARED),
    ("convert.pointsToPath", SHARED),
    ("convert.polygonsToLines", SHARED),
    ("convert.substring", SHARED),
    ("convert.tableEllipse", SHARED),
    ("convert.tablePoints", SHARED),
    ("convert.toPoints", SHARED),
    ("convert.vertices", SHARED),
    ("convert.xyLine", SHARED),
    ("generate.bounding", SHARED),
    ("generate.delaunay", SHARED),
    ("geometry.buffer", SHARED),
    ("geometry.clip", SHARED),
    ("geometry.difference", SHARED),
    ("geometry.dissolve", SHARED),
    ("geometry.intersection", SHARED),
    ("geometry.repair", SHARED),
    ("geometry.reproject", SHARED),
    ("geometry.simplify", SHARED),
    ("geometry.symDifference", SHARED),
    ("geometry.union", SHARED),
    ("geometry.validity", SHARED),
    ("interpolation.idw", SHARED),
    ("interpolation.tin", SHARED),
    ("layers.deleteDuplicates", SHARED),
    ("layers.extract", SHARED),
    ("layers.merge", SHARED),
    ("layers.orderBy", SHARED),
    ("layers.splitByAttribute", SHARED),
    ("linearRef.calibrateRoutes", SHARED),
    ("linearRef.createRoutes", SHARED),
    ("linearRef.eventDrift", SHARED),
    ("linearRef.extractMeasures", SHARED),
    ("linearRef.locateAlong", SHARED),
    ("linearRef.overlayEvents", SHARED),
    ("linearRef.routeEvents", SHARED),
    ("pointcloud.areaStats", SHARED),
    ("pointcloud.boundary", SHARED),
    ("pointcloud.classify", SHARED),
    ("pointcloud.clip", SHARED),
    ("pointcloud.ground", SHARED),
    ("pointcloud.merge", SHARED),
    ("pointcloud.rasterize", SHARED),
    ("pointcloud.thin", SHARED),
    ("pointcloud.tile", SHARED),
    ("pointcloud.transform", SHARED),
    ("raster.clipByMask", SHARED),
    ("raster.cog", SHARED),
    ("raster.mosaic", SHARED),
    ("raster.resample", SHARED),
    ("raster.toPolygons", SHARED),
    ("sample.alongLines", SHARED),
    ("sample.grid", SHARED),
    ("sample.regular", SHARED),
    ("scan.captureLine", SHARED),
    ("scan.closeArea", SHARED),
    ("scan.contourElevations", SHARED),
    ("statistics.summary", SHARED),
    ("surface.aspect", SHARED),
    ("surface.colorRelief", SHARED),
    ("surface.contours", SHARED),
    ("surface.hillshade", SHARED),
    ("surface.slope", SHARED),
    // CBS and Harita: addresses, a plan's scenarios, the topology rules and the zoning check.
    ("address.geocode", GIS_MAP),
    ("address.reverse", GIS_MAP),
    ("remote.composite", GIS_MAP),
    ("scenarios.links", GIS_MAP),
    ("topology.check", GIS_MAP),
    ("zoning.check", GIS_MAP),
    // CBS and Yol: the water's way and the cost of crossing over a DEM (drainage, corridors).
    ("distance.corridor", GIS_ROAD),
    ("distance.cost", GIS_ROAD),
    ("distance.euclidean", GIS_ROAD),
    ("distance.path", GIS_ROAD),
    ("hydrology.basins", GIS_ROAD),
    ("hydrology.fill", GIS_ROAD),
    ("hydrology.flowAccumulation", GIS_ROAD),
    ("hydrology.flowDirection", GIS_ROAD),
    ("hydrology.pourPoint", GIS_ROAD),
    ("hydrology.streams", GIS_ROAD),
    ("hydrology.watershed", GIS_ROAD),
    ("hydrology.wetness", GIS_ROAD),
    // Harita and Yol: a drawing's own writing, edge lengths and numbered corners as text objects.
    ("annotation.edgeLengths", CAD_DRAWING),
    ("points.numberVertices", CAD_DRAWING),
    // CBS alone: analyses of tables and images a drawing does not hold.
    ("density.kernel", GIS),
    ("density.line", GIS),
    ("generalize.aggregate", GIS),
    ("generalize.building", GIS),
    ("generalize.coverageSimplify", GIS),
    ("generalize.dropParts", GIS),
    ("generalize.eliminate", GIS),
    ("generalize.orthogonalize", GIS),
    ("generalize.smooth", GIS),
    ("generate.concaveHull", GIS),
    ("generate.pole", GIS),
    ("generate.voronoi", GIS),
    ("interpolation.kriging", GIS),
    ("interpolation.naturalNeighbor", GIS),
    ("interpolation.spline", GIS),
    ("multidim.meshCalculator", GIS),
    ("multidim.profile", GIS),
    ("multidim.series", GIS),
    ("network.closestFacility", GIS),
    ("network.odMatrix", GIS),
    ("network.serviceAreas", GIS),
    ("proximity.hub", GIS),
    ("proximity.matrix", GIS),
    ("proximity.nearest", GIS),
    ("proximity.neighbors", GIS),
    ("proximity.shortestLine", GIS),
    ("raster.calculator", GIS),
    ("raster.cellStatistics", GIS),
    ("raster.focalStatistics", GIS),
    ("raster.histogram", GIS),
    ("raster.rasterize", GIS),
    ("raster.reclassify", GIS),
    ("raster.toLines", GIS),
    ("raster.toPoints", GIS),
    ("raster.zonalStatistics", GIS),
    ("remote.accuracy", GIS),
    ("remote.change", GIS),
    ("remote.index", GIS),
    ("remote.pansharpen", GIS),
    ("remote.qaMask", GIS),
    ("remote.split", GIS),
    ("remote.supervised", GIS),
    ("remote.unsupervised", GIS),
    ("sample.random", GIS),
    ("sample.subdivide", GIS),
    ("stats.dbscan", GIS),
    ("stats.dbscanSweep", GIS),
    ("stats.directionalDistribution", GIS),
    ("stats.hotSpot", GIS),
    ("stats.kMeans", GIS),
    ("stats.localMoran", GIS),
    ("stats.meanCenter", GIS),
    ("stats.medianCenter", GIS),
    ("stats.moransI", GIS),
    ("stats.nearestNeighbor", GIS),
    ("stats.standardDistance", GIS),
    ("suitability.fuzzyMembership", GIS),
    ("suitability.fuzzyOverlay", GIS),
    ("suitability.pairwise", GIS),
    ("suitability.roc", GIS),
    ("suitability.weightedOverlay", GIS),
    ("suitability.weightedSum", GIS),
    ("surface.curvature", GIS),
    ("surface.insolation", GIS),
    ("surface.ruggedness", GIS),
];

/// A built-in tool's modules; none for an id the table lacks (offered nowhere until it is placed).
pub fn placed(id: &str) -> ModuleSet {
    PLACED
        .iter()
        .find(|(t, _)| *t == id)
        .map_or(ModuleSet::NONE, |(_, s)| *s)
}

/// The module a drawing's project belongs to, by its type and CAD work; none for one not asked them yet.
pub fn module_of(doc: &Document) -> Option<Module> {
    let settings = doc.settings();
    Module::of_project(
        settings.project_type().map(|t| t.id()),
        settings.cad_work().map(|w| w.id()),
    )
}

/// A module's projects' word in a list: “harita ve yol projelerinde”.
fn project_word(module: Module) -> &'static str {
    match module {
        Module::Architecture => "mimari",
        Module::Map => "harita",
        Module::Gis => "CBS",
        Module::Road => "yol",
    }
}

/// Whether the tool runs on this drawing: its project's module is one of the tool's, or the project's module is not
/// known yet.
pub fn runs_on(doc: &Document, tool: &Tool) -> bool {
    module_of(doc).is_none_or(|m| tool.modules.has(m))
}

/// Why the tool does not run on this drawing (`tool_not_in_module`): the projects that have it; none when it runs.
pub fn refusal(doc: &Document, tool: &Tool) -> Option<String> {
    let here = module_of(doc)?;
    if tool.modules.has(here) {
        return None;
    }
    let there: Vec<&str> = tool.modules.modules().into_iter().map(project_word).collect();
    Some(match there.as_slice() {
        [] => format!("“{}” hiçbir projede çalışmaz: aracın modülü yok.", tool.label),
        [one] => format!("“{}” {} yok; {one} projelerinde var.", tool.label, here.in_project()),
        [rest @ .., last] => format!(
            "“{}” {} yok; {} ve {last} projelerinde var.",
            tool.label,
            here.in_project(),
            rest.join(", ")
        ),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// Every built-in tool is placed once, and every placed id is a tool (CLAUDE.md §25 rule 2: no tool is offered
    /// everywhere by default); each tool's set is the table's and names at least one module.
    #[test]
    fn every_builtin_tool_is_placed_once() {
        let tools = crate::builtin::tools();
        let ids: BTreeSet<&str> = tools.iter().map(|t| t.id.as_str()).collect();
        let mut placed_ids = BTreeSet::new();
        for (id, set) in PLACED {
            assert!(placed_ids.insert(*id), "{id} is placed twice");
            assert!(ids.contains(id), "{id} is placed but no built-in tool has it");
            assert!(!set.is_empty(), "{id} is placed in no module");
        }
        for t in &tools {
            assert!(placed_ids.contains(t.id.as_str()), "{} is not placed: add it to PLACED", t.id);
            assert_eq!(t.modules, placed(&t.id), "{}", t.id);
        }
    }

    /// The sample parcels as a project of `module`'s type and work.
    fn drawing(module: Module) -> Document {
        use kentos_commands::{CadWork, Workspace};
        let text = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/processing/v1/parcels.kcad"),
        )
        .expect("the drawing");
        let mut doc =
            Document::from_snapshot(kentos_commands::DocumentSnapshotV1::from_json(&text).expect("reads")).expect("opens");
        let mut settings = doc.settings().clone();
        settings.workspace = Some(if module == Module::Gis { Workspace::Gis } else { Workspace::Cad });
        settings.cad_work = match module {
            Module::Architecture => Some(CadWork::Architecture),
            Module::Map => Some(CadWork::Map),
            Module::Road => Some(CadWork::Road),
            Module::Gis => None,
        };
        doc.set_settings(settings);
        assert_eq!(module_of(&doc), Some(module));
        doc
    }

    /// Python's, MCP's and an agent's way (`cad.processing.run`, `.tools`) asks the same sets as the window: a project
    /// of another module refuses the run, naming the projects that have the tool, and lists none of the others.
    #[test]
    fn a_project_of_another_module_refuses_the_tool_and_does_not_list_it() {
        use kentos_native_application::processing_host::ToolHost;
        let registry = crate::registry::Registry::builtin();
        let run = |tool: &str| -> kentos_commands::ProcessingRun {
            serde_json::from_value(serde_json::json!({ "tool": tool })).expect("a run")
        };
        let refused = |module: Module, tool: &str| match crate::command::ready(&registry, &drawing(module), &run(tool)) {
            Err(e) => Some((e.code.clone(), e.message.clone())),
            Ok(_) => None,
        };
        let (code, message) = refused(Module::Architecture, "network.serviceAreas").expect("refused");
        assert_eq!(code, kentos_commands::codes::TOOL_NOT_IN_MODULE);
        assert_eq!(message, "“Hizmet alanları” bu mimari projede yok; CBS projelerinde var.");
        let (_, message) = refused(Module::Gis, "annotation.edgeLengths").expect("refused");
        assert_eq!(message, "“Kenar uzunluklarını yaz” bu CBS projesinde yok; harita ve yol projelerinde var.");
        // Where it is offered, the module is no reason to refuse it.
        assert!(
            refused(Module::Map, "annotation.edgeLengths")
                .is_none_or(|(code, _)| code != kentos_commands::codes::TOOL_NOT_IN_MODULE)
        );
        let listed = |module: Module, query: &str| -> Vec<String> {
            let input: kentos_commands::ProcessingTools =
                serde_json::from_value(serde_json::json!({ "query": query })).expect("a query");
            crate::command::Tools
                .list(&drawing(module), &input)
                .expect("lists")
                .tools
                .into_iter()
                .map(|t| t.id)
                .collect()
        };
        assert!(listed(Module::Map, "hizmet alanları").is_empty());
        assert_eq!(listed(Module::Gis, "hizmet alanları"), ["network.serviceAreas"]);
        let edge = listed(Module::Road, "annotation.edgeLengths");
        assert_eq!(edge, ["annotation.edgeLengths"]);
    }

    /// The matrix, cell by cell: each set's modules as the ADR's table says them.
    #[test]
    fn each_set_says_each_module() {
        let row = |s: ModuleSet| Module::ALL.map(|m| s.has(m));
        // Mimari, Harita, CBS, Yol.
        assert_eq!(row(EVERY), [true, true, true, true]);
        assert_eq!(row(SHARED), [false, true, true, true]);
        assert_eq!(row(GIS_MAP), [false, true, true, false]);
        assert_eq!(row(GIS_ROAD), [false, false, true, true]);
        assert_eq!(row(CAD_DRAWING), [false, true, false, true]);
        assert_eq!(row(GIS), [false, false, true, false]);
    }
}
