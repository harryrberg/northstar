# NorthStar

NorthStar is an open-source, non-commercial reconstruction of WordStar Professional 4.00 for distraction-free writing.

The defining method is to connect reconstructed behavior to original binary analysis, controlled DOS observations, and repeatable differential tests.

## Direction

- First release: a Rust application for Windows 11.
- Portable Rust core, independent of Windows and the GUI toolkit.
- Preserve the original editing workflow, command sequences, document semantics, and observable state where qualified by evidence.
- Serve authors and other people who value a quiet, keyboard-driven writing environment. Local operation and safe document storage are foundational.
- Later targets: macOS, Linux, iPadOS, Android, selected e-paper devices, and an Emacs integration.
- Modern printing and PDF output are planned extensions. Unicode and recovery behavior must have explicit contracts rather than silently changing legacy semantics.

The GUI library and release schedule are not decided. The repository contains a portable Rust core and a CLI for structural inspection, ASCII insert/overwrite editing, automatic wrapping, Enter, and a bounded reconstruction of Ctrl-B. An interactive editor is not implemented yet.

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
```

Replay compares local Rust behavior with declared retained DOS observations. It does not execute the original program. Add `--output NEW_FILE` to write the scenario result to a new path; existing files are never overwritten. Inspection is read-only.

[Compatibility profile and limitations](docs/COMPATIBILITY_PROFILE.md).

Ctrl-B currently has 27 synthetic scenarios compared against the DOS original, covering saved bytes, cursor offsets, and declared modes. [Reconstructed rules, evidence, and remaining scope](docs/CTRL_B.md).

ASCII editing adds 46 comparisons, including typing in formatted text, overwrite at line breaks and EOF, Enter, and sequences combining editing with Ctrl-B. [Editing rules and evidence](docs/EDITING.md).

Automatic wrapping adds 44 comparisons with right justification off, including cursor-boundary behavior, spaces beyond the margin, marker cleanup and combined editing sequences. [Wrapping rules and evidence](docs/WRAPPING.md).

## First milestone

Open a genuine WordStar 4.0 document, navigate using WordStar commands, edit in insert and overwrite modes, reformat with Ctrl-B, save, and reopen the result in the DOS original.

Completion requires passing declared differential cases for document bytes, cursor state, editor modes, and relevant screen output, with no unexplained differences. It does not imply complete program compatibility.

## Project documents

- [Architecture decision](docs/decisions/0001-rust-windows-first.md)
- [Delivery plan](docs/ROADMAP.md)
- [Fidelity and evidence rules](docs/FIDELITY.md)
- [Analysis provenance](docs/PROVENANCE.md)
- [Recorded source identities](docs/analysis-baseline.json)

Original executables, dictionaries, proprietary resources, decompiled source, and the bulk investigation archive are not included in this repository.

## License

NorthStar is licensed under the GNU General Public License, version 3 only (SPDX: `GPL-3.0-only`). See [LICENSE](LICENSE) for the complete terms. This applies to the project's original code and documentation, not to the original WordStar software or third-party material.

NorthStar is developed as a non-commercial project. The GPL nevertheless permits commercial use and redistribution under its terms; there is no additional non-commercial restriction.
