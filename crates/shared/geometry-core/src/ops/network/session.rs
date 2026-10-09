//! A network the apps ask again and again (docs/adr/0209 §10, §12): built
//! once from the geometry store, then asked for places, routes, the way to
//! the cursor, service areas, closest facilities, traces and its check. The
//! answers are typed (the desktop's network thread reads them) and written
//! as the contract-like JSON the web reads (its worker holds one) and Python.
//! Places are given as points with how far to look; a point that finds no
//! network is said by its index. The way to the cursor is numbers, not JSON:
//! it is asked on every pointer move.

use super::area::{AreaQuery, service_lines, service_polygons};
use super::check::{Checked, ProblemKind, check};
use super::closest::{Nearest, nearest};
use super::graph::{BuildReport, Graph, Location, Role};
use super::input::{EdgeValues, JunctionValues, from_store};
use super::route::{Reorder, RouteError, route};
use super::rules::Rules;
use super::search::{Query, Searcher, Span, Tree, polyline_of, spans_cost, spans_edges};
use super::trace::{TraceKind, trace};
use crate::api::json::{self, Json};
use crate::entity::Shape;
use crate::geom::bulge::bulge_of_sweep;
use crate::geom::intersect::{Edge, point_at};
use crate::jsmath::js_min;
use crate::ops::parts::one_area;
use crate::store::Store;
use crate::vec2::Vec2;

/// A built network and the buffers its searches use again.
pub struct NetworkSession {
    pub graph: Graph,
    pub report: BuildReport,
    /// Objects of the network's layers not taken: their ids and kinds.
    pub skipped: Vec<(f64, &'static str)>,
    searcher: Searcher,
    tree: Option<Tree>,
}

/// A way along the network: its points and one bulge a segment (arcs exact).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Line {
    pub pts: Vec<[f64; 2]>,
    pub bulges: Vec<f64>,
}

crate::json_struct!(out Line { pts, bulges });

/// Where a point sits on the network: its piece, how far along it, the point there and how far it was.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub piece: u32,
    pub offset: f64,
    pub x: f64,
    pub y: f64,
    pub d: f64,
}

crate::json_struct!(out Place { piece, offset, x, y, d });

/// What a point that finds no network was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Missing {
    Stop,
    Barrier,
    Facility,
    Origin,
    Target,
    Start,
}

impl Missing {
    fn name(self) -> &'static str {
        match self {
            Missing::Stop => "stopNotFound",
            Missing::Barrier => "barrierNotFound",
            Missing::Facility => "facilityNotFound",
            Missing::Origin => "originNotFound",
            Missing::Target => "targetNotFound",
            Missing::Start => "startNotFound",
        }
    }
}

/// Why a question has no answer: a point (by its index) that finds no network, too few or too many stops, no order
/// of them that can be travelled, two stops (by their order) with no way between, breaks that are not rising numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    NotFound(Missing, usize),
    TooFew,
    TooMany,
    NoOrder,
    Unreachable(usize, usize),
    Breaks,
}

impl Refusal {
    /// The refusal in words (`within`: the search distance as written, “20 piksel” in the tools); the web's
    /// `refusalWords`.
    pub fn words(&self, within: &str) -> String {
        let place = |m: Missing| match m {
            Missing::Stop => "durak",
            Missing::Barrier => "engel",
            Missing::Facility => "tesis",
            Missing::Origin | Missing::Start => "başlangıç",
            Missing::Target => "varış",
        };
        match *self {
            Refusal::NotFound(m, at) => {
                format!("{}. {} ağa {within} içinde değil.", at + 1, place(m))
            }
            Refusal::TooFew => "En az iki durak gerekir.".into(),
            Refusal::TooMany => {
                "Sıra en çok 12 durakla iyileştirilir; durakları azaltın ya da Sıra’yı kapatın."
                    .into()
            }
            Refusal::NoOrder => {
                "Durakları her sırayla gezen bir yol yok; bazı duraklar birbirine ulaşamıyor."
                    .into()
            }
            Refusal::Unreachable(a, b) => {
                format!(
                    "{}. ve {}. duraklar arasında yol yok (kapalı kenar, tek yön ya da kopuk ağ).",
                    a + 1,
                    b + 1
                )
            }
            Refusal::Breaks => "Aralıklar artan, sıfırdan büyük sayılar olmalı.".into(),
        }
    }

    /// The refusal as the JSON answers write it: `{error, at?, between?}`.
    pub fn json(&self) -> String {
        match self {
            Refusal::NotFound(m, i) => format!(r#"{{"error":"{}","at":{i}}}"#, m.name()),
            Refusal::TooFew => r#"{"error":"tooFew"}"#.into(),
            Refusal::TooMany => r#"{"error":"tooMany"}"#.into(),
            Refusal::NoOrder => r#"{"error":"noOrder"}"#.into(),
            Refusal::Unreachable(a, b) => {
                format!(r#"{{"error":"unreachable","between":[{a},{b}]}}"#)
            }
            Refusal::Breaks => r#"{"error":"breaks"}"#.into(),
        }
    }
}

/// A route: the stops' order, its cost, each cost's total along it (none: a part it cannot be travelled with), its
/// way and its legs' costs.
#[derive(Clone, Debug, PartialEq)]
pub struct Routed {
    pub order: Vec<usize>,
    pub cost: f64,
    pub totals: Vec<Option<f64>>,
    pub line: Line,
    pub legs: Vec<f64>,
}

crate::json_struct!(out Routed { order, cost, totals, line, legs });

/// A service area's line: its facility (none: merged), its band and its way.
#[derive(Clone, Debug, PartialEq)]
pub struct ServedLine {
    pub facility: Option<usize>,
    pub band: usize,
    pub line: Line,
}

crate::json_struct!(out ServedLine { facility, band, line });

/// A service area's area: its facility, its band and its polygon (none: nothing reached).
#[derive(Clone, Debug, PartialEq)]
pub struct ServedArea {
    pub facility: Option<usize>,
    pub band: usize,
    pub shape: Option<Shape>,
}

crate::json_struct!(out ServedArea { facility, band, shape });

/// A service area: its lines (by facility, band, piece and start) and its areas.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Served {
    pub lines: Vec<ServedLine>,
    pub areas: Vec<ServedArea>,
}

crate::json_struct!(out Served { lines, areas });

/// A target found from an origin: its index, its cost, and when its way was asked each cost along it and the way.
#[derive(Clone, Debug, PartialEq)]
pub struct Reached {
    pub target: usize,
    pub cost: f64,
    pub totals: Option<Vec<Option<f64>>>,
    pub line: Option<Line>,
}

crate::json_struct!(out Reached { target, cost, totals, line });

/// A valve to close: its object and where it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Valve {
    pub id: f64,
    pub x: f64,
    pub y: f64,
}

crate::json_struct!(out Valve { id, x, y });

/// A trace: the objects reached, the valves to close and what is no longer fed (their objects' ids), their lengths
/// and lines.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TraceAnswer {
    pub objects: Vec<f64>,
    pub valves: Vec<Valve>,
    pub unfed: Vec<f64>,
    pub length: f64,
    pub unfed_length: f64,
    pub lines: Vec<Line>,
    pub unfed_lines: Vec<Line>,
}

crate::json_struct!(out TraceAnswer { objects, valves, unfed, length, unfed_length => "unfedLength", lines, unfed_lines => "unfedLines" });

static NULL: Json = Json::Null;

/// An array's item `i`; `null` past its end or for another value.
fn item(v: &Json, i: usize) -> &Json {
    match v {
        Json::Arr(list) => list.get(i).unwrap_or(&NULL),
        _ => &NULL,
    }
}

fn parse(text: &str, what: &str) -> Result<Json, String> {
    Json::parse(text).map_err(|e| format!("ağın {what} okunamadı: {e}"))
}

/// Points written as `[[x, y], …]`.
pub fn points_of(text: &str) -> Result<Vec<Vec2>, String> {
    match parse(text, "noktaları")? {
        Json::Arr(list) => list
            .iter()
            .map(|p| match p {
                Json::Arr(xy) if xy.len() == 2 => match (&xy[0], &xy[1]) {
                    (Json::Num(x), Json::Num(y)) => Ok(Vec2::new(*x, *y)),
                    _ => Err("bir nokta iki sayı olmalı".to_string()),
                },
                _ => Err("bir nokta iki sayı olmalı".to_string()),
            })
            .collect(),
        _ => Err("noktalar liste olmalı".into()),
    }
}

fn line_of(g: &Graph, spans: &[Span]) -> Line {
    line_of_edges(&spans_edges(g, spans))
}

/// Each span's own way (a trace's stretches), their primitives found into one buffer.
fn span_lines(g: &Graph, spans: &[Span]) -> Vec<Line> {
    let mut edges = Vec::new();
    spans
        .iter()
        .map(|s| {
            edges.clear();
            g.span_edges(s.piece, s.a, s.b, &mut edges);
            line_of_edges(&edges)
        })
        .collect()
}

/// Primitives in a row as a way (`polyline_of`'s points and bulges, written straight into the answer's arrays).
fn line_of_edges(edges: &[Edge]) -> Line {
    let mut pts: Vec<[f64; 2]> = Vec::with_capacity(edges.len() + 1);
    let mut bulges: Vec<f64> = Vec::with_capacity(edges.len());
    for e in edges {
        let a = point_at(e, 0.0);
        match pts.last() {
            None => pts.push([a.x, a.y]),
            Some(&[x, y]) if x != a.x || y != a.y => {
                bulges.push(0.0);
                pts.push([a.x, a.y]);
            }
            _ => {}
        }
        bulges.push(match *e {
            Edge::Seg { .. } => 0.0,
            Edge::Arc { sweep, .. } => bulge_of_sweep(sweep),
        });
        let b = point_at(e, 1.0);
        pts.push([b.x, b.y]);
    }
    Line { pts, bulges }
}

/// A number that may be none (a cost a way cannot be travelled with).
fn finite(x: f64) -> Option<f64> {
    x.is_finite().then_some(x)
}

/// An answer or its refusal, as the JSON answers write them.
fn json_of<T: json::ToJson>(r: Result<T, Refusal>) -> String {
    match r {
        Ok(v) => json::to_string(&v),
        Err(e) => e.json(),
    }
}

/// A problem kind's name in the JSON answers.
pub fn problem_name(k: ProblemKind) -> &'static str {
    match k {
        ProblemKind::Detached => "detached",
        ProblemKind::NearMiss => "nearMiss",
        ProblemKind::Crossing => "crossing",
        ProblemKind::OffNetwork => "offNetwork",
        ProblemKind::Short => "short",
        ProblemKind::Unread => "unread",
    }
}

impl NetworkSession {
    /// A session over a graph built elsewhere (the desktop and the tests).
    pub fn new(
        graph: Graph,
        report: BuildReport,
        skipped: Vec<(f64, &'static str)>,
    ) -> NetworkSession {
        NetworkSession {
            graph,
            report,
            skipped,
            searcher: Searcher::new(),
            tree: None,
        }
    }

    /// The network of definition `def` (the contract's `NetworkDef` JSON) over `store`'s objects: `edges` with their
    /// values (`[direction | null, [cost values], closed]` each) and `junctions` with theirs (`[role, closed]` each).
    pub fn from_store(
        store: &Store,
        def: &str,
        edges: &[f64],
        edge_values: &str,
        junctions: &[f64],
        junction_values: &str,
    ) -> Result<NetworkSession, String> {
        let rules = Rules::from_json(&parse(def, "tanımı")?)?;
        let text = |v: &Json| match v {
            Json::Str(s) => Some(s.clone()),
            Json::Num(n) => Some(json::to_string(n)),
            _ => None,
        };
        let ev = match parse(edge_values, "kenar değerleri")? {
            Json::Arr(list) if list.len() == edges.len() => list,
            _ => return Err("kenarların değerleri kenar başına bir olmalı".into()),
        };
        let edge_list: Vec<(f64, EdgeValues)> = edges
            .iter()
            .zip(&ev)
            .map(|(id, v)| {
                let costs = match item(v, 1) {
                    Json::Arr(c) => c.iter().map(text).collect(),
                    _ => Vec::new(),
                };
                (
                    *id,
                    EdgeValues {
                        direction: text(item(v, 0)),
                        costs,
                        closed: matches!(item(v, 2), Json::Bool(true)),
                    },
                )
            })
            .collect();
        let jv = match parse(junction_values, "düğüm değerleri")? {
            Json::Arr(list) if list.len() == junctions.len() => list,
            _ => return Err("düğümlerin değerleri düğüm başına bir olmalı".into()),
        };
        let junction_list: Vec<(f64, JunctionValues)> = junctions
            .iter()
            .zip(&jv)
            .map(|(id, v)| {
                let role = match item(v, 0) {
                    Json::Str(s) if s == "source" => Role::Source,
                    Json::Str(s) if s == "valve" => Role::Valve,
                    _ => Role::Junction,
                };
                (
                    *id,
                    JunctionValues {
                        role,
                        closed: matches!(item(v, 1), Json::Bool(true)),
                    },
                )
            })
            .collect();
        let (e, j, skipped) = from_store(store, &edge_list, &junction_list);
        let (graph, report) = Graph::build(rules, &e, &j);
        Ok(NetworkSession::new(graph, report, skipped))
    }

    /// Where the network is near each point within `reach`; a refusal names the first point that finds none.
    fn locate_all(
        &self,
        pts: &[Vec2],
        reach: f64,
        what: Missing,
    ) -> Result<Vec<Location>, Refusal> {
        pts.iter()
            .enumerate()
            .map(|(i, p)| {
                self.graph
                    .locate(*p, reach)
                    .ok_or(Refusal::NotFound(what, i))
            })
            .collect()
    }

    /// The network's counts and what building it said.
    pub fn summary_json(&self) -> String {
        struct Skipped {
            id: f64,
            kind: &'static str,
        }
        crate::json_struct!(out Skipped { id, kind });
        struct Summary {
            nodes: usize,
            pieces: usize,
            length: f64,
            skipped: Vec<Skipped>,
            short: usize,
            off_network: usize,
            unread: usize,
        }
        crate::json_struct!(out Summary { nodes, pieces, length, skipped, short, off_network => "offNetwork", unread });
        json::to_string(&Summary {
            nodes: self.graph.nodes.len(),
            pieces: self.graph.pieces.len(),
            length: self.graph.length(),
            skipped: self
                .skipped
                .iter()
                .map(|&(id, kind)| Skipped { id, kind })
                .collect(),
            short: self.report.short.len(),
            off_network: self.report.off_network.len(),
            unread: self.report.unread.len(),
        })
    }

    /// The place nearest `at` within `reach`, if any.
    pub fn locate(&self, at: Vec2, reach: f64) -> Option<Place> {
        self.graph.locate(at, reach).map(|l| Place {
            piece: l.piece,
            offset: l.offset,
            x: l.p.x,
            y: l.p.y,
            d: l.d,
        })
    }

    /// The place nearest `(x, y)` within `reach`: `{piece, offset, x, y, d}` or `null`.
    pub fn locate_json(&self, x: f64, y: f64, reach: f64) -> String {
        json::to_string(&self.locate(Vec2::new(x, y), reach))
    }

    /// A route through `stops` not passing `barriers`, each found within `reach`, with cost `cost` (past the
    /// network's costs: its last), the stops reordered as `reorder` says.
    pub fn route(
        &mut self,
        stops: &[Vec2],
        barriers: &[Vec2],
        reach: f64,
        cost: usize,
        reorder: Reorder,
    ) -> Result<Routed, Refusal> {
        let stops = self.locate_all(stops, reach, Missing::Stop)?;
        let barriers = self.locate_all(barriers, reach, Missing::Barrier)?;
        let g = &self.graph;
        match route(
            g,
            &mut self.searcher,
            &stops,
            &barriers,
            cost.min(g.costs - 1),
            reorder,
        ) {
            Ok(r) => Ok(Routed {
                totals: (0..g.costs).map(|c| r.total(g, c)).collect(),
                line: line_of(g, &r.spans),
                legs: r.legs.iter().map(|l| l.cost).collect(),
                order: r.order,
                cost: r.cost,
            }),
            Err(RouteError::TooFew) => Err(Refusal::TooFew),
            Err(RouteError::TooMany) => Err(Refusal::TooMany),
            Err(RouteError::NoOrder) => Err(Refusal::NoOrder),
            Err(RouteError::Unreachable(a, b)) => Err(Refusal::Unreachable(a, b)),
        }
    }

    /// `route` with its points as JSON (`[[x, y], …]`) and `reorder` `none`, `keepFirst` or `keepFirstLast`:
    /// `{order, cost, totals, line, legs}` or `{error, at?, between?}`.
    pub fn route_json(
        &mut self,
        stops: &str,
        barriers: &str,
        reach: f64,
        cost: usize,
        reorder: &str,
    ) -> Result<String, String> {
        let (stops, barriers) = (points_of(stops)?, points_of(barriers)?);
        let reorder = match reorder {
            "keepFirst" => Reorder::KeepFirst,
            "keepFirstLast" => Reorder::KeepFirstLast,
            _ => Reorder::None,
        };
        Ok(json_of(self.route(&stops, &barriers, reach, cost, reorder)))
    }

    /// Searches from `at` and keeps the tree for [`Self::path_to`]: the rubber band of En kısa yol; false when the
    /// point finds no network. Barriers that find none are not barriers.
    pub fn tree_at(&mut self, at: Vec2, reach: f64, cost: usize, barriers: &[Vec2]) -> bool {
        if let Some(t) = self.tree.take() {
            self.searcher.recycle(t);
        }
        let Some(origin) = self.graph.locate(at, reach) else {
            return false;
        };
        let barriers: Vec<Location> = barriers
            .iter()
            .filter_map(|p| self.graph.locate(*p, reach))
            .collect();
        let g = &self.graph;
        let c = cost.min(g.costs - 1);
        self.tree = Some(self.searcher.search(
            g,
            &Query {
                barriers: &barriers,
                ..Query::from(std::slice::from_ref(&origin), c)
            },
        ));
        true
    }

    /// `tree_at` with its barriers as JSON.
    pub fn tree_from(
        &mut self,
        x: f64,
        y: f64,
        reach: f64,
        cost: usize,
        barriers: &str,
    ) -> Result<bool, String> {
        let barriers = points_of(barriers)?;
        Ok(self.tree_at(Vec2::new(x, y), reach, cost, &barriers))
    }

    /// The kept tree's way to `(x, y)` within `reach`, into `out`: `[cost, n, x0, y0, … x(n−1), y(n−1), bulge0, …]`;
    /// left empty when there is no tree, no network near or no way.
    pub fn path_to(&self, x: f64, y: f64, reach: f64, out: &mut Vec<f64>) {
        out.clear();
        let (Some(t), Some(loc)) = (&self.tree, self.graph.locate(Vec2::new(x, y), reach)) else {
            return;
        };
        let Some(p) = t.path_to(&self.graph, &loc) else {
            return;
        };
        let (pts, bulges) = polyline_of(&spans_edges(&self.graph, &p.spans));
        out.push(p.cost);
        out.push(pts.len() as f64);
        for q in &pts {
            out.push(q.x);
            out.push(q.y);
        }
        out.extend(bulges);
    }

    /// A service area of `facilities` with `breaks` (rising, above zero): its lines and, when `areas`, its areas
    /// (`rings`: each band less the one below) `trim` around the lines.
    #[allow(clippy::too_many_arguments)]
    pub fn service_area(
        &mut self,
        facilities: &[Vec2],
        breaks: &[f64],
        reach: f64,
        cost: usize,
        toward: bool,
        separate: bool,
        barriers: &[Vec2],
        trim: f64,
        rings: bool,
        areas: bool,
    ) -> Result<Served, Refusal> {
        let facilities = self.locate_all(facilities, reach, Missing::Facility)?;
        let barriers = self.locate_all(barriers, reach, Missing::Barrier)?;
        if breaks.is_empty() || breaks.windows(2).any(|w| w[1] <= w[0]) || breaks[0] <= 0.0 {
            return Err(Refusal::Breaks);
        }
        let g = &self.graph;
        let q = AreaQuery {
            facilities: &facilities,
            breaks,
            cost: cost.min(g.costs - 1),
            toward,
            separate,
            barriers: &barriers,
        };
        let lines = service_lines(g, &mut self.searcher, &q);
        let polygons = if areas {
            service_polygons(g, &lines, breaks.len(), trim, rings)
        } else {
            Vec::new()
        };
        // The lines in one order wherever they are written: by facility (merged first), band, piece and where they start.
        let mut sorted: Vec<&super::area::AreaLine> = lines.iter().collect();
        sorted.sort_by(|a, b| {
            let key = |f: Option<usize>| f.map_or(0, |x| x + 1);
            key(a.facility)
                .cmp(&key(b.facility))
                .then(a.band.cmp(&b.band))
                .then(a.span.piece.cmp(&b.span.piece))
                .then(js_min(a.span.a, a.span.b).total_cmp(&js_min(b.span.a, b.span.b)))
        });
        Ok(Served {
            lines: sorted
                .into_iter()
                .map(|l| ServedLine {
                    facility: l.facility,
                    band: l.band,
                    line: line_of(g, std::slice::from_ref(&l.span)),
                })
                .collect(),
            areas: polygons
                .into_iter()
                .map(|(facility, band, a)| ServedArea {
                    facility,
                    band,
                    shape: one_area(&a).map(|e| e.shape),
                })
                .collect(),
        })
    }

    /// `service_area` with its points and breaks as JSON: `{lines: [{facility, band, line}], areas: [{facility, band,
    /// shape}]}` or `{error, at}`.
    #[allow(clippy::too_many_arguments)]
    pub fn area_json(
        &mut self,
        facilities: &str,
        breaks: &str,
        reach: f64,
        cost: usize,
        toward: bool,
        separate: bool,
        barriers: &str,
        trim: f64,
        rings: bool,
        areas: bool,
    ) -> Result<String, String> {
        let (facilities, barriers) = (points_of(facilities)?, points_of(barriers)?);
        let breaks: Vec<f64> = match parse(breaks, "aralıkları")? {
            Json::Arr(list) => list
                .iter()
                .filter_map(|v| if let Json::Num(n) = v { Some(*n) } else { None })
                .collect(),
            _ => return Err("aralıklar liste olmalı".into()),
        };
        Ok(json_of(self.service_area(
            &facilities,
            &breaks,
            reach,
            cost,
            toward,
            separate,
            &barriers,
            trim,
            rings,
            areas,
        )))
    }

    /// From each of `origins` the `k` cheapest of `targets` within `cutoff` (`reverse`: the costs from the targets),
    /// with their ways when `paths`.
    #[allow(clippy::too_many_arguments)]
    pub fn nearest_of(
        &mut self,
        origins: &[Vec2],
        targets: &[Vec2],
        reach: f64,
        k: Option<usize>,
        cutoff: Option<f64>,
        cost: usize,
        reverse: bool,
        barriers: &[Vec2],
        paths: bool,
    ) -> Result<Vec<Vec<Reached>>, Refusal> {
        let origins = self.locate_all(origins, reach, Missing::Origin)?;
        let targets = self.locate_all(targets, reach, Missing::Target)?;
        let barriers = self.locate_all(barriers, reach, Missing::Barrier)?;
        let g = &self.graph;
        let q = Nearest {
            origins: &origins,
            targets: &targets,
            k,
            cutoff,
            cost: cost.min(g.costs - 1),
            reverse,
            barriers: &barriers,
        };
        Ok(nearest(g, &mut self.searcher, &q, paths)
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|(f, path)| Reached {
                        target: f.target,
                        cost: f.cost,
                        totals: path.as_ref().map(|p| {
                            (0..g.costs)
                                .map(|c| finite(spans_cost(g, &p.spans, c)))
                                .collect()
                        }),
                        line: path.as_ref().map(|p| line_of(g, &p.spans)),
                    })
                    .collect()
            })
            .collect())
    }

    /// `nearest_of` with its points as JSON: `[[{target, cost, totals, line?}], …]` or `{error, at}`.
    #[allow(clippy::too_many_arguments)]
    pub fn nearest_json(
        &mut self,
        origins: &str,
        targets: &str,
        reach: f64,
        k: Option<usize>,
        cutoff: Option<f64>,
        cost: usize,
        reverse: bool,
        barriers: &str,
        paths: bool,
    ) -> Result<String, String> {
        let (origins, targets, barriers) = (
            points_of(origins)?,
            points_of(targets)?,
            points_of(barriers)?,
        );
        Ok(json_of(self.nearest_of(
            &origins, &targets, reach, k, cutoff, cost, reverse, &barriers, paths,
        )))
    }

    /// A trace of `kind` from `starts`: the objects reached, the valves to close and what is no longer fed (their
    /// objects' ids), their lengths and lines.
    pub fn trace_of(
        &self,
        starts: &[Vec2],
        barriers: &[Vec2],
        reach: f64,
        kind: TraceKind,
    ) -> Result<TraceAnswer, Refusal> {
        let starts = self.locate_all(starts, reach, Missing::Start)?;
        let barriers = self.locate_all(barriers, reach, Missing::Barrier)?;
        let g = &self.graph;
        let t = trace(g, &starts, &barriers, kind);
        let ids = |edges: &[u32]| -> Vec<f64> {
            let mut out: Vec<f64> = edges.iter().map(|&e| g.edges[e as usize].id).collect();
            out.dedup();
            out
        };
        let unfed_spans: Vec<Span> = t
            .unfed
            .iter()
            .map(|&p| Span {
                piece: p,
                a: 0.0,
                b: g.pieces[p as usize].len,
            })
            .collect();
        Ok(TraceAnswer {
            objects: ids(&t.edges),
            valves: t
                .valves
                .iter()
                .map(|&j| {
                    let jm = &g.junctions[j as usize];
                    Valve {
                        id: jm.id,
                        x: jm.p.x,
                        y: jm.p.y,
                    }
                })
                .collect(),
            unfed: ids(&t.unfed_edges),
            length: t.length,
            unfed_length: t.unfed_length,
            lines: span_lines(g, &t.spans),
            unfed_lines: span_lines(g, &unfed_spans),
        })
    }

    /// `trace_of` with its points as JSON and `kind` `connected`, `downstream`, `upstream` or `isolation`.
    pub fn trace_json(
        &self,
        starts: &str,
        barriers: &str,
        reach: f64,
        kind: &str,
    ) -> Result<String, String> {
        let (starts, barriers) = (points_of(starts)?, points_of(barriers)?);
        let kind = match kind {
            "downstream" => TraceKind::Downstream,
            "upstream" => TraceKind::Upstream,
            "isolation" => TraceKind::Isolation,
            _ => TraceKind::Connected,
        };
        Ok(json_of(self.trace_of(&starts, &barriers, reach, kind)))
    }

    /// Denetle: the network's counts, parts and problems.
    pub fn checked(&self) -> Checked {
        check(&self.graph, &self.report)
    }

    /// Denetle: `{nodes, pieces, length, parts, deadEnds, counts, problems: [{kind, x, y, ids, value?, cost?}]}`.
    pub fn check_json(&self) -> String {
        let c = self.checked();
        struct Problem {
            kind: &'static str,
            x: f64,
            y: f64,
            ids: Vec<f64>,
            value: Option<f64>,
            cost: Option<usize>,
        }
        crate::json_struct!(out Problem { kind, x, y, ids, value, cost });
        struct Count {
            kind: &'static str,
            count: usize,
        }
        crate::json_struct!(out Count { kind, count });
        struct Out {
            nodes: usize,
            pieces: usize,
            length: f64,
            parts: Vec<[f64; 2]>,
            dead_ends: usize,
            counts: Vec<Count>,
            problems: Vec<Problem>,
        }
        crate::json_struct!(out Out { nodes, pieces, length, parts, dead_ends => "deadEnds", counts, problems });
        json::to_string(&Out {
            nodes: c.nodes,
            pieces: c.pieces,
            length: c.length,
            parts: c.parts.iter().map(|&(n, l)| [n as f64, l]).collect(),
            dead_ends: c.dead_ends,
            counts: c
                .counts
                .iter()
                .map(|&(k, n)| Count {
                    kind: problem_name(k),
                    count: n,
                })
                .collect(),
            problems: c
                .problems
                .iter()
                .map(|p| Problem {
                    kind: problem_name(p.kind),
                    x: p.at.x,
                    y: p.at.y,
                    ids: p.ids.clone(),
                    value: p.value,
                    cost: p.cost,
                })
                .collect(),
        })
    }
}
