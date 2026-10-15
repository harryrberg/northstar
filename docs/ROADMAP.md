# Delivery plan

## Current state

Initial Rust implementation: byte-preserving document inspection, bounded ASCII/Enter editing, and Ctrl-B at hard paragraph starts with default margins and right justification off. The CLI compares two Enter and 27 Ctrl-B scenarios with retained DOS observations. Actual Rust outputs have also been reopened in the DOS original. This is a tested foundation, not a complete editor. The REA analysis package remains in its original local investigation directory. [Current compatibility profiles](COMPATIBILITY_PROFILE.md) and [Ctrl-B evidence](CTRL_B.md).

## M0: Reconstruction contract

- Define the compatibility profile for the primary WordStar Professional 4.00 floppy edition.
- Turn the verified Enter, mode, wrap, Ctrl-B, and cursor observations into named scenarios. List missing cursor/selection observations rather than inventing them.
- Specify document byte positions versus screen columns, hard/soft boundaries, EOF/padding, and preservation of unknown controls.
- Define oracle observation and comparison formats, including input identities and coverage limits.
- Trial a Rust Windows input/view adapter for Ctrl prefixes, key repeat, focus, rendering, and Norwegian text entry. Choose the GUI library from results.

Exit: reviewable contracts and a bounded implementation plan with evidence references.

## M1: First complete editing flow

Open -> navigate -> insert/overwrite -> Ctrl-B -> save -> reopen in original DOS WordStar.

Compare document bytes, cursor state, modes, and relevant display observations for every declared scenario. Unchanged files must retain their bytes, including unrecognized content. Use isolated output paths; originals remain immutable. Explain any normalization rather than hiding it.

Exit: a runnable Windows 11 build and a reproducible report of passing cases and known limitations. No full-compatibility claim.

## M2: Working author workflow

Block operations, search/replace, margins, tabs, supported styles, help, deliberate quit/save semantics, and defined undo/recovery extensions. Qualify original behavior as each feature is reconstructed.

## M3: Modern output

Specify shared page layout and PDF generation, font metrics, supported legacy commands, and platform printing. Verify actual PDF and printer behavior; PCL trace compatibility alone does not establish physical output fidelity. Define useful manuscript export separately.

## M4: Additional clients

Port the same tested core to macOS and Linux, then tablet clients. Prototype Emacs state ownership and integration. Test e-paper on a selected real device, including key latency, refresh behavior, ghosting, suspend/resume, and safe storage. Each platform requires its own evidence before support is advertised.

## Later reconstruction

Expand messages/help, spelling, synonyms, and other original functions as required. Unknown message bytes, indirect code paths, and suggestion ranking remain research tasks, not assumed implementations.
