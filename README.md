# NorthStar

NorthStar is an open-source, non-commercial reconstruction of WordStar Professional 4.00 for distraction-free writing.

The defining method is to connect reconstructed behavior to original binary analysis, controlled DOS observations, and repeatable differential tests. Familiar key bindings alone do not establish compatibility. This project does not claim that the method is unique among all existing projects.

## Direction

- First release: a Rust application for Windows 11.
- Portable Rust core, independent of Windows and the GUI toolkit.
- Preserve the original editing workflow, command sequences, document semantics, and observable state where qualified by evidence.
- Serve authors and other people who value a quiet, keyboard-driven writing environment. Local operation and safe document storage are foundational.
- Later targets: macOS, Linux, iPadOS, Android, selected e-paper devices, and an Emacs integration.
- Modern printing and PDF output are planned extensions. Unicode and recovery behavior must have explicit contracts rather than silently changing legacy semantics.

The product name is NorthStar; the repository name is `northstar`. The GUI library and release schedule are not decided. This repository currently records direction and delivery criteria; it contains no working editor yet.

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

Copyright (C) 2026 Harry Berg.

NorthStar is licensed under the GNU General Public License, version 3 only (SPDX: `GPL-3.0-only`). See [LICENSE](LICENSE) for the complete terms. This applies to the project's original code and documentation, not to the original WordStar software or third-party material.

NorthStar is developed as a non-commercial project. The GPL nevertheless permits commercial use and redistribution under its terms; there is no additional non-commercial restriction.
