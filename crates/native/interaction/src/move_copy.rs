//! Taşı and Kopyala: the web's `MoveTool` (`apps/web/src/tools/modifyTools.ts`)
//! on its `SelectionFirstTool` base ([`crate::modify`]), step for step
//! (docs/adr/0037), with AutoCAD's MOVE and COPY rules (docs/adr/0291):
//!
//! - the objects (the selection, or picked first), a base point, then the
//!   target: clicked, or typed (`@dY,dX` from the base, a distance along the
//!   cursor);
//! - Yer değiştirme (Y), or Enter at the base point: the displacement typed
//!   at once (`dY,dX` or `@dY,dX`, a polar `@d<a` in the project's way), Enter
//!   the last one used; Enter with none the tool leaves;
//! - Taşı moves the selection there and leaves; Kopyala leaves a copy at
//!   every point until a confirm (Bitir, Enter), or one only with Kip (P)
//!   Tek, which stays for the next copies;
//! - Kopyala's Dizi (A): a count (the originals among them), then the second
//!   point: the copies a displacement apart along it, or with Sığdır (S)
//!   sharing it, the last on the point (`cad.entities.array`'s line).
//!
//! Written through `cad.entities.transform` (a displacement), one undo step
//! per move or copy, “Taşı” or “Kopyala”; says “2 nesne taşındı: ΔY 12.500
//! ΔX -7.250”. The ghosts and the length beside the cursor are the web's.

use kentos_contracts::{ArrayLayout, Transform};
use kentos_geometry_core::geom::affine::{Affine, translation};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::editing::line_array_transforms;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::modify::{Modify, Stages, array_selection, transform_selection};
use crate::points::plain_number;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow};

/// The move tool's id: its command is `tool.move`.
pub const MOVE_ID: &str = "move";
pub const MOVE_LABEL: &str = "Taşı";
/// The copy tool's id: its command is `tool.copy`.
pub const COPY_ID: &str = "copy";
pub const COPY_LABEL: &str = "Kopyala";

/// Kopyala's Dizi: its count once typed, and whether the copies share the displacement.
#[derive(Clone, Copy, Debug, Default)]
struct Array {
    count: Option<u32>,
    fit: bool,
}

/// The move tool's stages; with `copy`, the copy tool's.
#[derive(Clone, Debug, Default)]
pub struct Move {
    copy: bool,
    base: Option<Vec2>,
    /// Yer değiştirme: the displacement is typed, no base point given.
    displacement: bool,
    array: Option<Array>,
    /// Kip: one copy, then the tool leaves (the session remembers it).
    single: bool,
    /// The last displacement used (the session's), offered at Enter.
    last: Option<[f64; 2]>,
    /// The project's units, as of the last event.
    format: Option<Format>,
}

impl Move {
    /// Taşı.
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    /// Kopyala.
    pub fn copy_tool() -> Modify<Self> {
        Modify::with(Self {
            copy: true,
            ..Self::default()
        })
    }

    /// Moves or copies the selection by (`dx`, `dy`); with Dizi, the line of copies. Leaves after a move, a
    /// single copy or a displacement typed; a copy goes on.
    fn apply(&mut self, dx: f64, dy: f64, cx: &mut Context<'_>) -> Flow {
        let f = cx.format();
        if let Some(Array {
            count: Some(count),
            fit,
        }) = self.array
        {
            let layout = ArrayLayout::Line { count, dx, dy, fit };
            if let Some(n) = array_selection(layout, cx) {
                cx.memory.last_displacement = Some([dx, dy]);
                let line = format!(
                    "{n} kopya diziye yerleştirildi: {count} yer, ΔY {}  ΔX {}{}",
                    f.length_bare(dx),
                    f.length_bare(dy),
                    if fit { " (sığdırıldı)" } else { "" }
                );
                cx.say(Level::Success, line);
            }
            self.array = None;
            return if self.single || self.displacement {
                Flow::Exit
            } else {
                Flow::Stay
            };
        }
        if let Some(n) = transform_selection(Transform::Move { dx, dy }, self.copy, cx) {
            cx.memory.last_displacement = Some([dx, dy]);
            let done = if self.copy {
                "kopyalandı"
            } else {
                "taşındı"
            };
            let line = format!(
                "{n} nesne {done}: ΔY {}  ΔX {}",
                f.length_bare(dx),
                f.length_bare(dy)
            );
            cx.say(Level::Success, line);
        }
        if self.copy && !self.single && !self.displacement {
            Flow::Stay
        } else {
            Flow::Exit
        }
    }

    /// A displacement typed as `dY,dX`, `@dY,dX` or `@d<a`: the vector from the origin.
    fn typed_displacement(text: &str, cx: &mut Context<'_>) -> Option<[f64; 2]> {
        let t = js_trim(text);
        let t = t.strip_prefix('@').unwrap_or(t);
        let origin = Vec2::new(0.0, 0.0);
        let p = cx.typed_point(&format!("@{t}"), Some(origin), None)?;
        Some([p.x, p.y])
    }
}

impl Stages for Move {
    fn id(&self) -> &'static str {
        if self.copy { COPY_ID } else { MOVE_ID }
    }

    fn label(&self) -> &'static str {
        if self.copy { COPY_LABEL } else { MOVE_LABEL }
    }

    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.base = None;
        self.displacement = false;
        self.array = None;
        self.see(cx);
        Flow::Stay
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.single = cx.memory.copy_single;
        self.last = cx.memory.last_displacement;
        self.format = Some(cx.format());
    }

    fn anchor(&self) -> Option<Vec2> {
        self.base
    }

    fn prompt(&self, n: usize) -> Prompt {
        let label = self.label();
        if self.displacement {
            let last = match (self.last, self.format) {
                (Some([x, y]), Some(f)) => format!(" (Enter: {},{})", f.length_bare(x), f.length_bare(y)),
                _ => String::new(),
            };
            return Prompt::new(label, format!("yer değiştirmeyi yazın: dY,dX{last}"));
        }
        if let Some(a) = self.array {
            return match a.count {
                None => Prompt::new(label, "dizideki öğe sayısını yazın (asıllar dahil)"),
                Some(c) => Prompt::new(label, format!("dizinin ikinci noktasını belirtin ({c} yer)"))
                    .option_with("Sığdır", "S", if a.fit { "açık" } else { "kapalı" }),
            };
        }
        match self.base {
            None => {
                let prompt = Prompt::new(label, format!("{n} nesne için temel noktayı belirtin"))
                    .option("Yer değiştirme", "Y");
                if self.copy {
                    prompt.option_with("Kip", "P", if self.single { "tek" } else { "çoklu" })
                } else {
                    prompt
                }
            }
            Some(_) if self.copy => {
                Prompt::new(label, "kopyanın yerini belirtin ya da @dY,dX yazın")
                    .option("Dizi", "A")
                    .option("Bitir", "Enter")
            }
            Some(_) => Prompt::new(label, "hedef noktayı belirtin ya da @dY,dX yazın"),
        }
    }

    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) -> Flow {
        if self.displacement {
            return Flow::Stay;
        }
        let Some(base) = self.base else {
            self.base = Some(p);
            return Flow::Stay;
        };
        if self.array.is_some_and(|a| a.count.is_none()) {
            return Flow::Stay;
        }
        if dist(base, p) < 1e-12 && self.array.is_some() {
            return Flow::Stay;
        }
        self.apply(p.x - base.x, p.y - base.y, cx)
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        let key = upper_tr(js_trim(text));
        if self.displacement {
            let [dx, dy] = Self::typed_displacement(text, cx)?;
            return Some(self.apply(dx, dy, cx));
        }
        if let Some(a) = &mut self.array {
            if a.count.is_none() {
                let n = plain_number(text)?;
                if !(n.fract() == 0.0 && (2.0..=10_000.0).contains(&n)) {
                    cx.say(
                        Level::Warn,
                        "Adet 2 ile 10 000 arasında bir tam sayı olmalı; asıllar da sayılır.",
                    );
                    return Some(Flow::Stay);
                }
                a.count = Some(n as u32);
                return Some(Flow::Stay);
            }
            if key == "S" {
                a.fit = !a.fit;
                return Some(Flow::Stay);
            }
            return None;
        }
        match (self.base, key.as_str()) {
            (None, "Y") => {
                self.displacement = true;
                Some(Flow::Stay)
            }
            (None, "P") if self.copy => {
                self.single = !self.single;
                cx.memory.copy_single = self.single;
                Some(Flow::Stay)
            }
            (Some(_), "A") if self.copy => {
                self.array = Some(Array::default());
                Some(Flow::Stay)
            }
            _ => None,
        }
    }

    /// Enter: at the base point, Yer değiştirme (AutoCAD's default); in it, the last displacement, or with none
    /// the tool leaves; in Dizi, back to the copies; after a base point, the tool leaves (Bitir).
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.array.is_some() {
            self.array = None;
            return Flow::Stay;
        }
        if self.displacement {
            return match self.last {
                Some([dx, dy]) => self.apply(dx, dy, cx),
                None => Flow::Exit,
            };
        }
        if self.base.is_none() {
            self.displacement = true;
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, hover: Vec2) -> Option<Affine> {
        self.base
            .map(|base| translation(hover.x - base.x, hover.y - base.y))
    }

    fn previews(&self, hover: Option<Vec2>) -> Vec<Affine> {
        let (Some(base), Some(hover)) = (self.base, hover) else {
            return Vec::new();
        };
        let (dx, dy) = (hover.x - base.x, hover.y - base.y);
        match self.array {
            Some(Array {
                count: Some(count),
                fit,
            }) => line_array_transforms(f64::from(count), dx, dy, fit),
            Some(_) => Vec::new(),
            None => vec![translation(dx, dy)],
        }
    }

    fn tag(&self, hover: Vec2, format: &Format) -> Vec<String> {
        self.base
            .map(|base| vec![format.length(dist(base, hover))])
            .unwrap_or_default()
    }
}
