//! Adres's thread (docs/adr/0229 §17): one long-lived thread keeps the
//! project's address data built into locators and answers the tab's and the
//! window's questions in the order they were asked, off the interface's
//! thread. A build brings what the interface's thread read of the drawing
//! (`kentos_native_application::addresses::input`); the index is put
//! together here. An online provider's question waits on a thread of its
//! own (the provider's pace may hold it a second), never on this one.

use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Instant;

use iced::futures::channel::oneshot;
use kentos_geometry_core::ops::geocode::check::{Summary, check};
use kentos_geometry_core::ops::geocode::{Input, Locator, Status, find, parse};
use kentos_interaction::Vec2;
use kentos_processing::address::{Match, local_matches, local_place};

/// What a build said.
#[derive(Clone, Debug, PartialEq)]
pub struct Built {
    pub key: String,
    pub doors: usize,
    pub millis: f64,
}

/// An address asked, as one line.
#[derive(Clone, Debug, PartialEq)]
pub enum Asked {
    Line(String),
}

/// A question's answer.
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    /// An address's candidates, the best first, and the best one's status.
    Found {
        rows: Vec<Match>,
        status: Option<Status>,
    },
    /// A place's address, if any is near (boxed: the app's messages stay small).
    Place(Option<Box<Match>>),
    /// Denetle's summary.
    Checked(Box<Summary>),
    Failed(String),
}

/// An answer for the question `ticket`.
#[derive(Clone, Debug, PartialEq)]
pub struct Reply {
    pub ticket: u64,
    pub answer: Answer,
}

pub enum Job {
    Build {
        key: String,
        input: Box<Input>,
        done: oneshot::Sender<Built>,
    },
    Find {
        key: String,
        asked: Asked,
        min_score: u32,
        ticket: u64,
        done: oneshot::Sender<Reply>,
    },
    Reverse {
        key: String,
        at: Vec2,
        reach: f64,
        ticket: u64,
        done: oneshot::Sender<Reply>,
    },
    Check {
        key: String,
        ticket: u64,
        done: oneshot::Sender<Reply>,
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
            // No thread (the system refused one): the jobs are dropped with their answers' senders, which the
            // tasks waiting for them read as a failure.
            let _ = std::thread::Builder::new()
                .name("KentOS adresleri".into())
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

/// How many candidates the tab lists.
pub const LISTED: usize = 20;

fn run(jobs: mpsc::Receiver<Job>) {
    let mut built: HashMap<String, Locator> = HashMap::new();
    let missing = || Answer::Failed("Adres dizini kurulmadan soruldu.".into());
    while let Ok(job) = jobs.recv() {
        match job {
            Job::Build { key, input, done } => {
                let t0 = Instant::now();
                let loc = Locator::build(&input);
                let doors = loc.counts().doors;
                built.insert(key.clone(), loc);
                let _ = done.send(Built {
                    key,
                    doors,
                    millis: t0.elapsed().as_secs_f64() * 1000.0,
                });
            }
            Job::Find {
                key,
                asked,
                min_score,
                ticket,
                done,
            } => {
                let answer = match built.get(&key) {
                    Some(loc) => {
                        let found = match &asked {
                            Asked::Line(text) => find(loc, text, &parse(text), min_score, LISTED),
                        };
                        Answer::Found {
                            status: (!found.candidates.is_empty()).then_some(found.status),
                            rows: local_matches(&found),
                        }
                    }
                    None => missing(),
                };
                let _ = done.send(Reply { ticket, answer });
            }
            Job::Reverse {
                key,
                at,
                reach,
                ticket,
                done,
            } => {
                let answer = match built.get(&key) {
                    Some(loc) => Answer::Place(
                        loc.reverse(at, reach)
                            .as_ref()
                            .map(|p| Box::new(local_place(p))),
                    ),
                    None => missing(),
                };
                let _ = done.send(Reply { ticket, answer });
            }
            Job::Check { key, ticket, done } => {
                let answer = match built.get(&key) {
                    Some(loc) => Answer::Checked(Box::new(check(loc))),
                    None => missing(),
                };
                let _ = done.send(Reply { ticket, answer });
            }
            Job::Drop(key) => {
                built.remove(&key);
            }
        }
    }
}
