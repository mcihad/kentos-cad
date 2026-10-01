//! Live Python gallery. IO runs on a worker; UI components own no processes.

use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use iced::Task;
use iced::widget::text_editor;
use kentos_ui::theme::motion;
use kentos_ui::widget::python::{
    CompletionEvent, CompletionItem, CompletionRequest, EditorState, ReplEvent, ReplState, Request,
    RunResult, SymbolKind,
};

use crate::message::Message;

pub const OUTPUT: &str = "gallery-python-output";

pub const EXAMPLES: [(&str, &str); 4] = [
    (
        "Geometri",
        "from math import hypot\n\n# Bir parselin çevresini hesaplayalım.\npoints = [(0, 0), (30, 0), (30, 20), (0, 20)]\n\ndef perimeter(points):\n    \"\"\"Kapalı bir çokgenin çevresi.\"\"\"\n    edges = zip(points, points[1:] + points[:1])\n    return sum(hypot(b[0] - a[0], b[1] - a[1])\n               for a, b in edges)\n\nlength = perimeter(points)\nprint(f\"Çevre: {length:.2f} m\")\nprint(f\"Köşe sayısı: {len(points)}\")",
    ),
    (
        "Veri",
        "from dataclasses import dataclass\n\n@dataclass\nclass Parcel:\n    name: str\n    area: float\n\nparcels = [\n    Parcel(\"1244 / 7\", 642.5),\n    Parcel(\"1244 / 8\", 384.0),\n    Parcel(\"1244 / 9\", 518.75),\n]\n\nfor parcel in sorted(parcels, key=lambda p: p.area):\n    print(f\"{parcel.name}: {parcel.area:>8.2f} m²\")\n\nprint(f\"Toplam: {sum(p.area for p in parcels):.2f} m²\")",
    ),
    (
        "Hata",
        "# Hatalar REPL'de izleriyle görünür.\ndef normalize(values):\n    total = sum(values)\n    return [value / total for value in values]\n\nprint(normalize([0, 0, 0]))",
    ),
    (
        "Uzun işlem",
        "import time\n\n# Çalıştırma ışığını ve Durdur düğmesini deneyin.\nfor step in range(8):\n    time.sleep(0.5)\n\nprint(\"İşlem tamamlandı.\")",
    ),
];

#[derive(Debug, Clone)]
pub enum Event {
    Edit(text_editor::Action),
    Complete(CompletionEvent),
    CompletionTick,
    Completions(CompletionTarget, CompletionRequest, Vec<CompletionItem>),
    Undo,
    Redo,
    RunScript,
    Repl(ReplEvent),
    Finished(u64, RunResult),
    Example(usize),
    Copy,
    Restart,
    ReducedMotion(bool),
}

pub struct Studio {
    pub editor: EditorState,
    pub repl: ReplState,
    pub example: usize,
    kernel: Kernel,
    completion_pending: Option<(CompletionTarget, u64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionTarget {
    Editor,
    Repl,
}

impl Default for Studio {
    fn default() -> Self {
        let mut repl = ReplState::default();
        repl.note("Python 3 oturumu ilk çalıştırmada açılır. Değişkenler oturum boyunca korunur.");
        Self {
            editor: EditorState::with_text(EXAMPLES[0].1),
            repl,
            example: 0,
            kernel: Kernel::default(),
            completion_pending: None,
        }
    }
}

impl Studio {
    pub fn wants_completion(&self) -> bool {
        self.completion_pending.is_none()
            && self.repl.status() != kentos_ui::widget::python::RunStatus::Running
            && [&self.editor, &self.repl.input]
                .iter()
                .any(|editor| editor.completion.open && editor.completion.loading)
    }
    pub fn update(&mut self, event: Event) -> Task<Message> {
        let mut script = false;
        let request = match event {
            Event::Complete(event) => {
                self.editor.complete(event);
                None
            }
            Event::CompletionTick => {
                if self.completion_pending.is_none()
                    && self.repl.status() != kentos_ui::widget::python::RunStatus::Running
                {
                    for target in [CompletionTarget::Editor, CompletionTarget::Repl] {
                        let editor = match target {
                            CompletionTarget::Editor => &self.editor,
                            CompletionTarget::Repl => &self.repl.input,
                        };
                        if editor.completion.open
                            && editor.completion.loading
                            && let Some(request) = &editor.completion.request
                        {
                            self.completion_pending = Some((target, request.generation));
                            return self.kernel.complete(target, request.clone());
                        }
                    }
                }
                None
            }
            Event::Completions(target, request, items) => {
                if self.completion_pending == Some((target, request.generation)) {
                    self.completion_pending = None;
                }
                let editor = match target {
                    CompletionTarget::Editor => &mut self.editor,
                    CompletionTarget::Repl => &mut self.repl.input,
                };
                if editor.completion.request.as_ref().is_some_and(|latest| {
                    latest.source == request.source
                        && latest.line == request.line
                        && latest.column == request.column
                }) {
                    editor.completion.receive(request.generation, items);
                }
                None
            }
            Event::Edit(action) => {
                self.editor.perform(action);
                None
            }
            Event::Undo => {
                self.editor.undo();
                None
            }
            Event::Redo => {
                self.editor.redo();
                None
            }
            Event::Example(index) => {
                if let Some((_, source)) = EXAMPLES.get(index) {
                    self.example = index;
                    self.editor = EditorState::with_text(source);
                }
                None
            }
            Event::Copy => return iced::clipboard::write(self.editor.content.text()),
            Event::ReducedMotion(reduced) => {
                motion::set_reduced(reduced);
                None
            }
            Event::RunScript => {
                script = true;
                self.editor.completion.dismiss();
                self.repl.submit(self.editor.content.text())
            }
            Event::Repl(event) => {
                if matches!(event, ReplEvent::Interrupt) {
                    self.kernel.stop();
                }
                self.repl.update(event)
            }
            Event::Finished(id, result) => {
                self.repl.finish(id, result);
                return iced::widget::operation::snap_to_end(OUTPUT);
            }
            Event::Restart => {
                self.editor.completion.dismiss();
                self.repl.input.completion.dismiss();
                self.kernel.stop();
                self.repl.update(ReplEvent::Interrupt);
                self.repl.note(
                    "Python oturumu sıfırlandı; değişkenler ilk çalıştırmada yeniden oluşturulur.",
                );
                None
            }
        };
        if let Some(request) = request {
            Task::batch([
                self.kernel.run(request, script),
                iced::widget::operation::snap_to_end(OUTPUT),
            ])
        } else {
            Task::none()
        }
    }
}

struct Pipes {
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

#[derive(Default)]
struct Kernel {
    child: Option<Arc<Mutex<Child>>>,
    pipes: Option<Arc<Mutex<Pipes>>>,
}

impl Kernel {
    fn start(&mut self) -> io::Result<()> {
        if let Some(child) = &self.child {
            let exited = child
                .lock()
                .map_err(|_| io::Error::other("Python süreç kilidi açılamadı"))?
                .try_wait()?
                .is_some();
            if !exited {
                return Ok(());
            }
            self.stop();
        }
        let mut child = Command::new("python3")
            .args([
                "-u",
                "-c",
                &include_str!("python_kernel.py").replace(
                    "# COMPLETION_PROVIDER",
                    include_str!("python_completion.py"),
                ),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let input = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("Python giriş kanalı açılamadı"))?;
        let output = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("Python çıkış kanalı açılamadı"))?;
        self.child = Some(Arc::new(Mutex::new(child)));
        self.pipes = Some(Arc::new(Mutex::new(Pipes {
            input,
            output: BufReader::new(output),
        })));
        Ok(())
    }

    fn run(&mut self, request: Request, script: bool) -> Task<Message> {
        let id = request.id;
        let pipes = self.start().and_then(|()| {
            self.pipes
                .clone()
                .ok_or_else(|| io::Error::other("Python kanalı yok"))
        });
        let (send, receive) = iced::futures::channel::oneshot::channel();
        std::thread::spawn(move || {
            let result = pipes
                .and_then(|pipes| {
                    let mut pipes = pipes
                        .lock()
                        .map_err(|_| io::Error::other("Python kanal kilidi açılamadı"))?;
                    writeln!(
                        pipes.input,
                        "{} {} {}",
                        id,
                        if script { "script" } else { "repl" },
                        hex(request.source.as_bytes())
                    )?;
                    pipes.input.flush()?;
                    let mut line = String::new();
                    loop {
                        line.clear();
                        if pipes.output.read_line(&mut line)? == 0 {
                            return Err(io::Error::other("Python oturumu kapandı"));
                        }
                        if line.starts_with("R ") {
                            break;
                        }
                    }
                    decode_result(id, &line)
                })
                .unwrap_or_else(|error| {
                    failure(format!(
                        "Python çalıştırılamadı: {error}. python3 kurulumunu denetleyin."
                    ))
                });
            let _ = send.send(result);
        });
        Task::perform(
            async move {
                receive
                    .await
                    .unwrap_or_else(|_| failure("Python işçisi kapandı.".into()))
            },
            move |result| Message::PythonStudio(Event::Finished(id, result)),
        )
    }

    fn stop(&mut self) {
        if let Some(child) = self.child.take()
            && let Ok(mut child) = child.lock()
        {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.pipes = None;
    }

    fn complete(&mut self, target: CompletionTarget, request: CompletionRequest) -> Task<Message> {
        let pipes = self.start().and_then(|()| {
            self.pipes
                .clone()
                .ok_or_else(|| io::Error::other("Python kanalı yok"))
        });
        let (send, receive) = iced::futures::channel::oneshot::channel();
        let generation = request.generation;
        let payload = format!("{},{}\n{}", request.line, request.column, request.source);
        std::thread::spawn(move || {
            let items = pipes
                .and_then(|pipes| {
                    let mut pipes = pipes
                        .lock()
                        .map_err(|_| io::Error::other("Python kanalı kilitli"))?;
                    writeln!(
                        pipes.input,
                        "{generation} complete {}",
                        hex(payload.as_bytes())
                    )?;
                    pipes.input.flush()?;
                    let mut response = String::new();
                    pipes.output.read_line(&mut response)?;
                    decode_completions(generation, &response)
                })
                .unwrap_or_default();
            let _ = send.send(items);
        });
        Task::perform(
            async move { receive.await.unwrap_or_default() },
            move |items| Message::PythonStudio(Event::Completions(target, request.clone(), items)),
        )
    }
}

fn decode_completions(generation: u64, response: &str) -> io::Result<Vec<CompletionItem>> {
    let fields: Vec<_> = response
        .trim_end_matches(['\r', '\n'])
        .splitn(3, ' ')
        .collect();
    if fields.len() != 3 || fields[0] != "C" || fields[1].parse::<u64>().ok() != Some(generation) {
        return Err(io::Error::other("Python tamamlama yanıtı geçersiz"));
    }
    let payload = unhex(fields[2])?;
    payload
        .lines()
        .map(|line| {
            let columns: Vec<_> = line.split('\t').collect();
            if columns.len() != 5 {
                return Err(io::Error::other("Python önerisi geçersiz"));
            }
            let kind = match columns[0] {
                "function" => SymbolKind::Function,
                "method" => SymbolKind::Method,
                "parameter" => SymbolKind::Parameter,
                "class" => SymbolKind::Class,
                "module" => SymbolKind::Module,
                "package" => SymbolKind::Package,
                "property" => SymbolKind::Property,
                "keyword" => SymbolKind::Keyword,
                "constant" => SymbolKind::Constant,
                _ => SymbolKind::Variable,
            };
            Ok(CompletionItem {
                kind,
                name: unhex(columns[1])?,
                insert: unhex(columns[2])?,
                detail: unhex(columns[3])?,
                documentation: unhex(columns[4])?,
            })
        })
        .collect()
}

impl Drop for Kernel {
    fn drop(&mut self) {
        self.stop();
    }
}

fn failure(stderr: String) -> RunResult {
    RunResult {
        stdout: String::new(),
        stderr,
        success: false,
        incomplete: false,
        elapsed: Duration::ZERO,
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn unhex(encoded: &str) -> io::Result<String> {
    if !encoded.len().is_multiple_of(2) || !encoded.is_ascii() {
        return Err(io::Error::other("Python çıktısı çözümlenemedi"));
    }
    let bytes: Result<Vec<_>, _> = (0..encoded.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&encoded[i..i + 2], 16))
        .collect();
    String::from_utf8(bytes.map_err(io::Error::other)?).map_err(io::Error::other)
}

fn decode_result(id: u64, line: &str) -> io::Result<RunResult> {
    let fields: Vec<_> = line.trim_end_matches(['\r', '\n']).splitn(7, ' ').collect();
    if fields.len() != 7 || fields[0] != "R" || fields[1].parse::<u64>().ok() != Some(id) {
        return Err(io::Error::other("Python yanıtı geçersiz"));
    }
    Ok(RunResult {
        success: fields[2] == "1",
        incomplete: fields[3] == "1",
        elapsed: Duration::from_nanos(fields[4].parse().map_err(io::Error::other)?),
        stdout: unhex(fields[5])?,
        stderr: unhex(fields[6])?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "writes live gallery images; KENTOS_SNAPSHOT_BACKEND=wgpu enables GPU capture"]
    fn screens() {
        use crate::{app::Showcase, gallery::Page, message::RibbonTab};
        use iced::widget::text_editor::{Action, Motion};
        use iced::{Point, Size};
        use kentos_ui::{
            snapshot::{Input, Snapshot},
            theme::{Mode, motion},
        };
        motion::set_reduced(true);
        let directory = std::path::Path::new(".run/shots/python");
        std::fs::create_dir_all(directory).unwrap();
        let mut app = Showcase::new();
        app.ribbon_tab = RibbonTab::Gallery;
        app.gallery.page = Page::Python;
        app.python_studio.kernel.start().unwrap();
        let request = app.python_studio.repl.submit(EXAMPLES[0].1.into()).unwrap();
        let result = {
            let mut pipes = app
                .python_studio
                .kernel
                .pipes
                .as_ref()
                .unwrap()
                .lock()
                .unwrap();
            writeln!(
                pipes.input,
                "{} script {}",
                request.id,
                hex(request.source.as_bytes())
            )
            .unwrap();
            pipes.input.flush().unwrap();
            let mut response = String::new();
            pipes.output.read_line(&mut response).unwrap();
            decode_result(request.id, &response).unwrap()
        };
        assert!(result.success);
        assert!(result.stdout.contains("100.00 m"));
        app.python_studio.repl.finish(request.id, result);
        let mut snapshot = Snapshot::new(Size::new(1440.0, 1050.0)).unwrap();
        let mut update = |app: &mut Showcase, event| {
            let _ = app.update(event);
        };
        snapshot.settle(&mut app, Showcase::view, &mut update);
        snapshot.input(
            &mut app,
            Showcase::view,
            &mut update,
            Input::Scroll(Point::new(1300.0, 500.0), -30.0),
        );
        for (mode, name) in [
            (Mode::Dark, "dark"),
            (Mode::Light, "light"),
            (Mode::Night, "night"),
            (Mode::HighContrast, "contrast"),
        ] {
            let _ = app.update(Message::ThemeSelected(mode));
            snapshot
                .render(app.view(), &app.theme())
                .save(directory.join(format!("repl-{name}.png")))
                .unwrap();
        }
        let _ = app.update(Message::ThemeSelected(Mode::Dark));
        for (name, source) in [
            ("import", "from pathlib import Po"),
            ("method", "points.ap"),
            (
                "parameter",
                "def perimeter(points, *, closed=True):\n    return sum(points)\nperimeter(cl",
            ),
        ] {
            app.python_studio.editor = EditorState::with_text(source);
            snapshot.input(
                &mut app,
                Showcase::view,
                &mut update,
                Input::Click(Point::new(160.0, 325.0)),
            );
            app.python_studio
                .editor
                .content
                .perform(Action::Move(Motion::DocumentEnd));
            app.python_studio.editor.complete(CompletionEvent::Request);
            let request = app.python_studio.editor.completion.request.clone().unwrap();
            let items = {
                let mut pipes = app
                    .python_studio
                    .kernel
                    .pipes
                    .as_ref()
                    .unwrap()
                    .lock()
                    .unwrap();
                let payload = format!("{},{}\n{}", request.line, request.column, request.source);
                writeln!(pipes.input, "99 complete {}", hex(payload.as_bytes())).unwrap();
                pipes.input.flush().unwrap();
                let mut response = String::new();
                pipes.output.read_line(&mut response).unwrap();
                decode_completions(99, &response).unwrap()
            };
            assert!(!items.is_empty(), "{source}");
            app.python_studio
                .editor
                .completion
                .receive(request.generation, items);
            snapshot.settle(&mut app, Showcase::view, &mut update);
            snapshot
                .render(app.view(), &app.theme())
                .save(directory.join(format!("completion-{name}.png")))
                .unwrap();
        }
        println!("{}: {}", directory.display(), snapshot.renderer_name());
    }

    #[test]
    fn real_completion_resolves_imports_types_parameters_and_live_members_without_running_properties()
     {
        use iced::widget::text_editor::{Action, Motion};
        let mut kernel = Kernel::default();
        kernel.start().unwrap();
        let mut pipes = kernel.pipes.as_ref().unwrap().lock().unwrap();
        let mut query = |source: &str| {
            let row = source.lines().count() - 1;
            let column = source.lines().last().unwrap().len();
            let payload = format!("{row},{column}\n{source}");
            writeln!(pipes.input, "1 complete {}", hex(payload.as_bytes())).unwrap();
            pipes.input.flush().unwrap();
            let mut response = String::new();
            pipes.output.read_line(&mut response).unwrap();
            decode_completions(1, &response).unwrap()
        };
        for (source, name, kind) in [
            ("import ma", "math", SymbolKind::Module),
            ("from pathlib import Po", "PosixPath", SymbolKind::Class),
            ("import xml.", "dom", SymbolKind::Package),
            ("from os import pa", "path", SymbolKind::Module),
            ("import math as geo\ngeo.hy", "hypot", SymbolKind::Function),
            ("name = 'çizim'\nname.up", "upper", SymbolKind::Method),
            (
                "def polygon_area(points, *, signed=False):\n    return 42\npolygon_area(si",
                "signed",
                SymbolKind::Parameter,
            ),
            (
                "class Parcel:\n    def measure(self, factor=1):\n        return factor\nparcel = Parcel()\nparcel.me",
                "measure",
                SymbolKind::Method,
            ),
            (
                "def f(x: str):\n    return x.up",
                "upper",
                SymbolKind::Method,
            ),
            (
                "class Parcel:\n    def __init__(self):\n        self.area = 42\nparcel = Parcel()\nparcel.ar",
                "area",
                SymbolKind::Property,
            ),
        ] {
            let items = query(source);
            let item = items
                .iter()
                .find(|item| item.name == name)
                .unwrap_or_else(|| panic!("{source}: {items:?}"));
            assert_eq!(item.kind, kind, "{source}");
            if kind == SymbolKind::Parameter {
                assert_eq!(item.insert, "signed=");
            }
        }
        let program = "calls = 0\nclass RuntimeParcel:\n    @property\n    def expensive(self):\n        global calls\n        calls += 1\n        return 42\nruntime_obj = RuntimeParcel()\nruntime_values = [1, 2]\n";
        writeln!(pipes.input, "2 script {}", hex(program.as_bytes())).unwrap();
        pipes.input.flush().unwrap();
        let mut response = String::new();
        pipes.output.read_line(&mut response).unwrap();
        assert!(decode_result(2, &response).unwrap().success);
        let source = "runtime_values.ap";
        let payload = format!("0,{}\n{source}", source.len());
        writeln!(pipes.input, "3 complete {}", hex(payload.as_bytes())).unwrap();
        pipes.input.flush().unwrap();
        response.clear();
        pipes.output.read_line(&mut response).unwrap();
        let items = decode_completions(3, &response).unwrap();
        assert_eq!(items[0].name, "append");
        assert_eq!(items[0].kind, SymbolKind::Method);
        let mut editor = EditorState::with_text(source);
        editor.content.perform(Action::Move(Motion::DocumentEnd));
        editor.complete(CompletionEvent::Request);
        editor.completion.receive(
            editor.completion.request.as_ref().unwrap().generation,
            items,
        );
        editor.complete(CompletionEvent::Accept);
        assert_eq!(editor.content.text(), "runtime_values.append");
        let program = format!("{}(3)\nruntime_values", editor.content.text());
        writeln!(pipes.input, "4 repl {}", hex(program.as_bytes())).unwrap();
        pipes.input.flush().unwrap();
        response.clear();
        pipes.output.read_line(&mut response).unwrap();
        assert_eq!(decode_result(4, &response).unwrap().stdout, "[1, 2, 3]\n");
        writeln!(pipes.input, "5 complete {}", hex(b"0,14\nruntime_obj.ex")).unwrap();
        pipes.input.flush().unwrap();
        response.clear();
        pipes.output.read_line(&mut response).unwrap();
        assert_eq!(
            decode_completions(5, &response).unwrap()[0].kind,
            SymbolKind::Property
        );
        writeln!(pipes.input, "6 repl {}", hex(b"calls")).unwrap();
        pipes.input.flush().unwrap();
        response.clear();
        pipes.output.read_line(&mut response).unwrap();
        assert_eq!(decode_result(6, &response).unwrap().stdout, "0\n");
    }

    #[test]
    fn real_python_preserves_variables_displays_expressions_and_reports_errors() {
        let mut kernel = Kernel::default();
        kernel
            .start()
            .expect("python3 is installed for the live gallery test");
        let pipes = kernel.pipes.as_ref().unwrap();
        let mut pipes = pipes.lock().unwrap();
        let mut execute = |id, source: &str| {
            writeln!(pipes.input, "{id} repl {}", hex(source.as_bytes())).unwrap();
            pipes.input.flush().unwrap();
            let mut line = String::new();
            pipes.output.read_line(&mut line).unwrap();
            decode_result(id, &line).unwrap()
        };
        assert!(execute(1, "ölçü = 21").success);
        assert_eq!(execute(2, "ölçü * 2").stdout, "42\n");
        assert_eq!(execute(3, "_ + 1").stdout, "43\n");
        assert!(execute(4, "for x in range(3):").incomplete);
        let error = execute(5, "1 / 0");
        assert!(!error.success);
        assert!(error.stderr.contains("ZeroDivisionError"));
        assert_eq!(execute(6, "print('çizim 🦀')").stdout, "çizim 🦀\n");
        assert!(execute(7, "print('x' * 500000)").stdout.len() < 132000);
        assert!(!execute(8, "input('değer')").success);
        assert_eq!(execute(9, "42").stdout, "42\n");
    }

    #[test]
    fn stopping_python_terminates_an_infinite_script_and_opens_a_fresh_session() {
        let mut kernel = Kernel::default();
        kernel.start().unwrap();
        {
            let mut pipes = kernel.pipes.as_ref().unwrap().lock().unwrap();
            writeln!(pipes.input, "1 script {}", hex(b"while True:\n    pass\n")).unwrap();
            pipes.input.flush().unwrap();
        }
        let started = std::time::Instant::now();
        kernel.stop();
        assert!(started.elapsed() < Duration::from_secs(2));
        kernel.start().unwrap();
        let mut pipes = kernel.pipes.as_ref().unwrap().lock().unwrap();
        writeln!(pipes.input, "2 repl {}", hex(b"6 * 7")).unwrap();
        pipes.input.flush().unwrap();
        let mut response = String::new();
        pipes.output.read_line(&mut response).unwrap();
        assert_eq!(decode_result(2, &response).unwrap().stdout, "42\n");
    }
}
