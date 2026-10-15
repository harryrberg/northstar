// SPDX-License-Identifier: GPL-3.0-only
use crate::Message;
use iced::{Color, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse, widget::canvas};
use northstar_desktop::screen::Screen;

const COLUMNS: f32 = 80.0;
const ROWS: f32 = 25.0;
const CELL_ASPECT: f32 = 0.5;
// The embedded fonts have advances of 800/1600 em (VGA) and 600/1000 em (Plex).
const PLEX_X_SCALE: f32 = CELL_ASPECT / 0.6;

struct Grid {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Grid {
    fn fit(bounds: Size) -> Self {
        let height = (bounds.width / (COLUMNS * CELL_ASPECT)).min(bounds.height / ROWS);
        let width = height * CELL_ASPECT;
        Self {
            x: (bounds.width - COLUMNS * width) / 2.0,
            y: (bounds.height - ROWS * height) / 2.0,
            width,
            height,
        }
    }
}

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
        let Grid {
            x,
            y,
            width: w,
            height: h,
        } = Grid::fit(bounds.size());
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
        }
        let text_x_scale = if self.classic_font { 1.0 } else { PLEX_X_SCALE };
        frame.with_save(|frame| {
            frame.scale_nonuniform(Vector::new(text_x_scale, 1.0));
            for (r, row) in self.screen.cells.iter().enumerate() {
                let mut column = 0;
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
                        position: Point::new(
                            (x + start as f32 * w) / text_x_scale,
                            y + (r as f32 + 0.5) * h,
                        ),
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
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_fills_height_on_widescreen_in_both_font_modes() {
        let grid = Grid::fit(Size::new(2048.0, 1152.0));
        assert_eq!(grid.y, 0.0);
        assert_eq!(grid.height * ROWS, 1152.0);
        assert_eq!(PLEX_X_SCALE * 0.6, CELL_ASPECT);
    }
}
