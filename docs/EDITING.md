# ASCII editing reconstruction

The `ws4-ascii-edit-v1` profile adds 46 synthetic comparisons against WordStar Professional 4.00, executable SHA-256 `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`. The observations were captured on 2026-10-15 in isolated DOSBox copies. The cases in `tests/oracles/edit-*.json` pin their initial bytes, semantic commands, expected results and retained source identities.

## Supported conditions

Editing requires document mode, single spacing, and margins 1/65. The reference executable has word wrap enabled and CTLNFL=FF. Right justification is turned off after each file open except the explicit justification-on typing case. Combined Ctrl-B sequences use its separate justification-off profile. Alternate newline configuration, special editing modes and automatic wrapping remain outside this reconstruction.

The selected physical line may contain printable ASCII, highbit ASCII markers and A0 soft spaces, followed by an ordinary hard or soft CRLF or EOF. Other lines are preserved without requiring their content to be editable. The shared document parser owns line boundaries and the active/padding boundary; editing does not rescan raw bytes as a second format parser.

## Reconstructed behavior

1. `insert_ascii` means typing printable ASCII. Insert on inserts bytes; Insert off replaces text cells. Overwrite stops replacing at the physical line end and inserts remaining characters before its intact CRLF, or extends EOF.
2. Typing an ordinary space marks the preceding nonspace byte with its highbit, including a preceding character within the same typed sequence. It leaves a preceding ordinary or A0 space unchanged. Other typed characters do not clean stale markers. Overwriting a marked character replaces its highbit as well as its low seven bits.
3. Each typed character must start in columns 1 through 65. This tests the cursor, not the resulting line length: typing near the beginning of a long line can extend it without reformatting. A batch that crosses the bound is rejected before any character or marker changes.
4. Enter with Insert on inserts hard CRLF at the cursor and leaves existing markers and breaks intact. Splitting immediately before soft CRLF therefore produces hard CRLF followed by that soft CRLF.
5. Enter with Insert off moves to the next physical line when a hard/soft CRLF exists. On a final unterminated line it moves to that line's end and appends hard CRLF, even when invoked from the middle. This also applies to an empty document.
6. Mutations use the [bounded carrier serialization rule](COMPATIBILITY_PROFILE.md#carrier-serialization). Cursor-only Enter preserves the complete carrier. A returned error commits no changes to bytes, cursor, Insert mode or formatting.

These rules combine static derivation and runtime comparisons. The shared typing/Enter handler export is `ev_a01fff6da6bf87618251d9a31a979cf05fceeaea849d3cf911051e57e50e190c`: 86A4/86AA tests the cursor column, and 87D1–87EF updates the preceding marker when a space is typed. The overwrite helper at 8D38 is retained in `ev_02ee313a74ccce278cf13f2109ffce90e0c4db5a405b8bd1ed01ba8d74480f92`; the insertion fallback E5FD/E615 is in `ev_2b2ebcb5c022c0136ecf788ac678e28edf57225faac29aee024ab576c4c44d67`. Mode helper E9E1 is `ev_955ef15bb01947f1f47b86afdace25a80a38acf6113a30af628c273181ba2dd4`. The Enter path at 8358 onward distinguishes movement across a break from appending at EOF. These are local REA Evidence identifiers; runtime captures are separate observations. Original or decompiled code is not redistributed here.

## Compared cases

| Cases | Coverage |
|---|---|
| insert-start, insert-before-marker, insert-after-marker, insert-second-space, insert-space-in-word, insert-word, insert-at-eof, insert-plain-word-split | Insertion around marked text, spaces, word interiors and EOF. |
| type-new-document, type-punctuation, append-after-stale-marker | New text, punctuation including hyphens, and retained stale markers. |
| overwrite-start, overwrite-marker, overwrite-separator, overwrite-space-in-word, overwrite-marker-with-space, overwrite-word, overwrite-at-eof, overwrite-through-eof | Replacement and space marking, followed by insertion at EOF. |
| insert-at-hard-break, overwrite-at-hard-break, insert-at-soft-break, overwrite-at-soft-break, overwrite-through-soft-break, overwrite-through-hard-break | Existing CRLF boundaries remain intact. |
| insert-at-soft-space, overwrite-soft-space, space-after-soft-space, overwrite-soft-space-word | A0 occupies one cell and retains space semantics. |
| enter-split-formatted-line, enter-off-hard-line, enter-off-eof, enter-split-soft-line, enter-off-final-middle, enter-off-soft-line, enter-before-soft-break, enter-off-empty | Insert-mode differences, final-line behavior and soft/hard breaks. |
| extend-long-line-at-start, type-column-65, space-column-65 | Cursor-based margin boundary and a resulting line longer than 65 columns. |
| type-with-justification-on | Local typing with right justification enabled. |
| preserve-opaque-next-line, edit-after-opaque-line | Opaque content outside the selected line remains byte-exact. |
| reformat-edit-split-chain | Reformat, insert, overwrite, split, then reformat again. |
| insert-reformat-wrapped-chain | Insert into a soft-wrapped paragraph and rebuild its wrapping with Ctrl-B. |
| create-overwrite-reformat-chain | Create two lines, overwrite each, use Enter with Insert off, then reformat. |

Every case compares the complete saved carrier by SHA-256, final cursor byte offset, Insert mode and declared formatting. Before/after DOS status lines independently establish the observed cursor row/column and Insert indicator. Cursor byte offsets are derived using the saved line boundaries and one-byte text cells on the selected line. Formatting fields are declared setup conditions, not a complete internal-state dump. Intermediate Rust snapshots and complete screen rendering are not compared against DOS.

## Save/reopen verification

Twelve files produced by the Rust release build were opened in the original and saved with Ctrl-K/D. All retained exactly the Rust-generated bytes. The set covers the three combined sequences, overwrite through EOF/hard/soft breaks, space after A0, Enter before a soft break, Enter from the middle of a final line, insertion near the start of a long line, space at column 65, and preservation of an opaque following line. No Ctrl-B was invoked on reopening: ordinary typing can legitimately leave stale markers that a later reformat would change.

Local evidence is retained under `runtime-northstar-edit-01` and `-02` for the 46 comparisons, `northstar-replay-edit-final` for actual Rust outputs, and `runtime-northstar-edit-03` for the twelve reopens. The local read-only `verify-northstar-edit.py` audit checks target identities, input/output hashes, fixture commands, before/after status-derived cursors and Insert modes, Rust output equality, reopen equality and recorded runtime closure. `python tools/verify-local-evidence.py INVESTIGATION_ROOT` checks the source identities named by all public scenarios. Bulk captures remain outside this repository.

## Limits and deliberate refusals

Typing beyond column 65 is refused. A separate retained `COL66SP` probe demonstrates that the original accepts a space at column 66 without marking the preceding character; that branch is explicitly unimplemented. Nonspace typing beyond the margin can invoke a different wrapping path. Applying full Ctrl-B after each keystroke would not reconstruct that path.

The selected line rejects tabs, styles, framed characters, page-marked breaks, malformed tokens and dot commands; post-EOF payload rejects mutation. A mutation that would create a dot-command line is also refused. Unsupported formatting and nonprintable/non-ASCII input are rejected atomically. These are implementation boundaries, not claims that the original lacks those features. Core refusal tests and real CLI tests check state preservation and that failed operations do not publish an output file.

Automatic wrapping, general right justification, visual navigation/selection, complete file-save semantics and the interactive Windows editor remain unfinished. This profile establishes bounded editing and file interoperability; it does not complete the first interactive milestone or qualify printing, Unicode, other clients, or performance.
