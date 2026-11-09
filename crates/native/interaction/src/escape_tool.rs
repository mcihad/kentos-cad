//! Kaçış yolları (docs/adr/3035 §2; M35): the active storey's escape walks drawn as Mimari denetim's fire rules measure
//! them (Binaların Yangından Korunması Hakkında Yönetmelik): from each room's farthest point, 40 cm in from its walls
//! (32/6), to its nearest storey exit; a flat's from its farthest point to its door (48/2) and from its door to the
//! exit (48/4). Each walk says what it measured against its limit; red when it breaks it, green when it keeps it, grey
//! where the regulation does not ask it (48/1). It draws and leaves, one undo step; run again it takes the old walks
//! away first (`fire_marks`).

use kentos_native_application::fire_marks;

use crate::format::Format;
use crate::log::Level;
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer, Preview, Tool};

pub const ID: &str = "escapeRoutes";
pub const LABEL: &str = "Kaçış yolları";

/// The escape walks' drawing: it draws on starting and leaves.
#[derive(Clone, Debug, Default)]
pub struct EscapeRoutes;

impl EscapeRoutes {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for EscapeRoutes {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(LABEL, "etkin katın kaçış yolları çiziliyor")
    }

    fn point_count(&self) -> usize {
        0
    }

    /// The active storey's walks drawn, the old ones taken away; what was drawn said.
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        let level = cx.level.map(str::to_owned);
        let had = cx
            .doc
            .entities()
            .any(|e| e.base().attrs.contains_key(fire_marks::MARK_ATTR));
        let group = cx.doc.begin_group(LABEL);
        match fire_marks::draw(cx.doc, level.as_deref()) {
            Ok(d) if d.walks == 0 => {
                if had {
                    cx.doc.end_group(group);
                } else {
                    cx.doc.cancel_group(group);
                }
                cx.say(
                    Level::Info,
                    "Bu katta ölçülecek kaçış yolu yok: kapılarıyla mahaller ve bir kat çıkışı gerekir (adı Merdiven \
                     olan ortak mahal, açık merdiven ya da zemin katta dışarı açılan kapı).",
                );
            }
            Ok(d) => {
                cx.doc.end_group(group);
                let level_name = level
                    .as_deref()
                    .and_then(|id| {
                        cx.doc
                            .settings()
                            .arch
                            .as_ref()
                            .and_then(|a| a.levels.iter().find(|l| l.id == id))
                            .map(|l| format!(" (“{}”)", l.name))
                    })
                    .unwrap_or_default();
                if d.over > 0 {
                    cx.say(
                        Level::Warn,
                        format!(
                            "{} kaçış yolu çizildi{level_name}; {} tanesi sınırı aşıyor (kırmızı). Ayrıntı Denetim › \
                             Mevzuat'ta.",
                            d.walks, d.over
                        ),
                    );
                } else {
                    cx.say(
                        Level::Success,
                        format!(
                            "{} kaçış yolu çizildi{level_name}; hepsi sınırın içinde.",
                            d.walks
                        ),
                    );
                }
            }
            Err(why) => {
                cx.doc.cancel_group(group);
                cx.say(Level::Warn, why);
            }
        }
        Flow::Exit
    }

    fn snaps(&self) -> bool {
        false
    }

    fn pointer_move(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {}

    fn pointer_down(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {}

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview::default()
    }
}
