# NorthStar

NorthStar is an open-source, non-commercial reconstruction of WordStar Professional 4.00 for distraction-free writing.

Its behavior is checked against binary analysis, controlled runs of the DOS original, and repeatable comparison tests.

## Current status

The repository contains a portable Rust editing core, a comparison CLI, and a runnable Windows desktop prototype. The desktop view reproduces the observed 80-column, 25-row DOS screen on black. Its initial legacy editing flow has been checked against a retained WordStar 4.0 session, including a file saved in NorthStar and reopened in the DOS original. A separate UTF-8 mode now supports a bounded modern writing flow. Many WordStar commands and author workflow features remain unimplemented.

## Build and try the prototype

Install Rust with rustup and, on Windows, the MSVC C++ build tools and Windows SDK. The repository pins Rust 1.99.0. From the repository root:

```text
cargo test --workspace --locked
cargo run -p northstar-cli -- inspect path/to/document.doc
cargo run -p northstar-cli -- replay tests/oracles/enter-insert-on-alpha-beta.json
cargo run -p northstar-cli -- replay tests/oracles/enter-insert-off-next-line.json
cargo run -p northstar-cli -- replay tests/oracles/reformat-old-markers.json
cargo run -p northstar-cli -- replay tests/oracles/edit-reformat-edit-split-chain.json
cargo run -p northstar-cli -- replay tests/oracles/wrap-type-multiple-wrapped-lines.json
cargo run -p northstar-desktop -- path/to/document.doc
```

Replay compares local Rust behavior with declared retained DOS observations. It does not execute the original program. Add `--output NEW_FILE` to write the scenario result to a new path; existing files are never overwritten. Inspection is read-only.

The desktop prototype accepts an optional WordStar document path. Without one, press D at the opening menu and enter a path. F2 starts a new UTF-8 document, and F3 opens a UTF-8 file; `--utf8 FILE` explicitly selects that mode from the command line. UTF-8 mode supports Norwegian text, paste, grapheme-aware left/right movement and deletion, Enter, and saving. In the legacy editor, Ctrl-S/Ctrl-D move the cursor, Ctrl-Q R returns to the start, Ctrl-V toggles insert mode, Ctrl-B reformats the current paragraph, and Enter splits a line. Ctrl-K S saves to the current path, Ctrl-K D saves and returns to the menu, and Ctrl-Shift-S saves a copy to a new path. Saving an existing file retains its previous bytes in a backup and refuses to overwrite a file changed outside NorthStar. F11 toggles full screen. F12 switches between the default IBM Plex Mono and an optional VGA font. [Windows prototype controls, verification, and limits](docs/WINDOWS_PROTOTYPE.md).

An unsigned, per-user [Windows MSI prototype](releases/windows/NorthStar-Prototype-0.1.3-x64.msi) is available. Its [build and test instructions](packaging/windows/README.md) describe what it installs and its current limits.

The 0.1.3 MSI predates the same-path save change described above. Build the current source to try it.

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

## Project documents

- [Architecture decision](docs/decisions/0001-rust-windows-first.md)
- [UTF-8 document decision](docs/decisions/0002-utf8-documents.md)
- [Delivery plan](docs/ROADMAP.md)
- [Fidelity and evidence rules](docs/FIDELITY.md)
- [Analysis provenance](docs/PROVENANCE.md)
- [Recorded source identities](docs/analysis-baseline.json)
- [Save behavior and open questions](docs/SAVING.md)

Original executables, dictionaries, proprietary resources, decompiled source, and the bulk investigation archive are not included in this repository.

The desktop prototype includes IBM Plex Mono and a recreated VGA font under their respective licenses. [Font attribution and licenses](THIRD_PARTY_FONTS.md).

## The name

NorthStar refers to the North Star, a guiding star, and to the King in the North in George R. R. Martin's Westeros.

## Dedication

Dedicated to George R. R. Martin and the world of *Game of Thrones*. WordStar 4.0's keyboard-driven, distraction-free writing environment is the model for NorthStar.

## License

NorthStar is licensed under the GNU General Public License, version 3 only (SPDX: `GPL-3.0-only`). See [LICENSE](LICENSE) for the complete terms. This applies to the project's original code and documentation, not to the original WordStar software or third-party material.

NorthStar is developed as a non-commercial project. The GPL nevertheless permits commercial use and redistribution under its terms; there is no additional non-commercial restriction.
