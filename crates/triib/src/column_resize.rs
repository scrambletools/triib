//! A table heading whose column divider drags to widen or narrow the
//! column, reporting the width it would have; a double click on the
//! divider asks for the width that fits the column's text again.

use iced::advanced::layout::{self, Layout};
use iced::advanced::mouse::click;
use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay};
use iced::mouse::{self, Cursor};
use iced::{Background, Element, Event, Length, Rectangle, Size, Theme, Vector};
use scramble_ui::Scheme;

/// How far either side of the divider it can be grabbed.
const GRAB: f32 = 4.0;
/// The narrowest a column gets.
pub const MIN_WIDTH: f32 = 48.0;

pub struct Resizable<'a, Message> {
    content: Element<'a, Message>,
    width: Length,
    /// How far right of the heading its column's divider is.
    divider: f32,
    on_resize: Box<dyn Fn(f32) -> Message + 'a>,
    on_end: Message,
    on_reset: Message,
}

/// `content` as a heading with a draggable right edge: `on_resize` gets
/// each new width while dragging, `on_end` follows the drag, and a double
/// click on the edge sends `on_reset`.
pub fn resizable<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    on_resize: impl Fn(f32) -> Message + 'a,
    on_end: Message,
    on_reset: Message,
) -> Resizable<'a, Message> {
    Resizable {
        content: content.into(),
        width: Length::Fill,
        divider: 0.0,
        on_resize: Box::new(on_resize),
        on_end,
        on_reset,
    }
}

impl<Message> Resizable<'_, Message> {
    /// The heading's width: its column's, as a table lays fixed and fluid
    /// cells out apart.
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// How far right of the heading its column's divider is, as the
    /// table's padding sets it: where it is grabbed.
    pub fn divider(mut self, divider: f32) -> Self {
        self.divider = divider;
        self
    }
}

#[derive(Default)]
struct State {
    /// Where the drag started and the width then.
    drag: Option<(f32, f32)>,
    hovered: bool,
    last_click: Option<click::Click>,
}

/// Where the divider right of `bounds`, `divider` away, is grabbed.
fn edge(bounds: Rectangle, divider: f32) -> Rectangle {
    Rectangle {
        x: bounds.x + bounds.width + divider - GRAB,
        width: 2.0 * GRAB,
        ..bounds
    }
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, iced::Renderer> for Resizable<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(self.width);
        let content = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &limits);
        let size = limits.resolve(self.width, Length::Shrink, content.size());
        layout::Node::with_children(size, vec![content])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let Some(content) = layout.children().next() else {
            return;
        };
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], content, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let state = tree.state.downcast_mut::<State>();
        let hovered = cursor.is_over(edge(bounds, self.divider));
        if hovered != state.hovered {
            state.hovered = hovered;
            shell.request_redraw();
        }
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if hovered => {
                if let Some(position) = cursor.position() {
                    let click = click::Click::new(position, mouse::Button::Left, state.last_click);
                    state.last_click = Some(click);
                    if click.kind() == click::Kind::Double {
                        state.drag = None;
                        shell.publish(self.on_reset.clone());
                    } else {
                        state.drag = Some((position.x, bounds.width));
                    }
                    shell.capture_event();
                    shell.request_redraw();
                }
                return;
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some((origin, start)) = state.drag {
                    let width = (start + position.x - origin).max(MIN_WIDTH);
                    shell.publish((self.on_resize)(width));
                    shell.capture_event();
                    return;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.drag.take().is_some() =>
            {
                shell.publish(self.on_end.clone());
                shell.capture_event();
                shell.request_redraw();
                return;
            }
            _ => {}
        }
        let Some(content) = layout.children().next() else {
            return;
        };
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            content,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        if let Some(content) = layout.children().next() {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                content,
                cursor,
                viewport,
            );
        }
        let state = tree.state.downcast_ref::<State>();
        if !state.hovered && state.drag.is_none() {
            return;
        }
        // The divider being grabbed, drawn over the table's line.
        let scheme = Scheme::of(theme);
        let bounds = layout.bounds();
        let color = if state.drag.is_some() {
            scheme.primary
        } else {
            scheme.outline
        };
        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    x: bounds.x + bounds.width + self.divider - 1.0,
                    width: 2.0,
                    ..bounds
                },
                ..Quad::default()
            },
            Background::Color(color),
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        if state.drag.is_some() || cursor.is_over(edge(layout.bounds(), self.divider)) {
            return mouse::Interaction::ResizingHorizontally;
        }
        layout
            .children()
            .next()
            .map_or_else(mouse::Interaction::default, |content| {
                self.content.as_widget().mouse_interaction(
                    &tree.children[0],
                    content,
                    cursor,
                    viewport,
                    renderer,
                )
            })
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        let content = layout.children().next()?;
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            content,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: Clone + 'a> From<Resizable<'a, Message>> for Element<'a, Message> {
    fn from(resizable: Resizable<'a, Message>) -> Self {
        Element::new(resizable)
    }
}
