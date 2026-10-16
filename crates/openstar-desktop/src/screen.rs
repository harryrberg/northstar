// SPDX-License-Identifier: GPL-3.0-only
//! Reconstructed 80x25 monochrome screen cells; no original font or resources.
use crate::directory::DirectoryListing;
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
    pub fn opening(directory_visible: bool) -> Self {
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
            if directory_visible {
                "     T table of contents                  F turn directory off"
            } else {
                "     T table of contents                  F turn directory on "
            },
            "     X exit OpenStar                   Esc shorthand",
            "     J help                               R run a DOS command",
        ]
        .iter()
        .enumerate()
        {
            s.put(row + 2, 1, line, 7);
        }
        s
    }

    pub fn show_directory(&mut self, listing: &DirectoryListing, header_row: usize, offset: usize) {
        let rows = 24usize.saturating_sub(header_row + 1);
        if rows == 0 {
            return;
        }
        let path = listing.path.display().to_string();
        self.put(
            header_row,
            0,
            &format!("DIRECTORY  {}", path)
                .chars()
                .take(80)
                .collect::<String>(),
            7,
        );
        let start = listing.page_start(offset, rows * 4);
        for (index, entry) in listing
            .entries
            .iter()
            .skip(start)
            .take(rows * 4)
            .enumerate()
        {
            let row = header_row + 1 + index % rows;
            let col = index / rows * 20;
            let size = if entry.is_directory {
                "   \\".to_owned()
            } else if let Some(bytes) = entry.size {
                let kib = bytes as f64 / 1024.0;
                let value = if kib < 10.0 {
                    format!("{kib:.1}k").trim_start_matches('0').to_owned()
                } else {
                    format!("{kib:.0}k")
                };
                format!("{value:>4}")
            } else {
                "   ?".to_owned()
            };
            let text = format!("{size} {}", entry.name);
            self.put(row, col, &text.chars().take(19).collect::<String>(), 7);
        }
        if let Some(error) = &listing.error {
            self.put(24, 0, &error.chars().take(80).collect::<String>(), 7);
        } else if listing.truncated || start + rows * 4 < listing.entries.len() || start > 0 {
            self.put(
                24,
                0,
                &format!(
                    "Directory entries {}-{} of {}{}; PgUp/PgDn browse",
                    start + 1,
                    (start + rows * 4).min(listing.entries.len()),
                    listing.entries.len(),
                    if listing.truncated { "+" } else { "" },
                ),
                7,
            );
        }
    }
    pub fn opening_help(&mut self) {
        *self = Self::dialogue("J");
        self.put(
            2,
            0,
            "To get help with a command on the Opening Menu, press its key.",
            7,
        );
        self.put(
            4,
            0,
            "While editing, press Ctrl-J or F1 for the Edit Menu help.",
            7,
        );
        self.put(
            6,
            0,
            "D opens a document. F toggles the directory; L changes its path.",
            7,
        );
        self.put(
            7,
            0,
            "J opens help; X exits. F2-F4 handle UTF-8 files and recovery.",
            7,
        );
        self.put(9, 0, "Press Esc to return to the Opening Menu.", 7);
        self.cursor = None;
    }
    pub fn opening_help_topic(&mut self, topic: char) {
        if topic == 'J' {
            self.opening_help();
            return;
        }
        *self = Self::dialogue(&format!("J{topic}"));
        let (label, description) = match topic {
            'D' => (
                "D open a document",
                "Opens an existing legacy document or creates one at the typed path.",
            ),
            'F' => (
                "F turn directory on/off",
                "Shows or hides the logged directory. Turning it on asks for a file match.",
            ),
            'L' => (
                "L change logged drive/directory",
                "Selects the folder used for relative document paths.",
            ),
            'X' => ("X exit OpenStar", "Closes OpenStar from the Opening Menu."),
            'N' => (
                "N open a nondocument",
                "This command is not implemented yet.",
            ),
            'P' => ("P print a file", "This command is not implemented yet."),
            'M' => (
                "M merge print a file",
                "This command is not implemented yet.",
            ),
            'I' => ("I index a document", "This command is not implemented yet."),
            'T' => (
                "T table of contents",
                "This command is not implemented yet.",
            ),
            'C' => ("C protect a file", "This command is not implemented yet."),
            'E' => (
                "E rename a file",
                "Renames a regular file on the same volume to a new name.",
            ),
            'O' => (
                "O copy a file",
                "Copies a regular file under a new name without replacing a file.",
            ),
            'Y' => (
                "Y delete a file",
                "Erases a selected regular file after a Y/N confirmation.",
            ),
            'R' => (
                "R run a DOS command",
                "This command is not implemented yet.",
            ),
            _ => ("Opening Menu", "This command is not available."),
        };
        self.put(2, 0, label, 7);
        self.put(4, 0, description, 7);
        self.put(
            7,
            0,
            "Press another Opening Menu key for help, or Esc to return.",
            7,
        );
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
    fn submenu(&mut self, prefix: char) {
        let title = match prefix {
            'K' => " B L O C K   &   S A V E   M E N U ",
            'Q' => " Q U I C K   M E N U ",
            'O' => " O N S C R E E N   F O R M A T   M E N U ",
            'P' => " P R I N T   C O N T R O L S   M E N U ",
            _ => return,
        };
        for row in 1..=9 {
            self.cells[row].fill(Cell {
                glyph: ' ',
                attribute: 7,
            });
        }
        self.border(1, 9, title);
        let entries: &[(usize, usize, &str)] = match prefix {
            'K' => &[
                (2, 13, "SAVE"),
                (2, 37, "BLOCK"),
                (2, 63, "FILE"),
                (3, 2, "S save & resume edit"),
                (3, 26, "B mark begin"),
                (3, 46, "C copy"),
                (3, 58, "O copy"),
                (3, 70, "P print"),
                (4, 2, "D save document"),
                (4, 26, "K mark end"),
                (4, 46, "V move"),
                (4, 58, "E rename"),
                (5, 2, "X save & exit OpenStar"),
                (5, 26, "H turn display on"),
                (5, 46, "Y delete"),
                (5, 58, "J erase"),
                (6, 2, "Q quit without saving"),
                (6, 26, "W write to disk"),
                (6, 46, "M math"),
                (6, 58, "L logged drive/dir"),
                (7, 10, "CURSOR"),
                (7, 26, "N turn column mode on"),
                (7, 58, "R insert a file"),
                (8, 2, "0-9 set/remove marker"),
                (8, 26, "I turn column replace on"),
                (8, 58, "F run a DOS command"),
            ],
            'Q' => &[
                (2, 11, "CURSOR"),
                (2, 39, "FIND"),
                (2, 54, "OTHER"),
                (2, 69, "SPELL"),
                (3, 2, "E upper left"),
                (3, 18, "P previous find"),
                (3, 35, "F find text"),
                (3, 48, "U align paragraphs"),
                (3, 66, "L check rest"),
                (4, 2, "X lower right"),
                (4, 18, "V last find"),
                (4, 35, "A find/replace"),
                (4, 48, "M math"),
                (4, 66, "N check word"),
                (5, 2, "S left side"),
                (5, 18, "B beg block"),
                (5, 35, "G char forward"),
                (5, 48, "ERASE"),
                (5, 66, "O enter word"),
                (6, 2, "D right side"),
                (6, 18, "K end block"),
                (6, 35, "H char back"),
                (6, 48, "Y line to right"),
                (7, 2, "R beg doc"),
                (7, 18, "0-9 marker"),
                (7, 35, "I find page"),
                (7, 48, "Del line to left"),
                (7, 66, "SCROLL"),
                (8, 2, "C end doc"),
                (8, 18, "? char count"),
                (8, 35, "(or line)"),
                (8, 48, "T to character"),
                (8, 66, "W up Z down"),
            ],
            'O' => &[
                (2, 12, "MARGINS"),
                (2, 37, "TYPING"),
                (2, 64, "DISPLAY"),
                (3, 2, "L set left"),
                (3, 26, "W turn word wrap off"),
                (3, 52, "D turn print controls off"),
                (4, 2, "R set right"),
                (4, 26, "J turn right justify off"),
                (4, 52, "H turn hyphen help on"),
                (5, 2, "X release"),
                (5, 26, "E enter soft hyphen"),
                (5, 52, "P turn preview on"),
                (6, 2, "T turn ruler off"),
                (6, 26, "G temporary indent"),
                (6, 52, "B turn soft space dots on"),
                (7, 2, "F ruler from text"),
                (7, 26, "X set line spacing"),
                (7, 68, "TABS"),
                (8, 2, "O ruler to text"),
                (8, 26, "C center line"),
                (8, 52, "I set tab stop"),
                (8, 68, "N clear"),
            ],
            'P' => &[
                (2, 10, "BEGIN & END"),
                (2, 55, "OTHER"),
                (3, 5, "B bold"),
                (3, 25, "X strike out"),
                (3, 42, "H overprint char"),
                (3, 61, "O binding space"),
                (4, 5, "S underline"),
                (4, 25, "D double strike"),
                (4, 42, "< overprint line"),
                (4, 61, "C print pause"),
                (5, 5, "U subscript"),
                (5, 25, "Y italics/color"),
                (5, 42, "F phantom space"),
                (5, 61, "I column tab"),
                (6, 5, "T superscript"),
                (6, 25, "K indexing"),
                (6, 42, "G phantom rubout"),
                (6, 61, "Q fixed position"),
                (7, 31, "Q W E R custom"),
                (7, 61, "N normal pitch"),
                (8, 42, "L form feed"),
                (8, 61, "A alternate pitch"),
            ],
            _ => unreachable!(),
        };
        for (row, column, label) in entries {
            self.put(
                *row,
                *column,
                &label.chars().take(78 - column).collect::<String>(),
                7,
            );
        }
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
        if !utf8 && let Some(prefix) = prefix {
            s.submenu(prefix);
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
    use crate::directory::DirectoryEntry;

    #[test]
    fn help_uses_the_observed_opening_screen_and_retains_the_editor_menu() {
        let mut opening = Screen::opening(true);
        opening.opening_help();
        assert_eq!(opening.cells[0][0].glyph, 'J');
        assert_eq!(opening.cells[0][0].attribute, 0x70);
        assert_eq!(opening.cells[1][0].glyph, ' ');
        assert_eq!(opening.cells[2][0].glyph, 'T');
        opening.opening_help_topic('F');
        assert_eq!(opening.cells[0][1].glyph, 'F');
        assert_eq!(opening.cells[2][0].glyph, 'F');

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

    #[test]
    fn opening_directory_uses_the_dos_grid_and_toggle_label() {
        let listing = DirectoryListing {
            path: "C:/WORK".into(),
            entries: vec![
                DirectoryEntry {
                    name: "..".into(),
                    size: None,
                    is_directory: true,
                },
                DirectoryEntry {
                    name: "CHAPTER.DOC".into(),
                    size: Some(128),
                    is_directory: false,
                },
            ],
            error: None,
            truncated: false,
        };
        let mut screen = Screen::opening(true);
        screen.show_directory(&listing, 11, 0);
        let row = |index: usize| {
            screen.cells[index]
                .iter()
                .map(|cell| cell.glyph)
                .collect::<String>()
        };
        assert!(row(7).contains("F turn directory off"));
        assert!(row(11).contains("DIRECTORY  C:/WORK"));
        assert!(row(13).contains(".1k CHAPTER.DOC"));
        assert!(
            Screen::opening(false).cells[7]
                .iter()
                .map(|cell| cell.glyph)
                .collect::<String>()
                .contains("F turn directory on")
        );
    }

    #[test]
    fn legacy_prefix_menus_replace_only_the_menu_area() {
        let mut screen = Screen::default();
        screen.put(0, 0, "STATUS", 0x70);
        screen.put(10, 0, "RULER", 7);
        screen.put(11, 0, "DOCUMENT", 7);
        for (prefix, title, label) in [
            ('K', "B L O C K   &   S A V E", "S save & resume edit"),
            ('Q', "Q U I C K", "F find text"),
            ('O', "O N S C R E E N", "L set left"),
            ('P', "P R I N T   C O N T R O L S", "B bold"),
        ] {
            screen.submenu(prefix);
            let menu = screen.cells[1..10]
                .iter()
                .flatten()
                .map(|cell| cell.glyph)
                .collect::<String>();
            assert!(menu.contains(title), "missing {prefix} menu title");
            assert!(menu.contains(label), "missing {prefix} menu command");
            assert_eq!(screen.cells[0][0].glyph, 'S');
            assert_eq!(screen.cells[10][0].glyph, 'R');
            assert_eq!(screen.cells[11][0].glyph, 'D');
            assert_eq!(screen.cells[8][78].glyph, '║');
        }
    }
}
