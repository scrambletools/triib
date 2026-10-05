//! Fitting text to the room it has, for widgets that draw their own.

use iced::advanced::text::{Alignment, Paragraph as _, Shaping, Wrapping};
use iced::alignment::Vertical;
use iced::widget::text::LineHeight;
use iced::{Font, Pixels, Size};

/// Text cut to fit, and its width.
#[derive(Debug, Clone)]
pub struct Fitted {
    pub name: String,
    pub name_width: f32,
}

/// How wide `content` is in `font` at `size` pixels.
pub fn measure(content: &str, font: Font, size: f32) -> f32 {
    use iced::advanced::graphics::text::Paragraph;
    Paragraph::with_text(iced::advanced::text::Text {
        content,
        bounds: Size::INFINITE,
        size: Pixels(size),
        line_height: LineHeight::Absolute(Pixels(size + 4.0)),
        font,
        align_x: Alignment::Default,
        align_y: Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::None,
    })
    .min_width()
}

/// `content` cut short with an ellipsis to fit `max` pixels.
pub fn fit(content: &str, font: Font, size: f32, max: f32) -> Fitted {
    let width = measure(content, font, size);
    if width <= max {
        return Fitted {
            name: content.to_owned(),
            name_width: width,
        };
    }
    let ends: Vec<usize> = content
        .char_indices()
        .map(|(index, _)| index)
        .skip(1)
        .collect();
    let cut = |count: usize| {
        let end = ends.get(count).copied().unwrap_or(content.len());
        format!("{}…", content[..end].trim_end())
    };
    // The most characters that fit, found by halving.
    let (mut low, mut high) = (0, ends.len());
    while low < high {
        let middle = (low + high).div_ceil(2);
        if measure(&cut(middle), font, size) <= max {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let name = cut(low);
    Fitted {
        name_width: measure(&name, font, size),
        name,
    }
}
