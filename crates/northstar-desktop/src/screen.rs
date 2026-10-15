// SPDX-License-Identifier: GPL-3.0-only
//! Reconstructed 80x25 monochrome screen cells; no original font or resources.
use crate::projection::Projection;
use northstar_core::Snapshot;

#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub glyph: char,
    pub attribute: u8,
}
pub struct Screen {
    pub cells: [[Cell; 80]; 25],
    pub cursor: Option<(usize, usize)>,
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            cells: [[Cell {
                glyph: ' ',
                attribute: 7,
            }; 80]; 25],
            cursor: None,
        }
    }
}
impl Screen {
    pub fn put(&mut self, row: usize, column: usize, text: &str, attribute: u8) {
        if let Some(line) = self.cells.get_mut(row) {
            for (cell, glyph) in line.iter_mut().skip(column).zip(text.chars()) {
                *cell = Cell { glyph, attribute };
            }
        }
    }
    fn border(&mut self, top: usize, bottom: usize, title: &str) {
        self.put(top, 0, &format!("╔{}╗", "═".repeat(77)), 7);
        self.put(bottom, 0, &format!("╚{}╝", "═".repeat(77)), 7);
        let title_start = (79 - title.len()) / 2;
        self.put(top, title_start, title, 7);
        for cell in &mut self.cells[top][title_start + 1..title_start + title.len() - 1] {
            cell.attribute = 15;
        }
        for row in top + 1..bottom {
            self.put(row, 0, "║", 7);
            self.put(row, 78, "║", 7);
        }
    }
    pub fn opening() -> Self {
        let mut s = Self::default();
        s.put(0, 26, "NorthStar WordStar 4.0", 15);
        s.border(1, 10, " O P E N I N G   M E N U ");
        for (row, line) in [
            "     D open a document                    L change logged drive/directory",
            "     N open a nondocument                 C protect a file",
            "     P print a file                       E rename a file",
            "     M merge print a file                 O copy a file",
            "     I index a document                   Y delete a file",
            "     T table of contents                  F turn directory off",
            "     X exit NorthStar                   Esc shorthand",
            "     J help                               R run a DOS command",
        ]
        .iter()
        .enumerate()
        {
            s.put(row + 2, 1, line, 7);
        }
        s.put(
            12,
            0,
            "Select D to open or create a document. J describes implemented commands.",
            7,
        );
        s
    }
    pub fn editor(
        projection: &Projection,
        state: &Snapshot,
        label: &str,
        prefix: Option<char>,
    ) -> Self {
        let mut s = Self::default();
        s.cells[0].fill(Cell {
            glyph: ' ',
            attribute: 0x70,
        });
        s.put(0, 3, &label.chars().take(19).collect::<String>(), 0x70);
        let (row, column) = projection.cursor;
        // The retained DOS captures establish P01 through L14. Later automatic
        // page transitions are not yet qualified, so do not guess their number.
        let page = if row < 14 && !projection.page_unknown_before_cursor {
            "01"
        } else {
            "??"
        };
        s.put(
            0,
            23,
            &format!("P{page} L{:02} C{:02}", row + 1, column + 1),
            0x70,
        );
        if state.insert {
            s.put(0, 35, "Insert", 0x70);
        }
        s.put(0, 42, "Align", 0x70);
        if let Some(prefix) = prefix {
            s.put(0, 0, &prefix.to_string(), 0x70);
        }
        s.border(1, 9, " E D I T   M E N U ");
        s.put(
            2,
            1,
            "    CURSOR      SCROLL        ERASE      OTHER               MENUS",
            7,
        );
        for (column, width) in [(5, 6), (17, 6), (31, 5), (42, 5), (62, 5)] {
            for cell in &mut s.cells[2][column..column + width] {
                cell.attribute = 15;
            }
        }
        let insert = if state.insert { "off" } else { "on " };
        let lines = [
            " ^E up       ^W up         ^G char    ^J help             ^O onscreen format".into(),
            " ^X down     ^Z down       ^T word    ^I tab              ^K block & save".into(),
            format!(
                " ^S left     ^R up screen  ^Y line    ^V turn insert {insert}  ^P print controls"
            ),
            " ^D right    ^C down      Del char    ^B align paragraph  ^Q quick functions".into(),
            " ^A word left   screen     ^U unerase ^N split the line  Esc shorthand".into(),
            " ^F word right                        ^L find/replace again".into(),
        ];
        for (index, line) in lines.iter().enumerate() {
            let r = index + 3;
            s.put(r, 1, line, 7);
            for (c, _) in line.match_indices('^') {
                s.cells[r][c + 1].attribute = 15;
                s.cells[r][c + 2].attribute = 15;
            }
        }
        for cell in &mut s.cells[6][27..30] {
            cell.attribute = 15;
        }
        for cell in &mut s.cells[7][58..61] {
            cell.attribute = 15;
        }
        s.put(10, 0, &format!("L{}R", "-".repeat(63)), 7);
        for column in (5..=55).step_by(5) {
            s.cells[10][column].glyph = '!';
        }
        let top = row.saturating_sub(11);
        let left = column.saturating_sub(78);
        for screen_row in 11..23 {
            let document_row = top + screen_row - 11;
            let mut indicator = '^';
            if let Some(cells) = projection.rows.get(document_row) {
                for (c, cell) in cells.iter().enumerate().skip(left).take(79) {
                    if cell.marker && matches!(cell.glyph, '¶' | '↵' | '!') {
                        continue;
                    }
                    s.cells[screen_row][c - left].glyph = cell.glyph;
                }
                if let Some(last) = cells.last() {
                    indicator = match last.glyph {
                        '¶' => '<',
                        '↵' => ' ',
                        '!' => '!',
                        _ => '^',
                    };
                }
            }
            s.put(screen_row, 79, &indicator.to_string(), 7);
        }
        s.cursor = Some((11 + row - top, column - left));
        s.put(
            23,
            0,
            " Display Center  ChkRest ChkWord Del Blk HideBlk MoveBlk CopyBlk Beg Blk1End Blk",
            7,
        );
        s.put(
            24,
            0,
            "1Help   2Undo   3Undrlin4Bold   5DelLine6DelWord7Align  8Ruler  9Save & 0Done   ",
            7,
        );
        for r in [23, 24] {
            for c in 0..80 {
                if c % 8 != 0 {
                    s.cells[r][c].attribute = 0x70;
                }
            }
        }
        s
    }
    /// Encode the reconstructed cells for comparison with retained B800 captures.
    /// Unsupported display glyphs have an explicit question-mark placeholder.
    pub fn monochrome_bytes(&self) -> Vec<u8> {
        self.cells
            .iter()
            .flatten()
            .flat_map(|c| {
                [
                    match c.glyph {
                        '╔' => 201,
                        '═' => 205,
                        '╗' => 187,
                        '║' => 186,
                        '╚' => 200,
                        '╝' => 188,
                        c if c.is_ascii() => c as u8,
                        _ => b'?',
                    },
                    c.attribute,
                ]
            })
            .collect()
    }
}
