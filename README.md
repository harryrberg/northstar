# NorthStar

NorthStar is an open-source, non-commercial reconstruction of WordStar Professional 4.00 for distraction-free writing.

Its behavior is checked against binary analysis, controlled runs of the DOS original, and repeatable comparison tests.

## Current status

The repository contains a portable Rust editing core, a comparison CLI, and a runnable Windows desktop prototype. The desktop view reproduces the observed 80-column, 25-row DOS screen on black. Its initial editing flow has been checked against a retained WordStar 4.0 session, including a file saved in NorthStar and reopened in the DOS original. This is a bounded prototype; many WordStar commands remain unimplemented.

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

The desktop prototype accepts an optional document path. Without one, press D at the opening menu and enter a path. In the editor, Ctrl-S/Ctrl-D move the cursor, Ctrl-Q R returns to the start, Ctrl-V toggles insert mode, Ctrl-B reformats the current paragraph, and Enter splits a line. Ctrl-K S saves to a **new** path; existing files cannot be overwritten. F11 toggles full screen. F12 switches between the default IBM Plex Mono and an optional VGA font. [Windows prototype controls, verification, and limits](docs/WINDOWS_PROTOTYPE.md).

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
- [Delivery plan](docs/ROADMAP.md)
- [Fidelity and evidence rules](docs/FIDELITY.md)
- [Analysis provenance](docs/PROVENANCE.md)
- [Recorded source identities](docs/analysis-baseline.json)

Original executables, dictionaries, proprietary resources, decompiled source, and the bulk investigation archive are not included in this repository.

The desktop prototype includes IBM Plex Mono and a recreated VGA font under their respective licenses. [Font attribution and licenses](THIRD_PARTY_FONTS.md).

## The name

NorthStar refers to the North Star, a guiding star, and to the King in the North in George R. R. Martin's Westeros.

## Dedication

Dedicated to George R. R. Martin and the world of *Game of Thrones*. WordStar 4.0's keyboard-driven, distraction-free writing environment is the model for NorthStar.

## License

NorthStar is licensed under the GNU General Public License, version 3 only (SPDX: `GPL-3.0-only`). See [LICENSE](LICENSE) for the complete terms. This applies to the project's original code and documentation, not to the original WordStar software or third-party material.

NorthStar is developed as a non-commercial project. The GPL nevertheless permits commercial use and redistribution under its terms; there is no additional non-commercial restriction.
