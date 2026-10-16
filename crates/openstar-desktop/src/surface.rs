// SPDX-License-Identifier: GPL-3.0-only
use crate::Message;
use iced::{Color, Font, Point, Rectangle, Renderer, Size, Theme, mouse, widget::canvas};
use openstar_desktop::screen::Screen;

const COLUMNS: f32 = 80.0;
const ROWS: f32 = 25.0;
const CELL_ASPECT: f32 = 0.5;
// The embedded fonts have advances of 800/1600 em (VGA) and 600/1000 em (Plex).
const PLEX_ADVANCE: f32 = 0.6;
const PLEX_SIZE_RATIO: f32 = CELL_ASPECT / PLEX_ADVANCE;

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

pub(crate) struct Typography {
    pub font: Font,
    pub size: f32,
    pub row_height: f32,
    pub grid_width: f32,
    pub origin_x: f32,
    pub origin_y: f32,
    pub column_width: f32,
}

pub(crate) fn typography(bounds: Size, classic_font: bool) -> Typography {
    let grid = Grid::fit(bounds);
    let row_height = grid.height;
    Typography {
        font: Font::with_name(if classic_font {
            "Px437 IBM VGA 8x16"
        } else {
            "IBM Plex Mono"
        }),
        size: if classic_font {
            row_height
        } else {
            row_height * PLEX_SIZE_RATIO
        },
        row_height,
        grid_width: grid.width * COLUMNS,
        origin_x: grid.x,
        origin_y: grid.y,
        column_width: grid.width,
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
        let typography = typography(bounds.size(), self.classic_font);
        let font = typography.font;
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
        let text_size = typography.size;
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
                    position: Point::new(x + start as f32 * w, y + (r as f32 + 0.5) * h),
                    size: text_size.into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_and_dialog_typography_share_both_font_metrics() {
        for bounds in [Size::new(1120.0, 700.0), Size::new(2048.0, 1152.0)] {
            let grid = Grid::fit(bounds);
            let classic = typography(bounds, true);
            let plex = typography(bounds, false);
            assert_eq!(grid.y, 0.0);
            assert_eq!(grid.height * ROWS, bounds.height);
            assert_eq!(classic.row_height, grid.height);
            assert_eq!(plex.row_height, grid.height);
            assert_eq!(classic.grid_width, grid.width * COLUMNS);
            assert_eq!(plex.grid_width, grid.width * COLUMNS);
            assert_eq!(classic.size, grid.height);
            assert!((plex.size * PLEX_ADVANCE - grid.width).abs() < 0.0001);
        }
    }
}
