# OpenStar

OpenStar is an open-source, non-commercial reconstruction of WordStar Professional 4.00 for distraction-free writing.

Its behavior is checked against binary analysis, controlled runs of the DOS original, and repeatable comparison tests.

## Current status

The repository contains a portable Rust editing core, a comparison CLI, and a runnable Windows desktop prototype. The desktop view reproduces the observed 80-column, 25-row DOS screen on black. Its initial legacy editing flow has been checked against a retained WordStar 4.0 session, including a file saved in OpenStar and reopened in the DOS original. Ctrl-E/Ctrl-X move through ordinary physical lines, Ctrl-A/Ctrl-F move by word, Ctrl-Q F/Ctrl-L search plain ASCII and Ctrl-Q A replaces words under a separate DOS-checked profile, Backspace/Delete erase characters, and Ctrl-K B/K/C/V/Y mark, copy, move and delete blocks under separate DOS-checked profiles. A separate UTF-8 mode supports a bounded modern writing flow. Many WordStar commands and author workflow features remain unimplemented.

## Build and try the prototype

Install Rust with rustup and, on Windows, the MSVC C++ build tools and Windows SDK. The repository pins Rust 1.99.0. From the repository root:

```text
cargo test --workspace --locked
cargo run -p openstar-cli -- inspect path/to/document.doc
cargo run -p openstar-cli -- replay tests/oracles/enter-insert-on-alpha-beta.json
cargo run -p openstar-cli -- replay tests/oracles/enter-insert-off-next-line.json
cargo run -p openstar-cli -- replay tests/oracles/reformat-old-markers.json
cargo run -p openstar-cli -- replay tests/oracles/edit-reformat-edit-split-chain.json
cargo run -p openstar-cli -- replay tests/oracles/wrap-type-multiple-wrapped-lines.json
cargo run -p openstar-desktop -- path/to/document.doc
```

Replay compares local Rust behavior with declared retained DOS observations. It does not execute the original program. Add `--output NEW_FILE` to write the scenario result to a new path; existing files are never overwritten. Inspection is read-only.

The desktop prototype accepts an optional WordStar document path. Without one, press D at the opening menu and enter a path. F2 starts a new UTF-8 document, and F3 opens a UTF-8 file; `--utf8 FILE` explicitly selects that mode from the command line. UTF-8 mode supports Norwegian text, paste, visual word wrap, grapheme-aware movement and deletion, Enter, saving, undo/redo, and separate recovery copies of unsaved drafts. WordStar mode also keeps byte-preserving recovery copies of unsaved edits. F4 on the opening menu finds a recovery copy. In UTF-8 mode, Up/Down or Ctrl-E/Ctrl-X move through visual rows without changing the file's line endings. In the legacy editor, Ctrl-S/Ctrl-D move left and right, Ctrl-A/Ctrl-F move by word, Ctrl-E/Ctrl-X move between physical lines, Backspace deletes the previous character, Delete or Ctrl-G deletes the current character, Ctrl-T deletes a word, Ctrl-Y deletes a line, Ctrl-U inserts the last erased text, Ctrl-K B marks a block start, Ctrl-K K marks its end, Ctrl-K C copies it, Ctrl-K V moves it to the cursor, Ctrl-K Y deletes it, Ctrl-Q A starts bounded search and replace, Ctrl-Q R returns to the start, Ctrl-V toggles insert mode, Ctrl-B reformats the current paragraph, and Enter splits a line. Block move requires an ordinary ASCII source and destination on the same physical line, with the destination outside the marked range. Block delete requires the cursor at or after the end mark and can cross one ordinary hard CRLF. An end mark one byte before EOF is excluded. Ctrl-K S saves to the current path, Ctrl-K D saves and returns to the menu, and Ctrl-Shift-S saves a copy to a new path. Saving an existing file retains its previous bytes in a backup and refuses to overwrite a file changed outside OpenStar. F11 toggles full screen. F12 switches between the default IBM Plex Mono and an optional VGA font. [Windows prototype controls, verification, and limits](docs/WINDOWS_PROTOTYPE.md).

An unsigned, per-user [Windows MSI prototype](releases/windows/OpenStar-Prototype-0.1.28-x64.msi) is available. Its [build and test instructions](packaging/windows/README.md) describe what it installs and its current limits. [Menu coverage](docs/MENUS.md) lists commands still to implement.

In legacy mode, Ctrl-Q F asks for a search term and W, U, or B options. Ctrl-Q A asks for a word and its replacement; Y/N answers each match, while option N replaces without asking. Ctrl-L repeats either command. Wildcards, G, and broader replacement remain unqualified.

## Direction

- First release: a Rust application for Windows 11.
- Portable Rust core, independent of Windows and the GUI toolkit.
- Preserve the original editing workflow, command sequences, document semantics, and observable state where qualified by evidence.
- Serve authors and other people who value a quiet, keyboard-driven writing environment. Local operation and safe document storage are foundational.
- Later targets: macOS, Linux, iPadOS, Android, selected e-paper devices, and an Emacs integration.
- Modern printing and PDF output are planned extensions. Unicode and recovery behavior must have explicit contracts rather than silently changing legacy semantics.

## First milestone

Open a genuine WordStar 4.0 document, navigate using WordStar commands, edit in insert and overwrite modes, reformat with Ctrl-B, save, and reopen the result in the DOS original.

Completion requires passing declared differential cases for document bytes, cursor state, editor modes, and relevant screen output, with no unexplained differences. It does not imply complete program compatibility.

## Compatibility evidence

[Compatibility profile and limitations](docs/COMPATIBILITY_PROFILE.md).

Ctrl-B currently has 27 synthetic scenarios compared against the DOS original, covering saved bytes, cursor offsets, and declared modes. [Reconstructed rules, evidence, and remaining scope](docs/CTRL_B.md).

ASCII editing adds 46 comparisons, including typing in formatted text, overwrite at line breaks and EOF, Enter, and sequences combining editing with Ctrl-B. [Editing rules and evidence](docs/EDITING.md).

Automatic wrapping adds 44 comparisons with right justification off, including cursor-boundary behavior, spaces beyond the margin, marker cleanup and combined editing sequences. [Wrapping rules and evidence](docs/WRAPPING.md).

Vertical WordStar navigation adds seven comparisons covering short and empty lines, a soft break, and document boundaries. [Navigation rules and evidence](docs/NAVIGATION.md).

WordStar Ctrl-A/Ctrl-F navigation adds 27 comparisons covering word positions, separators, marked text, punctuation, hard breaks, and document boundaries. [Word navigation rules and evidence](docs/WORD_NAVIGATION.md).

Ctrl-Q F/Ctrl-L literal search adds 17 comparisons covering forward and backward hits, overlapping matches, word and case options, and failure positions. [Search rules and evidence](docs/FIND.md).

Ctrl-Q A search and replace adds 12 comparisons for Y/N answers, the N/W/U options, repeated replacement, length changes, and hard line breaks. [Replacement rules and evidence](docs/REPLACEMENT.md).

Ctrl-G adds five comparisons for ordinary text, a hard line break, EOF, high-bit text, and Insert off. [Deletion rules and evidence](docs/DELETION.md).

Ctrl-Y/Ctrl-U add 14 comparisons for line deletion, reusable unerase content, cursor placement, and interactions with typing, Ctrl-G, and Insert mode. [Line erasure rules and evidence](docs/ERASURE.md).

Ctrl-T adds 27 comparisons for word deletion and the shared Ctrl-U buffer. [Word erasure rules and evidence](docs/WORD_ERASURE.md).

Ctrl-K B/K/C add eight comparisons for block marking and copying. [Block copy rules and evidence](docs/BLOCK_COPY.md).

Ctrl-K V/Y add twenty comparisons for bounded block movement and deletion. [Block move/delete rules and evidence](docs/BLOCK_MOVE_DELETE.md).

## Project documents

- [Architecture decision](docs/decisions/0001-rust-windows-first.md)
- [UTF-8 document decision](docs/decisions/0002-utf8-documents.md)
- [Delivery plan](docs/ROADMAP.md)
- [Fidelity and evidence rules](docs/FIDELITY.md)
- [Analysis provenance](docs/PROVENANCE.md)
- [Recorded source identities](docs/analysis-baseline.json)
- [Save behavior and open questions](docs/SAVING.md)
- [WordStar vertical navigation](docs/NAVIGATION.md)
- [WordStar word navigation](docs/WORD_NAVIGATION.md)
- [WordStar literal search](docs/FIND.md)
- [WordStar search and replace](docs/REPLACEMENT.md)
- [WordStar character deletion](docs/DELETION.md)
- [WordStar Backspace and Delete](docs/BACKSPACE_DELETE.md)
- [WordStar block copy](docs/BLOCK_COPY.md)
- [WordStar block move and delete](docs/BLOCK_MOVE_DELETE.md)
- [WordStar line erasure](docs/ERASURE.md)
- [WordStar word erasure](docs/WORD_ERASURE.md)
- [Unsaved draft recovery](docs/RECOVERY.md)

Original executables, dictionaries, proprietary resources, decompiled source, and the bulk investigation archive are not included in this repository.

The desktop prototype includes IBM Plex Mono and a recreated VGA font under their respective licenses. [Font attribution and licenses](THIRD_PARTY_FONTS.md).

## Dedication

Dedicated to George R. R. Martin and the world of *Game of Thrones*. Martin [described](https://georgerrmartin.com/notablog/2007/05/18/) writing his fiction in WordStar 4.0 on a DOS machine separate from the computer he used for the internet. He preferred typed commands to icons and mouse gestures. OpenStar seeks to preserve that familiar, distraction-free way of writing on current hardware, without requiring a dedicated DOS computer.

## License

OpenStar is licensed under the GNU General Public License, version 3 only (SPDX: `GPL-3.0-only`). See [LICENSE](LICENSE) for the complete terms. This applies to the project's original code and documentation, not to the original WordStar software or third-party material.

OpenStar is developed as a non-commercial project. The GPL nevertheless permits commercial use and redistribution under its terms; there is no additional non-commercial restriction.
