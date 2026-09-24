//! Açılır panel: bir alanın altında açılan, durumunu kendisi tutan panel.
//!
//! Tarih, saat ve liste seçicilerinin ortak altyapısıdır. Alan (`anchor`)
//! uygulamanın öğesidir; yazı girişi gibi etkileşimleri doğrudan uygulamaya
//! gider. Panel ise kendi iç mesajlarıyla (`Local`) çalışır: takvimde ay
//! değiştirmek gibi ara adımlar uygulamaya uğramaz, yalnızca bir değer
//! seçildiğinde uygulamanın mesajı üretilir.
//!
//! Panel alanın altına, sığmazsa üstüne açılır; dışarı tıklanınca, Esc'e
//! basılınca ya da pencere odağını yitirince kapanır. Alanın yanındaki
//! tetikleyici (ör. takvim düğmesi) paneli açıp kapatır; tetikleyici
//! verilmezse alanın tamamı tetikleyicidir.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{self, Operation, Tree, Widget, operation, tree};
use iced::advanced::{Clipboard, Shell, clipboard, overlay, renderer};
use iced::time::Instant;
use iced::{
    Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector, keyboard, mouse,
    window,
};

/// Alanla tetikleyici arasındaki boşluk.
const TRIGGER_GAP: f32 = 2.0;
/// Panelle alan arasındaki boşluk.
const PANEL_GAP: f32 = 3.0;

/// Panelin iç mesajına verilen yanıt.
pub(crate) struct Reaction<Message> {
    /// Uygulamaya iletilecek mesaj.
    pub publish: Option<Message>,
    /// Panel kapansın mı.
    pub close: bool,
}

impl<Message> Reaction<Message> {
    /// Yalnızca panelin durumu değişti.
    pub fn stay() -> Self {
        Self {
            publish: None,
            close: false,
        }
    }

    /// Mesaj iletilir, panel açık kalır.
    pub fn publish(message: Message) -> Self {
        Self {
            publish: Some(message),
            close: false,
        }
    }

    /// Mesaj iletilir ve panel kapanır.
    pub fn commit(message: Message) -> Self {
        Self {
            publish: Some(message),
            close: true,
        }
    }

    /// Panel kapanır.
    pub fn close() -> Self {
        Self {
            publish: None,
            close: true,
        }
    }
}

/// Paneli iç durumdan kuran fonksiyon.
type Build<'a, S, Local> = Box<dyn Fn(&S) -> Element<'a, Local> + 'a>;

/// İç mesajı uygulayan fonksiyon.
type Reduce<'a, S, Local, Message> = Box<dyn Fn(&mut S, Local) -> Reaction<Message> + 'a>;

/// Açılır panel.
pub(crate) struct Dropdown<'a, Message, Local, S> {
    anchor: Element<'a, Message>,
    trigger: Option<Element<'a, ()>>,
    init: Box<dyn Fn() -> S + 'a>,
    panel: Build<'a, S, Local>,
    reduce: Reduce<'a, S, Local, Message>,
    focus: Option<widget::Id>,
    match_width: bool,
    /// Kurulmuş panel ve kurulduğu durum sürümü. Panel yalnızca durum
    /// değişince yeniden kurulur: düğmeler durumlarını (üzerinde, basılı)
    /// öğenin kendisinde tutar ve her kurulumda yitirir.
    built: Option<(u64, Element<'a, Local>)>,
}

impl<'a, Message, Local, S> Dropdown<'a, Message, Local, S>
where
    Message: 'a,
    Local: Clone + 'a,
    S: 'static,
{
    /// `init` panel açılırken iç durumu kurar; `panel` içeriği durumdan
    /// çizer; `reduce` iç mesajları uygular.
    pub fn new(
        anchor: impl Into<Element<'a, Message>>,
        init: impl Fn() -> S + 'a,
        panel: impl Fn(&S) -> Element<'a, Local> + 'a,
        reduce: impl Fn(&mut S, Local) -> Reaction<Message> + 'a,
    ) -> Self {
        Self {
            anchor: anchor.into(),
            trigger: None,
            init: Box::new(init),
            panel: Box::new(panel),
            reduce: Box::new(reduce),
            focus: None,
            match_width: false,
            built: None,
        }
    }

    /// Alanın sağındaki tetikleyici; herhangi bir mesajı paneli açıp kapatır.
    pub fn trigger(mut self, trigger: impl Into<Element<'a, ()>>) -> Self {
        self.trigger = Some(trigger.into());
        self
    }

    /// Panel açılınca odaklanacak öğe (ör. arama kutusu).
    pub fn focus(mut self, id: widget::Id) -> Self {
        self.focus = Some(id);
        self
    }

    /// Panel en az alan kadar geniş olur (liste seçicileri).
    pub fn match_width(mut self) -> Self {
        self.match_width = true;
        self
    }
}

struct State<S> {
    open: Option<Open<S>>,
    /// Açılış, kapanış ve iç mesajlarla artan sürüm.
    revision: u64,
}

struct Open<S> {
    inner: S,
    /// Açılışta odak verildi mi.
    focused: bool,
}

impl<S> Default for State<S> {
    fn default() -> Self {
        Self {
            open: None,
            revision: 0,
        }
    }
}

impl<S> State<S> {
    fn toggle(&mut self, init: impl FnOnce() -> S) {
        self.revision += 1;
        self.open = match self.open.take() {
            Some(_) => None,
            None => Some(Open {
                inner: init(),
                focused: false,
            }),
        };
    }
}

impl<'a, Message, Local, S> Widget<Message, Theme, Renderer> for Dropdown<'a, Message, Local, S>
where
    Message: 'a,
    Local: Clone + 'a,
    S: 'static,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<S>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<S>::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![
            Tree::new(&self.anchor),
            self.trigger.as_ref().map_or_else(Tree::empty, Tree::new),
            Tree::empty(),
        ]
    }

    fn diff(&self, tree: &mut Tree) {
        if tree.children.len() != 3 {
            tree.children = self.children();
            return;
        }

        tree.children[0].diff(&self.anchor);

        if let Some(trigger) = &self.trigger {
            tree.children[1].diff(trigger);
        }
    }

    fn size(&self) -> Size<Length> {
        self.anchor.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.anchor.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let Some(trigger) = &mut self.trigger else {
            let anchor =
                self.anchor
                    .as_widget_mut()
                    .layout(&mut tree.children[0], renderer, limits);

            return layout::Node::with_children(anchor.size(), vec![anchor]);
        };

        let trigger =
            trigger
                .as_widget_mut()
                .layout(&mut tree.children[1], renderer, &limits.loose());
        let trigger_size = trigger.size();

        let anchor = self.anchor.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &limits.shrink(Size::new(trigger_size.width + TRIGGER_GAP, 0.0)),
        );
        let anchor_size = anchor.size();

        let height = anchor_size.height.max(trigger_size.height);

        layout::Node::with_children(
            Size::new(anchor_size.width + TRIGGER_GAP + trigger_size.width, height),
            vec![
                anchor.move_to(Point::new(0.0, (height - anchor_size.height) / 2.0)),
                trigger.move_to(Point::new(
                    anchor_size.width + TRIGGER_GAP,
                    (height - trigger_size.height) / 2.0,
                )),
            ],
        )
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let mut layouts = layout.children();
        let Some(anchor_layout) = layouts.next() else {
            return;
        };

        self.anchor.as_widget_mut().update(
            &mut tree.children[0],
            event,
            anchor_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let state = tree.state.downcast_mut::<State<S>>();

        match (&mut self.trigger, layouts.next()) {
            (Some(trigger), Some(trigger_layout)) => {
                let mut toggles = Vec::new();
                let mut local = Shell::new(&mut toggles);

                trigger.as_widget_mut().update(
                    &mut tree.children[1],
                    event,
                    trigger_layout,
                    cursor,
                    renderer,
                    clipboard,
                    &mut local,
                    viewport,
                );

                propagate(&local, shell);
                drop(local);

                if !toggles.is_empty() {
                    state.toggle(&self.init);
                    shell.invalidate_layout();
                    shell.request_redraw();
                }
            }
            _ => {
                if !shell.is_event_captured()
                    && let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
                    && cursor.is_over(layout.bounds())
                {
                    state.toggle(&self.init);
                    shell.capture_event();
                    shell.invalidate_layout();
                    shell.request_redraw();
                }
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let mut layouts = layout.children();

        let anchor = layouts.next().map_or(mouse::Interaction::None, |layout| {
            self.anchor.as_widget().mouse_interaction(
                &tree.children[0],
                layout,
                cursor,
                viewport,
                renderer,
            )
        });

        match (&self.trigger, layouts.next()) {
            (Some(trigger), Some(layout)) => {
                let trigger = trigger.as_widget().mouse_interaction(
                    &tree.children[1],
                    layout,
                    cursor,
                    viewport,
                    renderer,
                );

                if trigger == mouse::Interaction::None {
                    anchor
                } else {
                    trigger
                }
            }
            _ if anchor == mouse::Interaction::None && cursor.is_over(layout.bounds()) => {
                mouse::Interaction::Pointer
            }
            _ => anchor,
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let mut layouts = layout.children();

        if let Some(anchor) = layouts.next() {
            self.anchor.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                anchor,
                cursor,
                viewport,
            );
        }

        if let (Some(trigger), Some(layout)) = (&self.trigger, layouts.next()) {
            trigger.as_widget().draw(
                &tree.children[1],
                renderer,
                theme,
                style,
                layout,
                cursor,
                viewport,
            );
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(anchor) = layout.children().next() {
            self.anchor
                .as_widget_mut()
                .operate(&mut tree.children[0], anchor, renderer, operation);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State<S>>();
        let [anchor_tree, _, panel_tree] = children.as_mut_slice() else {
            return None;
        };

        let anchor = layout.children().next().and_then(|anchor_layout| {
            self.anchor.as_widget_mut().overlay(
                anchor_tree,
                anchor_layout,
                renderer,
                viewport,
                translation,
            )
        });

        let panel = match &state.open {
            Some(open) => {
                let revision = state.revision;

                if self
                    .built
                    .as_ref()
                    .is_none_or(|(built, _)| *built != revision)
                {
                    let mut panel = (self.panel)(&open.inner);
                    panel_tree.diff(&panel);
                    prime(&mut panel, panel_tree, renderer);
                    self.built = Some((revision, panel));
                }

                let (_, panel) = self.built.as_mut()?;

                Some(overlay::Element::new(Box::new(Panel {
                    anchor: layout.bounds() + translation,
                    panel,
                    tree: panel_tree,
                    state,
                    reduce: &*self.reduce,
                    focus: self.focus.clone(),
                    match_width: self.match_width,
                })))
            }
            None => {
                self.built = None;
                None
            }
        };

        match (anchor, panel) {
            (Some(anchor), Some(panel)) => {
                Some(overlay::Group::with_children(vec![anchor, panel]).overlay())
            }
            (anchor, panel) => anchor.or(panel),
        }
    }
}

impl<'a, Message, Local, S> From<Dropdown<'a, Message, Local, S>> for Element<'a, Message>
where
    Message: 'a,
    Local: Clone + 'a,
    S: 'static,
{
    fn from(dropdown: Dropdown<'a, Message, Local, S>) -> Self {
        Element::new(dropdown)
    }
}

/// Yeni kurulan paneli bir "yeniden çiz" olayıyla hazırlar: düğmeler
/// durumlarını (etkin, devre dışı) bu olayda belirler; aksi hâlde ilk
/// çizimde devre dışı görünürler.
fn prime<Local>(panel: &mut Element<'_, Local>, tree: &mut Tree, renderer: &Renderer) {
    let limits = layout::Limits::new(Size::ZERO, Size::new(4096.0, 4096.0));
    let node = panel.as_widget_mut().layout(tree, renderer, &limits);
    let bounds = Rectangle::with_size(node.size());

    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);

    panel.as_widget_mut().update(
        tree,
        &Event::Window(window::Event::RedrawRequested(Instant::now())),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        renderer,
        &mut clipboard::Null,
        &mut shell,
        &bounds,
    );
}

/// Yerel kabuğun durumunu (yakalama, yeniden çizim, yerleşim, yazı girişi)
/// dış kabuğa aktarır; mesajlar aktarılmaz.
fn propagate<A, B>(local: &Shell<'_, A>, shell: &mut Shell<'_, B>) {
    if local.is_event_captured() {
        shell.capture_event();
    }

    if local.is_layout_invalid() {
        shell.invalidate_layout();
    }

    shell.request_redraw_at(local.redraw_request());
    shell.request_input_method(local.input_method());
}

/// Açık panel.
struct Panel<'a, 'b, Message, Local, S> {
    /// Alanın penceredeki sınırları.
    anchor: Rectangle,
    panel: &'b mut Element<'a, Local>,
    tree: &'b mut Tree,
    state: &'b mut State<S>,
    reduce: &'b dyn Fn(&mut S, Local) -> Reaction<Message>,
    focus: Option<widget::Id>,
    match_width: bool,
}

impl<Message, Local, S> Panel<'_, '_, Message, Local, S> {
    fn close(&mut self, shell: &mut Shell<'_, Message>) {
        self.state.open = None;
        self.state.revision += 1;
        shell.invalidate_layout();
        shell.request_redraw();
    }
}

impl<Message, Local: Clone, S> overlay::Overlay<Message, Theme, Renderer>
    for Panel<'_, '_, Message, Local, S>
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let minimum = if self.match_width {
            Size::new(self.anchor.width, 0.0)
        } else {
            Size::ZERO
        };

        let node = self.panel.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(minimum, bounds),
        );
        let size = node.size();

        // Altına sığmazsa ve üstünde daha çok yer varsa üstüne açılır.
        let below = self.anchor.y + self.anchor.height + PANEL_GAP;
        let above = self.anchor.y - PANEL_GAP - size.height;
        let y = if below + size.height <= bounds.height || above < 0.0 {
            below.min((bounds.height - size.height).max(0.0))
        } else {
            above
        };
        let x = self.anchor.x.min(bounds.width - size.width).max(0.0);

        layout::Node::with_children(bounds, vec![node.move_to(Point::new(x, y))])
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        if let Some(panel) = layout.children().next() {
            self.panel.as_widget().draw(
                self.tree,
                renderer,
                theme,
                style,
                panel,
                cursor,
                &layout.bounds(),
            );
        }
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let Some(panel_layout) = layout.children().next() else {
            return;
        };

        let over = cursor.is_over(panel_layout.bounds());

        // Açılışta arama kutusu gibi öğeye odak verilir.
        if let Some(open) = &mut self.state.open
            && !open.focused
        {
            open.focused = true;

            if let Some(id) = self.focus.clone() {
                self.panel.as_widget_mut().operate(
                    self.tree,
                    panel_layout,
                    renderer,
                    &mut operation::focusable::focus(id),
                );
            }
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(button)) if !over => {
                // Alanın kendisine (ör. yazı girişine) tıklamak paneli
                // kapatmaz; tetikleyiciye tıklamak alan tarafından ele alınır.
                if cursor.is_over(self.anchor) {
                    return;
                }

                self.close(shell);

                if *button != mouse::Button::Right {
                    shell.capture_event();
                }

                return;
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) => {
                self.close(shell);
                shell.capture_event();
                return;
            }
            Event::Window(window::Event::Unfocused) => {
                self.close(shell);
                return;
            }
            _ => {}
        }

        let mut messages = Vec::new();
        let mut local = Shell::new(&mut messages);

        self.panel.as_widget_mut().update(
            self.tree,
            event,
            panel_layout,
            cursor,
            renderer,
            clipboard,
            &mut local,
            &layout.bounds(),
        );

        propagate(&local, shell);
        drop(local);

        for message in messages {
            let Some(open) = self.state.open.as_mut() else {
                break;
            };

            let reaction = (self.reduce)(&mut open.inner, message);
            self.state.revision += 1;

            if let Some(message) = reaction.publish {
                shell.publish(message);
            }

            if reaction.close {
                self.state.open = None;
            }

            shell.invalidate_layout();
            shell.request_redraw();
        }

        // Panelin üzerindeki fare olayları alttaki öğelere geçmez.
        if over && matches!(event, Event::Mouse(_)) {
            shell.capture_event();
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let Some(panel) = layout.children().next() else {
            return mouse::Interaction::None;
        };

        if !cursor.is_over(panel.bounds()) {
            return mouse::Interaction::None;
        }

        match self.panel.as_widget().mouse_interaction(
            self.tree,
            panel,
            cursor,
            &layout.bounds(),
            renderer,
        ) {
            mouse::Interaction::None => mouse::Interaction::Idle,
            interaction => interaction,
        }
    }
}
