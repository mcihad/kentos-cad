//! Places ceiling devices on the current storey's ceiling and its existing finish grid.

use kentos_commands::{
    BlockDefinition, BlockId, BlockUnits, BlocksInsert, CEILING_DEVICE_ATTR, CEILING_SPACE_ATTR,
    CommandResult, Entity, EntityBase, PathEntity,
};
use kentos_native_application::{ExecutionContext, blocks_insert, ceiling_plan};
use std::collections::BTreeMap;

use crate::tool::{Context, Flow, OptionChoice, Pointer, Preview, Stroke, Tag, Tool};
use crate::tool_settings::Setting;
use crate::{Vec2, arch_layer, format::Format, log::Level, points, prompt::Prompt};

pub const ID: &str = "ceilingDevice";
const LABEL: &str = "Tavan cihazı";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Device {
    #[default]
    Light,
    Diffuser,
    Access,
}

impl Device {
    const ALL: [(Self, &str, &str); 3] = [
        (Self::Light, "light", "Armatür"),
        (Self::Diffuser, "diffuser", "Menfez"),
        (Self::Access, "access", "Bakım kapağı"),
    ];

    fn label(self) -> &'static str {
        Self::ALL[self as usize].2
    }
    fn id(self) -> BlockId {
        BlockId([
            1,
            147,
            165,
            224,
            38,
            19,
            112,
            0,
            128,
            0,
            116,
            97,
            118,
            97,
            110,
            1 + self as u8,
        ])
    }

    fn outline(self) -> Vec<(Vec<Vec2>, bool)> {
        let point = Vec2::new;
        let border = if self == Self::Light {
            (0..32)
                .map(|i| {
                    let a = f64::from(i) * std::f64::consts::TAU / 32.0;
                    point(0.15 * kentos_num::cos(a), 0.15 * kentos_num::sin(a))
                })
                .collect()
        } else {
            let half = if self == Self::Diffuser { 0.15 } else { 0.30 };
            vec![
                point(-half, -half),
                point(half, -half),
                point(half, half),
                point(-half, half),
            ]
        };
        let mut out = vec![(border, true)];
        match self {
            Self::Light => {
                out.push((vec![point(-0.09, 0.0), point(0.09, 0.0)], false));
                out.push((vec![point(0.0, -0.09), point(0.0, 0.09)], false));
            }
            Self::Diffuser => {
                for y in [-0.075, 0.0, 0.075] {
                    out.push((vec![point(-0.11, y), point(0.11, y)], false));
                }
            }
            Self::Access => out.push((vec![point(-0.30, -0.30), point(0.30, 0.30)], false)),
        }
        out
    }

    fn definition(self) -> BlockDefinition {
        let entities = self
            .outline()
            .into_iter()
            .enumerate()
            .map(|(i, (pts, closed))| {
                Entity::Polyline(PathEntity {
                    base: EntityBase {
                        id: i as u32 + 1,
                        layer_id: String::new(),
                        attrs: BTreeMap::new(),
                        color: None,
                        label: None,
                        symbol: None,
                        line_weight: None,
                        label_pins: Vec::new(),
                        attachments: Vec::new(),
                        look: Default::default(),
                    },
                    pts: pts.into_iter().map(points::wire).collect(),
                    closed,
                    space_separator: None,
                    bulges: None,
                    holes: None,
                    zs: None,
                    ms: None,
                    parts: None,
                    widths: None,
                })
            })
            .collect();
        BlockDefinition {
            id: self.id(),
            name: format!("Tavan · {}", self.label()),
            base: points::wire(Vec2::new(0.0, 0.0)),
            entities,
            attributes: Vec::new(),
            description: None,
            units: Some(BlockUnits::M),
            card: None,
            dynamic: None,
            xref: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CeilingDeviceTool {
    device: Device,
    align: bool,
    hover: Option<Vec2>,
    preview: Preview,
}

impl Default for CeilingDeviceTool {
    fn default() -> Self {
        Self {
            device: Device::Light,
            align: true,
            hover: None,
            preview: Preview::default(),
        }
    }
}

impl CeilingDeviceTool {
    fn target(&self, p: Vec2, cx: &Context<'_>) -> Option<(String, Vec2, f64, bool)> {
        let level = cx
            .spatial
            .ceiling_view()
            .and_then(|(level, _)| level.as_deref())
            .or(cx.level);
        let (room, _) = ceiling_plan::room_at(cx.doc, level, p)?;
        let uid = cx.doc.uid(room)?.to_string();
        let found = self
            .align
            .then(|| ceiling_plan::align(cx.doc, room, p))
            .flatten();
        let (at, turn) = found.unwrap_or((p, 0.0));
        Some((uid, at, turn, found.is_some()))
    }

    fn redraw(&mut self, cx: &Context<'_>) {
        self.preview = Preview::default();
        let Some((_, at, turn, aligned)) = self.hover.and_then(|p| self.target(p, cx)) else {
            return;
        };
        let (sin, cos) = kentos_num::sin_cos(turn.to_radians());
        for (line, closed) in self.device.outline() {
            let pts = line
                .into_iter()
                .map(|p| Vec2::new(at.x + cos * p.x - sin * p.y, at.y + sin * p.x + cos * p.y))
                .collect();
            self.preview.strokes.push(Stroke::solid(pts, closed));
        }
        self.preview.tag = Some(Tag {
            at,
            lines: vec![
                self.device.label().into(),
                if aligned {
                    "Tavan ızgarası · hücre ortası"
                } else {
                    "Serbest konum"
                }
                .into(),
            ],
        });
    }

    fn place(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let Some((room, at, turn, _)) = self.target(p, cx) else {
            cx.say(
                Level::Warn,
                "Tavan cihazını etkin kattaki bir mahalin içine yerleştirin.",
            );
            return;
        };
        let device = self.device;
        let kind = arch_layer::Kind::CeilingDevice;
        arch_layer::writing(&kind, cx, |layer, cx| {
            let group = cx.doc.begin_group(LABEL);
            if cx.doc.block(device.id()).is_none() {
                let added = cx.doc.add_block(device.definition());
                if let Err(problem) = added {
                    cx.doc.cancel_group(group);
                    cx.say(Level::Warn, problem.to_string());
                    return false;
                }
            }
            let attrs = BTreeMap::from([
                (CEILING_DEVICE_ATTR.into(), device.label().into()),
                (CEILING_SPACE_ATTR.into(), room),
            ]);
            let mut arch = cx.new_arch().unwrap_or_default();
            if let Some((Some(level), _)) = cx.spatial.ceiling_view() {
                arch.level = Some(level.clone());
            }
            let result = blocks_insert::execute(
                &mut ExecutionContext::new(cx.doc),
                BlocksInsert {
                    block: Some(device.id()),
                    name: None,
                    at: points::wire(at),
                    layer_id: layer.into(),
                    scale: Some(1.0),
                    y_scale: None,
                    rotation: Some(turn),
                    mirror: None,
                    color: None,
                    attributes: None,
                    attrs: Some(attrs),
                    label: None,
                    symbol: None,
                    arch,
                    parameters: None,
                    expected_revision: None,
                },
            );
            if matches!(result, CommandResult::Completed { .. }) {
                cx.doc.end_group(group);
                cx.say(
                    Level::Success,
                    format!("{} tavan planına yerleştirildi.", device.label()),
                );
                true
            } else {
                cx.doc.cancel_group(group);
                let _ = points::written(result, cx);
                false
            }
        });
    }
}

impl Tool for CeilingDeviceTool {
    fn id(&self) -> &'static str {
        ID
    }
    fn label(&self) -> &'static str {
        LABEL
    }
    fn point_count(&self) -> usize {
        0
    }
    fn snaps(&self) -> bool {
        !self.align
    }
    fn prompt(&self) -> Prompt {
        Prompt::new(LABEL, "bir mahalde cihazın yerini belirtin")
    }
    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.redraw(cx);
    }
    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.place(p.world, cx);
    }
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let Some(p) = cx.typed_point(text, None, self.hover) else {
            return false;
        };
        self.place(p, cx);
        true
    }
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }
    fn preview(&self, _fmt: &Format) -> Preview {
        self.preview.clone()
    }

    fn settings(&self, _fmt: &Format) -> Vec<Setting> {
        vec![
            Setting::choice(
                "device",
                "Cihaz",
                Device::ALL
                    .iter()
                    .map(|(kind, key, label)| OptionChoice {
                        label: (*label).into(),
                        typed: (*key).into(),
                        icon: None,
                        preview: None,
                        checked: *kind == self.device,
                        command: None,
                    })
                    .collect(),
            ),
            Setting::toggle("grid", "Izgaraya hizala", self.align),
        ]
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        match key {
            "grid" => self.align = value == "yes",
            "device" => {
                let Some((kind, _, _)) = Device::ALL.iter().find(|(_, k, _)| *k == value) else {
                    return false;
                };
                self.device = *kind;
            }
            _ => return false,
        }
        self.redraw(cx);
        true
    }
}
