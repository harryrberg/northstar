// SPDX-License-Identifier: GPL-3.0-only
use crate::Message;
use iced::{Color, Font, Point, Rectangle, Renderer, Size, Theme, mouse, widget::canvas};
use northstar_desktop::screen::Screen;
pub struct Surface {
    pub screen: Screen,
    pub focused: bool,
    pub classic_font: bool,
}
impl canvas::Program<Message> for Surface {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::BLACK);
        let font = Font::with_name(if self.classic_font {
            "Px437 IBM VGA 8x16"
        } else {
            "IBM Plex Mono"
        });
        let aspect = if self.classic_font { 0.5 } else { 0.6 };
        let h = (bounds.width / (80.0 * aspect)).min(bounds.height / 25.0);
        let w = h * aspect;
        let x = (bounds.width - 80.0 * w) / 2.0;
        let y = (bounds.height - 25.0 * h) / 2.0;
        let gray = Color::from_rgb8(170, 170, 170);
        for (r, row) in self.screen.cells.iter().enumerate() {
            let mut column = 0;
            while column < row.len() {
                if row[column].attribute != 0x70 {
                    column += 1;
                    continue;
                }
                let start = column;
                while column < row.len() && row[column].attribute == 0x70 {
                    column += 1;
                }
                frame.fill_rectangle(
                    Point::new(x + start as f32 * w, y + r as f32 * h),
                    Size::new((column - start) as f32 * w, h),
                    gray,
                );
            }
            column = 0;
            while column < row.len() {
                let start = column;
                let attribute = row[start].attribute;
                while column < row.len() && row[column].attribute == attribute {
                    column += 1;
                }
                let content = row[start..column]
                    .iter()
                    .map(|cell| cell.glyph)
                    .collect::<String>();
                let content = content.trim_end_matches(' ');
                if content.is_empty() {
                    continue;
                }
                frame.fill_text(canvas::Text {
                    content: content.to_string(),
                    position: Point::new(x + start as f32 * w, y + (r as f32 + 0.5) * h),
                    size: h.into(),
                    font,
                    align_x: iced::widget::text::Alignment::Left,
                    align_y: iced::alignment::Vertical::Center,
                    color: if attribute == 0x70 {
                        Color::BLACK
                    } else if attribute == 15 {
                        Color::WHITE
                    } else {
                        gray
                    },
                    ..Default::default()
                });
            }
        }
        if self.focused
            && let Some((r, c)) = self.screen.cursor
        {
            frame.fill_rectangle(
                Point::new(x + c as f32 * w, y + (r as f32 + 0.88) * h),
                Size::new(w, (h * 0.12).max(1.0)),
                Color::WHITE,
            );
        }
        vec![frame.into_geometry()]
    }
}
