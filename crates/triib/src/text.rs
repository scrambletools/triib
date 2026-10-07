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

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::graphics::text::Paragraph;

    /// Where the cursor before each character of `content` falls, as it
    /// is drawn.
    fn positions(content: &str) -> Vec<f32> {
        let paragraph = Paragraph::with_text(iced::advanced::text::Text {
            content,
            bounds: Size::INFINITE,
            size: Pixels(14.0),
            line_height: LineHeight::Absolute(Pixels(18.0)),
            font: Font::DEFAULT,
            align_x: Alignment::Default,
            align_y: Vertical::Top,
            shaping: Shaping::Advanced,
            wrapping: Wrapping::None,
        });
        (0..content.chars().count())
            .map(|index| paragraph.grapheme_position(0, index).unwrap().x)
            .collect()
    }

    /// Text cut short ends in an ellipsis on the side it reads towards: the
    /// right for left to right text, the left for right to left text.
    #[test]
    fn text_is_cut_on_the_side_it_ends() {
        for (content, right_to_left) in [
            ("AVB Example Entity on the stage", false),
            ("ישות לדוגמה על הבמה הראשית", true),
            ("كيان تجريبي على المسرح الرئيسي", true),
        ] {
            let fitted = fit(content, Font::DEFAULT, 14.0, 90.0);
            assert!(fitted.name.ends_with('…'), "{content}: {}", fitted.name);
            assert!(
                fitted.name_width <= 90.0,
                "{content}: {}",
                fitted.name_width
            );
            let places = positions(&fitted.name);
            let (first, ellipsis) = (places[0], places[places.len() - 1]);
            assert_eq!(
                ellipsis < first,
                right_to_left,
                "{content}: the ellipsis at {ellipsis}, the first letter at {first}"
            );
        }
    }
}
