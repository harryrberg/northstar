# Delivery plan

## Current state

Initial Rust implementation: byte-preserving document inspection, bounded ASCII insert/overwrite and Enter on formatted lines, automatic wrapping with justification off, and Ctrl-B at hard paragraph starts with default margins and right justification off. The CLI compares 257 scenarios with retained DOS observations: two original Enter cases, 27 Ctrl-B cases, 46 further editing cases, 44 wrapping cases, seven vertical-navigation cases, 18 word-navigation cases, 17 search cases, 12 search-and-replace cases, five Ctrl-G deletion cases, ten Backspace/Delete cases, eight block-copy cases, 20 block-move/delete cases, 14 Ctrl-Y/Ctrl-U erasure cases, and 27 Ctrl-T word-erasure cases. The Windows prototype renders the retained 80x25 screen, edits through actual keyboard input, saves the current file with a backup and external-byte comparison, and supports full screen with a smooth default font. A file produced by this flow reopened in the DOS original without byte changes. Three additional screen states from a longer document match retained DOS captures after core vertical-movement commands. A separate plain UTF-8 mode handles visual word wrap and vertical navigation without converting WordStar files. Both modes keep separate unsaved-draft recovery copies. Broader legacy wrapping, formatting, navigation beyond the declared profiles, and author workflow remain unfinished. The REA analysis package remains in its original local investigation directory. [Windows verification](WINDOWS_PROTOTYPE.md), [UTF-8 decision](decisions/0002-utf8-documents.md), [save behavior](SAVING.md), [current compatibility profiles](COMPATIBILITY_PROFILE.md), [Ctrl-B evidence](CTRL_B.md), [editing evidence](EDITING.md), [wrapping evidence](WRAPPING.md), [vertical navigation evidence](NAVIGATION.md), [word navigation evidence](WORD_NAVIGATION.md), [search evidence](FIND.md), [replacement evidence](REPLACEMENT.md), [Ctrl-G deletion evidence](DELETION.md), [Backspace/Delete evidence](BACKSPACE_DELETE.md), [block-copy evidence](BLOCK_COPY.md), [block move/delete evidence](BLOCK_MOVE_DELETE.md), [Ctrl-Y/Ctrl-U erasure evidence](ERASURE.md), and [Ctrl-T word-erasure evidence](WORD_ERASURE.md).

## M0: Reconstruction contract

- Define the compatibility profile for the primary WordStar Professional 4.00 floppy edition.
- Turn the verified Enter, mode, wrap, Ctrl-B, and cursor observations into named scenarios. List missing cursor/selection observations rather than inventing them.
- Specify document byte positions versus screen columns, hard/soft boundaries, EOF/padding, and preservation of unknown controls.
- Define oracle observation and comparison formats, including input identities and coverage limits.
- Trial a Rust Windows input/view adapter for Ctrl prefixes, key repeat, focus, and rendering. Iced 0.14 is selected for the Windows prototype. Norwegian text entry stays outside the legacy ASCII profile and follows the separate UTF-8 contract.

Exit: reviewable contracts and a bounded implementation plan with evidence references.

## M1: First complete editing flow

Open -> navigate -> insert/overwrite -> Backspace/Delete/Ctrl-G/Ctrl-T/Ctrl-Y/Ctrl-U -> mark/copy block -> Ctrl-B -> save -> reopen in original DOS WordStar.

Compare document bytes, cursor state, modes, and relevant display observations for every declared scenario. Unchanged files must retain their bytes, including unrecognized content. Use isolated output paths; originals remain immutable. Explain any normalization rather than hiding it.

Exit: a runnable Windows 11 build and a reproducible report of passing cases and known limitations. The narrow initial editing flow has passed the DOS reopen check; the broader milestone remains open. No full-compatibility claim.

## Windows distribution

An unsigned per-user MSI of the current prototype is checked in at [releases/windows/NorthStar-Prototype-0.1.19-x64.msi](../releases/windows/NorthStar-Prototype-0.1.19-x64.msi). It adds a Start menu entry and uninstall path, carries the project license and font notices, and does not move or remove user documents. The build and install/uninstall checks are in [packaging/windows](../packaging/windows/README.md). For a public Windows release, offer a portable ZIP built from the same executable, complete the author workflow, then decide signing and release hosting. MSI does not supply automatic updates; consider MSIX or Store distribution later if that channel becomes useful.

## Native UTF-8 writing

Keep UTF-8 documents separate from the WordStar carrier and qualify their own behavior. The current flow covers explicit mode selection, Norwegian text input, paste, grapheme-aware movement and deletion, visual word wrap, vertical navigation, bounded undo/redo, Save As, same-path saving with backup and external-byte comparison, and exact reopen. Next specify an explicit conversion policy and improve the author workflow. Both modes have separate local recovery snapshots, with distinct UTF-8 and byte-preserving WordStar records. Do not present modern behavior as DOS evidence.

## M2: Working author workflow

Further block operations, broader search/replace, margins, tabs, supported styles, help, deliberate quit/save semantics, and defined undo/recovery extensions. Qualify original behavior as each feature is reconstructed.

## M3: Modern output

Specify shared page layout and PDF generation, font metrics, supported legacy commands, and platform printing. Verify actual PDF and printer behavior; PCL trace compatibility alone does not establish physical output fidelity. Define useful manuscript export separately.

## M4: Additional clients

Port the same tested core to macOS and Linux, then tablet clients. Prototype Emacs state ownership and integration. Test e-paper on a selected real device, including key latency, refresh behavior, ghosting, suspend/resume, and safe storage. Each platform requires its own evidence before support is advertised.

## Later reconstruction

Expand messages/help, spelling, synonyms, and other original functions as required. Unknown message bytes, indirect code paths, and suggestion ranking remain research tasks, not assumed implementations.
