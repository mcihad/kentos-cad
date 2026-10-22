//! Ephemeral paper/model mode. The host's existing model tools own all model edits.
use crate::{Designer, Effect, Message};
use crate::message::StageEvent;
use kentos_sheet::model_scope::{Viewport, NavigateViewport};
use kentos_sheet::ops::Op;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointerAction {
    Move,
    Press,
    Release,
    Leave,
    RightClick,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ModelPointer {
    pub action: PointerAction,
    pub world: [f64; 2],
    pub pixels_per_metre: f64,
    pub rotation: i32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelCamera {
    pub center: [f64; 2],
    pub size: [f64; 2],
    pub resolution: f64,
    pub rotation: i32,
}
impl Designer {
    pub fn model_camera(&self) -> Option<ModelCamera> {
        let v = self.active_viewport()?;
        let paper = self.open_sheet()?.page.size;
        let size = [f64::from(self.stage.width), f64::from(self.stage.height)];
        let point = iced::Point::new(self.stage.width / 2., self.stage.height / 2.);
        let paper_point = self.view.xf(self.stage, paper).paper(point);
        Some(ModelCamera {
            center: v.model_unclipped(paper_point),
            size,
            resolution: self.model_resolution()?,
            rotation: v.total_rotation(),
        })
    }
    pub fn model_scope(&self) -> Option<&str> {
        self.model_scope.as_deref()
    }
    pub fn model_resolution(&self) -> Option<f64> {
        let v = self.active_viewport()?;
        let paper = self.open_sheet()?.page.size;
        Some(
            kentos_sheet::projection::Scale::for_rendering(v.denominator)
                .pixels_per_metre(self.view.xf(self.stage, paper).k),
        )
    }
    pub fn active_viewport(&self) -> Option<Viewport> {
        Viewport::resolve(&self.book, self.open.as_deref()?, self.model_scope()?).ok()
    }
    pub(crate) fn enter_model(&mut self, id: &str) -> Vec<Effect> {
        let Some(sheet) = self.open.as_deref() else {
            return vec![];
        };
        match Viewport::resolve(&self.book, sheet, id) {
            Ok(_) => {
                self.model_scope = Some(id.into());
                self.drag = None;
                self.preview = None;
                self.hovered_item = None;
                self.reset_caches();
                vec![Effect::ModelScope(true)]
            }
            Err(e) => vec![Effect::Notice(e.message)],
        }
    }
    pub(crate) fn model_stage(&mut self, e: StageEvent) -> Vec<Effect> {
        let Some(v) = self.active_viewport() else {
            self.model_scope = None;
            return vec![Effect::ModelScope(false)];
        };
        let Some(paper) = self.open_sheet().map(|s| s.page.size) else {
            return vec![];
        };
        let (action, at, clipped) = match e {
            StageEvent::RightClick { at } => (PointerAction::RightClick, at, true),
            StageEvent::DoubleClick { at } => {
                if v.model_point(at).is_none() {
                    return self.update(Message::ReturnPaper);
                }
                (PointerAction::Press, at, true)
            }
            StageEvent::Move { at, size, .. } => {
                self.stage = size;
                self.hover = Some(at);
                (PointerAction::Move, at, true)
            }
            StageEvent::Press { at, size, .. } => {
                self.stage = size;
                (PointerAction::Press, at, true)
            }
            StageEvent::Release { at, .. } => (PointerAction::Release, at, false),
            StageEvent::Leave => (PointerAction::Leave, [0., 0.], false),
            StageEvent::Resize { size } => {
                self.stage = size;
                return vec![];
            }
            StageEvent::Zoom { factor, px, size } => {
                self.stage = size;
                let at = self.view.xf(size, paper).paper(px);
                if v.model_point(at).is_some() {
                    self.commit(vec![Op::NavigateViewport(NavigateViewport {
                        id: v.item,
                        delta: [0., 0.],
                        factor,
                        anchor: Some(at),
                    })]);
                }
                return vec![];
            }
            StageEvent::Pan { dx, dy, size } => {
                self.stage = size;
                let k = self.view.xf(size, paper).k;
                self.commit(vec![Op::NavigateViewport(NavigateViewport {
                    id: v.item,
                    delta: [f64::from(dx) / k, f64::from(dy) / k],
                    factor: 1.,
                    anchor: None,
                })]);
                return vec![];
            }
        };
        let world = if clipped {
            v.model_point(at)
        } else {
            Some(v.model_unclipped(at))
        };
        let Some(world) = world else {
            return vec![Effect::ModelPointer(ModelPointer {
                action: PointerAction::Leave,
                world: [0., 0.],
                pixels_per_metre: 1.,
                rotation: 0,
            })];
        };
        let ppm = kentos_sheet::projection::Scale::for_rendering(v.denominator)
            .pixels_per_metre(self.view.xf(self.stage, paper).k);
        vec![Effect::ModelPointer(ModelPointer {
            action,
            world,
            pixels_per_metre: ppm,
            rotation: v.total_rotation(),
        })]
    }
}
