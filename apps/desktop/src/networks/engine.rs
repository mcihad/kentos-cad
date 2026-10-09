//! The network thread (docs/adr/0209 §12; the web's network worker,
//! `apps/web/src/io/network/`): one long-lived thread keeps the project's
//! networks built and answers their questions in the order they were asked,
//! off the interface's thread. A build brings what the interface's thread
//! read of the drawing (the edges' paths and values, the junctions); the
//! graph is put together here. The way to the cursor is answered as points,
//! its arcs as short segments.

use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Instant;

use iced::futures::channel::oneshot;
use kentos_geometry_core::geom::bulge::bulge_path_outline;
use kentos_geometry_core::ops::network::{EdgeIn, Graph, JunctionIn, NetworkSession, Rules};
use kentos_interaction::Vec2;
use kentos_interaction::network::{Answer, NetworkQuestion, NetworkReply};

/// What a build said, for the interface's thread to say once it is done.
#[derive(Clone, Debug, PartialEq)]
pub struct Built {
    pub network: String,
    pub name: String,
    pub problems: Vec<String>,
    pub skipped: usize,
    pub nodes: usize,
    pub pieces: usize,
    pub millis: f64,
}

pub enum Job {
    Build {
        network: String,
        name: String,
        rules: Rules,
        edges: Vec<EdgeIn>,
        junctions: Vec<JunctionIn>,
        skipped: Vec<(f64, &'static str)>,
        problems: Vec<String>,
        done: oneshot::Sender<Built>,
    },
    Ask {
        network: String,
        ticket: u64,
        question: NetworkQuestion,
        done: oneshot::Sender<NetworkReply>,
    },
    Drop(String),
}

/// The thread, started on first use.
#[derive(Default)]
pub struct Engine {
    jobs: Option<mpsc::Sender<Job>>,
}

impl Engine {
    pub fn send(&mut self, job: Job) {
        let jobs = self.jobs.get_or_insert_with(|| {
            let (tx, rx) = mpsc::channel();
            // No thread (the system refused one): the jobs are dropped and the answers' senders with them, which
            // the tasks waiting for them read as the network's failure.
            let _ = std::thread::Builder::new()
                .name("KentOS ağları".into())
                .spawn(move || run(rx));
            tx
        });
        if let Err(mpsc::SendError(job)) = jobs.send(job) {
            // The thread ended (it never does but by a fault): the next job starts another.
            self.jobs = None;
            self.send(job);
        }
    }
}

fn run(jobs: mpsc::Receiver<Job>) {
    let mut nets: HashMap<String, NetworkSession> = HashMap::new();
    while let Ok(job) = jobs.recv() {
        match job {
            Job::Build {
                network,
                name,
                rules,
                edges,
                junctions,
                skipped,
                problems,
                done,
            } => {
                let t0 = Instant::now();
                let (graph, report) = Graph::build(rules, &edges, &junctions);
                let session = NetworkSession::new(graph, report, skipped);
                let built = Built {
                    network: network.clone(),
                    name,
                    problems,
                    skipped: session.skipped.len(),
                    nodes: session.graph.nodes.len(),
                    pieces: session.graph.pieces.len(),
                    millis: t0.elapsed().as_secs_f64() * 1000.0,
                };
                nets.insert(network, session);
                let _ = done.send(built);
            }
            Job::Ask {
                network,
                ticket,
                question,
                done,
            } => {
                let answer = match nets.get_mut(&network) {
                    Some(session) => answer(session, question),
                    None => Answer::Failed("Ağ kurulmadan soruldu.".into()),
                };
                let _ = done.send(NetworkReply { ticket, answer });
            }
            Job::Drop(network) => {
                nets.remove(&network);
            }
        }
    }
}

/// A question answered by a built network.
pub fn answer(s: &mut NetworkSession, q: NetworkQuestion) -> Answer {
    match q {
        NetworkQuestion::Prepare => Answer::Ready,
        NetworkQuestion::Locate { at, reach } => Answer::Place(s.locate(at, reach)),
        NetworkQuestion::Route {
            stops,
            barriers,
            reach,
            cost,
            reorder,
        } => Answer::Route(s.route(&stops, &barriers, reach, cost, reorder)),
        NetworkQuestion::TreeFrom {
            at,
            reach,
            cost,
            barriers,
        } => Answer::Tree(s.tree_at(at, reach, cost, &barriers)),
        NetworkQuestion::PathTo { at, reach } => {
            let mut nums = Vec::new();
            s.path_to(at.x, at.y, reach, &mut nums);
            Answer::Path(way(&nums))
        }
        NetworkQuestion::Area {
            facilities,
            breaks,
            reach,
            cost,
            toward,
            separate,
            barriers,
            trim,
            rings,
            areas,
        } => Answer::Area(s.service_area(
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
        )),
        NetworkQuestion::Trace {
            starts,
            barriers,
            reach,
            kind,
        } => Answer::Trace(s.trace_of(&starts, &barriers, reach, kind)),
        NetworkQuestion::Check => Answer::Check(Box::new(s.checked())),
    }
}

/// The way to the cursor from the core's numbers (`[cost, n, x0, y0, …, bulge0, …]`): its cost and its points, arcs
/// as short segments.
fn way(nums: &[f64]) -> Option<(f64, Vec<Vec2>)> {
    let (&cost, rest) = nums.split_first()?;
    let n = *rest.first()? as usize;
    let xy = rest.get(1..1 + 2 * n)?;
    let pts: Vec<Vec2> = xy.chunks_exact(2).map(|p| Vec2::new(p[0], p[1])).collect();
    let bulges = &rest[1 + 2 * n..];
    let out = if bulges.iter().any(|b| *b != 0.0) {
        bulge_path_outline(&pts, Some(bulges), false, std::f64::consts::PI / 36.0)
    } else {
        pts
    };
    Some((cost, out))
}
