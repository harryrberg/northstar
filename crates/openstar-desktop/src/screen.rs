// SPDX-License-Identifier: GPL-3.0-only
//! Reconstructed 80x25 monochrome screen cells; no original font or resources.
use crate::projection::Projection;
use openstar_core::Snapshot;

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
    pub fn dialogue(command: &str) -> Self {
        let mut screen = Self::default();
        screen.cells[0].fill(Cell {
            glyph: ' ',
            attribute: 0x70,
        });
        screen.put(0, 0, command, 0x70);
        let title = "OpenStar 4.0";
        screen.put(0, (80 - title.len()) / 2, title, 0x70);
        screen
    }
    fn clear_find_header(&mut self) {
        for row in 1..=9 {
            self.cells[row].fill(Cell {
                glyph: ' ',
                attribute: 7,
            });
        }
        self.cells[0][0] = Cell {
            glyph: 'Q',
            attribute: 0x70,
        };
        self.cells[0][1] = Cell {
            glyph: 'F',
            attribute: 0x70,
        };
    }

    pub fn find_prompt(&mut self, term: &str, options: Option<&str>) {
        self.clear_find_header();
        self.put(2, 3, &format!("Find what? {term}"), 7);
        self.put(3, 1, "↵ done  : Backspace or ^H erase left", 7);
        self.put(4, 1, "^U cancel :          Del erase", 7);
        if let Some(options) = options {
            self.put(5, 3, &format!("Option(s)? {options}"), 7);
            self.put(
                6,
                3,
                "W whole words   U ignore case   B look backwards   ? wildcards",
                7,
            );
            self.put(7, 3, "G start from beginning or end", 7);
        }
        self.cursor = None;
    }

    pub fn replace_prompt(&mut self, term: &str, replacement: Option<&str>, options: Option<&str>) {
        self.clear_find_header();
        self.cells[0][1].glyph = 'A';
        self.put(2, 3, &format!("Find what? {term}"), 7);
        self.put(3, 1, "↵ done  : Backspace or ^H erase left", 7);
        self.put(4, 1, "^U cancel :          Del erase", 7);
        if let Some(replacement) = replacement {
            self.put(5, 1, &format!("Replace with? {replacement}"), 7);
        }
        if let Some(options) = options {
            self.put(6, 3, &format!("Option(s)? {options}"), 7);
            self.put(
                7,
                3,
                "W whole words   U ignore case   N replace without asking",
                7,
            );
        }
        self.cursor = None;
    }

    pub fn replace_question(&mut self) {
        self.put(0, 68, "Replace Y/N", 0x70);
    }

    pub fn find_notice(&mut self, term: &str, repeated: bool) {
        self.clear_find_header();
        self.cells[0].fill(Cell {
            glyph: ' ',
            attribute: 0x70,
        });
        self.cells[0][0] = Cell {
            glyph: if repeated { 'L' } else { 'Q' },
            attribute: 0x70,
        };
        self.cells[0][1] = Cell {
            glyph: if repeated { ' ' } else { 'F' },
            attribute: 0x70,
        };
        self.put(0, 3, "Wait", 0x70);
        self.put(
            2,
            0,
            &format!(
                "{}:  {term}",
                if repeated {
                    "End of search for"
                } else {
                    "Could not find"
                }
            ),
            7,
        );
        self.put(4, 0, "Press Esc to continue.", 7);
        self.cursor = None;
    }
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
        let title = "OpenStar 4.0";
        s.cells[0].fill(Cell {
            glyph: ' ',
            attribute: 0x70,
        });
        s.put(0, (80 - title.len()) / 2, title, 0x70);
        s.border(1, 10, " O P E N I N G   M E N U ");
        for (row, line) in [
            "     D open a document                    L change logged drive/directory",
            "     N open a nondocument                 C protect a file",
            "     P print a file                       E rename a file",
            "     M merge print a file                 O copy a file",
            "     I index a document                   Y delete a file",
            "     T table of contents                  F turn directory off",
            "     X exit OpenStar                   Esc shorthand",
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
    pub fn opening_help(&mut self) {
        self.put(0, 0, "J", 0x70);
        for row in 11..25 {
            self.cells[row].fill(Cell {
                glyph: ' ',
                attribute: 7,
            });
        }
        self.put(11, 0, "The Opening Menu above shows the main commands.", 7);
        self.put(
            12,
            0,
            "D opens a document; F2 and F3 work with UTF-8 files.",
            7,
        );
        self.put(
            14,
            0,
            "D, J, X and F2-F4 are currently available from this menu.",
            7,
        );
        self.put(16, 0, "Press Esc to return to the Opening Menu.", 7);
        self.put(21, 0, crate::APP_TITLE, 7);
        self.cursor = None;
    }
    pub fn editor_help(&mut self, utf8: bool) {
        for row in 11..23 {
            self.cells[row].fill(Cell {
                glyph: ' ',
                attribute: 7,
            });
        }
        self.put(12, 0, "The Edit Menu above shows the main editing keys.", 7);
        self.put(
            13,
            0,
            "^ means hold Control while pressing the following key.",
            7,
        );
        if utf8 {
            self.put(
                15,
                0,
                "F2 starts a new UTF-8 document; F3 opens one from the menu.",
                7,
            );
            self.put(16, 0, "Ctrl-Z undoes; Ctrl-Y redoes; Ctrl-K S saves.", 7);
        } else {
            self.put(15, 0, "Ctrl-S/D move left/right; Ctrl-E/X move up/down.", 7);
            self.put(
                16,
                0,
                "Ctrl-K S saves; Ctrl-K D saves and returns to the menu.",
                7,
            );
        }
        self.put(18, 0, "F11 toggles full screen; F12 changes the font.", 7);
        self.put(20, 0, "Press Esc to return to your document.", 7);
        self.put(22, 0, crate::APP_TITLE, 7);
        self.cursor = None;
    }
    pub fn editor(
        projection: &Projection,
        state: &Snapshot,
        label: &str,
        prefix: Option<char>,
    ) -> Self {
        Self::editor_view(projection, state.insert, label, prefix, None)
    }

    pub fn utf8_editor(
        projection: &Projection,
        label: &str,
        prefix: Option<char>,
        viewport_top: usize,
    ) -> Self {
        Self::editor_view(projection, true, label, prefix, Some(viewport_top))
    }

    fn editor_view(
        projection: &Projection,
        insert_mode: bool,
        label: &str,
        prefix: Option<char>,
        utf8_viewport_top: Option<usize>,
    ) -> Self {
        let utf8 = utf8_viewport_top.is_some();
        let mut s = Self::default();
        s.cells[0].fill(Cell {
            glyph: ' ',
            attribute: 0x70,
        });
        s.put(0, 3, &label.chars().take(19).collect::<String>(), 0x70);
        let (row, column) = projection.cursor;
        // The retained DOS captures establish P01 through L14. Later automatic
        // page transitions are not yet qualified, so do not guess their number.
        let page = if !utf8 && row < 14 && !projection.page_unknown_before_cursor {
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
        if insert_mode {
            s.put(0, 35, "Insert", 0x70);
        }
        s.put(0, 42, if utf8 { "Text " } else { "Align" }, 0x70);
        if utf8 {
            s.put(0, 60, "UTF-8", 0x70);
        }
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
        let lines: [String; 6] = if utf8 {
            [
                " ^S left       ^D right        ^E up       ^X down".into(),
                " Arrows move   Backspace/Delete   Enter new line".into(),
                " ^V paste      ^Z undo         ^Y redo      ^K S save".into(),
                " ^K D save and done   ^K Q done   ^J help   F1 help".into(),
                " F11 full screen              F12 change font".into(),
                " UTF-8 wraps on screen; stored text is unchanged".into(),
            ]
        } else {
            let insert = if insert_mode { "off" } else { "on " };
            [
                " ^E up       ^W up         ^G char    ^J help             ^O onscreen format"
                    .into(),
                " ^X down     ^Z down       ^T word    ^I tab              ^K block & save".into(),
                format!(
                    " ^S left     ^R up screen  ^Y line    ^V turn insert {insert}  ^P print controls"
                ),
                " ^D right    ^C down      Del char    ^B align paragraph  ^Q quick functions"
                    .into(),
                " ^A word left   screen     ^U unerase ^N split the line  Esc shorthand".into(),
                " ^F word right                        ^L find/replace again".into(),
            ]
        };
        for (index, line) in lines.iter().enumerate() {
            let r = index + 3;
            s.put(r, 1, line, 7);
            for (c, _) in line.match_indices('^') {
                s.cells[r][c + 1].attribute = 15;
                s.cells[r][c + 2].attribute = 15;
            }
        }
        if utf8 {
            s.put(10, 0, &"-".repeat(80), 7);
        } else {
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
        }
        let top = utf8_viewport_top.unwrap_or_else(|| row.saturating_sub(11));
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
                    if cell.selected {
                        s.cells[screen_row][c - left].attribute = 0x70;
                    }
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
            if !utf8 && projection.block_end_row == Some(document_row) {
                s.put(screen_row, 79, "K", 7);
            } else if !utf8 && projection.block_begin_row == Some(document_row) {
                s.put(screen_row, 79, "B", 7);
            }
        }
        s.cursor = Some((11 + row - top, column - left));
        if utf8 {
            s.put(
                23,
                0,
                " UTF-8 document. ^K S save; Ctrl-Shift-S Save As.",
                7,
            );
            s.put(
                24,
                0,
                "1Help                                   9Save      0Save & done",
                7,
            );
        } else {
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
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_keeps_the_dos_menu_and_replaces_only_the_work_area() {
        let mut opening = Screen::opening();
        let border = opening.cells[1][0].glyph;
        opening.opening_help();
        assert_eq!(opening.cells[0][0].glyph, 'J');
        assert_eq!(opening.cells[0][0].attribute, 0x70);
        assert_eq!(opening.cells[1][0].glyph, border);
        assert_eq!(opening.cells[13][0].glyph, ' ');
        assert_eq!(opening.cells[11][0].glyph, 'T');

        let mut editor = Screen::default();
        editor.put(1, 0, "MENU", 7);
        editor.put(11, 0, "DOCUMENT", 7);
        editor.put(24, 0, "STATUS", 7);
        editor.editor_help(false);
        assert_eq!(editor.cells[1][0].glyph, 'M');
        assert_eq!(editor.cells[11][0].glyph, ' ');
        assert_eq!(editor.cells[12][0].glyph, 'T');
        assert_eq!(editor.cells[24][0].glyph, 'S');
        assert!(editor.cursor.is_none());
    }

    #[test]
    fn dialogue_uses_the_same_reverse_video_status_bar() {
        let screen = Screen::dialogue("D");
        assert_eq!(screen.cells[0][0].glyph, 'D');
        assert!(screen.cells[0].iter().all(|cell| cell.attribute == 0x70));
    }
}
