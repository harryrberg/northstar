# OpenStar

OpenStar is an open-source reconstruction of WordStar Professional 4.00 for writers who want its keyboard commands and quiet DOS screen on a modern computer. The Windows 11 prototype has a black, 80-column, 25-row display, full-screen mode, and a font that scales smoothly on high-resolution screens.

The project checks its behavior against controlled runs of the DOS original and identified binary evidence. Its [compatibility profile](docs/COMPATIBILITY_PROFILE.md) records what has been verified and what remains unknown.

## Try it on Windows

[Download OpenStar 0.1.36 for Windows (MSI)](releases/windows/OpenStar-Prototype-0.1.36-x64.msi). The unsigned installer is per-user and adds a Start menu entry. [Installer details](packaging/windows/README.md).

At the opening menu:

- Press **D** to open or create a WordStar document.
- Press **F2** to start a UTF-8 document or **F3** to open one.
- Press **F4** to find an unsaved recovery copy.

Press **F11** for full screen and **F12** to switch between IBM Plex Mono and a recreated VGA font. See the [Windows guide](docs/WINDOWS_PROTOTYPE.md) for all implemented keys and their limits. [Font licenses](THIRD_PARTY_FONTS.md).

## What works today

- **WordStar mode:** bounded ASCII editing with cursor and word movement, hard returns, search and replace, block operations, paragraph reformatting, and saving. These behaviors are checked against specific DOS observations; support varies by command and document content.
- **UTF-8 mode:** a separate format for modern text input, paste, visual word wrap, cursor movement, and undo/redo. It does not convert WordStar documents.
- **File safety:** saving an existing document creates a backup and refuses to overwrite a file changed outside OpenStar. Both modes keep recovery copies of unsaved work.

Many WordStar 4.0 commands remain unfinished, including visible menu entries, broader formatting, spelling tools, and printing. WordStar mode currently accepts ASCII edits. macOS, Linux, tablets, e-paper devices, and Emacs are future targets. See [menu coverage](docs/MENUS.md) and the [delivery plan](docs/ROADMAP.md).

## Build from source

Install Rust with rustup. The repository pins version 1.99.0. On Windows, install the MSVC C++ build tools and Windows SDK. From the repository root:

```text
cargo run -p openstar-desktop
cargo run -p openstar-cli -- inspect path/to/document.doc
cargo run -p openstar-cli -- replay tests/oracles/hard-break-insert-on.json
cargo test --workspace --locked
```

`inspect` reads a file without editing it. `replay` compares OpenStar with a declared, retained DOS result; it does not start the original program. Add `--output NEW_FILE` to write a passing scenario to a new file.

## Reconstruction and contributing

The [fidelity contract](docs/FIDELITY.md) explains how a behavior becomes a compatibility claim. The [provenance record](docs/PROVENANCE.md) identifies the original software and local investigations; the [architecture decision](docs/decisions/0001-rust-windows-first.md) explains the portable Rust core and Windows-first approach. Original WordStar executables, dictionaries, proprietary resources, and decompiled source are not distributed here.

To report a difference from WordStar 4.0 or propose a feature, [open an issue](https://github.com/harryrberg/openstar/issues). For a compatibility change, include the starting document, modes, key sequence, and observed result as described in the fidelity contract.

## Dedication

OpenStar is dedicated to George R. R. Martin, author of *A Song of Ice and Fire*, the novels behind *Game of Thrones*. Martin [described](https://georgerrmartin.com/notablog/2007/05/18/) his writing setup in 2007:

> Windows sucks. I use it, under protest, for the internet, but all my fiction is written on a DOS machine, using WordStar 4.0. I’m a word guy. When I want to copy something, I like to type “copy.” When I want to delete it, I like to type “delete.” I don’t like puzzling out these stupid little cartoons they call icons, or dragging them around with a mouse. The day WordStar 4.0 stops working is the day I retire.

OpenStar aims to recreate the WordStar 4.0 writing experience on current computers, with its familiar commands and quiet screen, without requiring a dedicated DOS machine.

## License

OpenStar is licensed under the GNU General Public License, version 3 only (SPDX: `GPL-3.0-only`). See [LICENSE](LICENSE) for the complete terms. This applies to the project's original code and documentation, not to the original WordStar software or third-party material.

OpenStar is developed as a non-commercial project. The GPL nevertheless permits commercial use and redistribution under its terms; there is no additional non-commercial restriction.
