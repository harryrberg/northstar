# Ctrl-B reconstruction

The `ws4-ascii-reformat-v1` profile compares 27 synthetic cases against WordStar Professional 4.00, executable SHA-256 `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`. Fresh observations were collected on 2026-10-15 in isolated DOSBox copies. The redistributable cases are in `tests/oracles/reformat-*.json`; each pins its initial carrier and retained source identities.

## Implemented conditions

The caller declares document mode, single line spacing, left margin 1, right margin 65, and right justification off. The cursor must be at document start or directly after an ordinary hard CRLF. The selected paragraph may contain printable ASCII, highbit ASCII markers, A0 soft spaces, and 8D/0A soft breaks. Ordinary hard CRLF ends the paragraph. Other paragraphs retain their bytes, including supported structural tokens whose editing remains unknown.

Ctrl-B returns the cursor immediately after the hard CRLF, or at active EOF when there is no following hard paragraph. Insert mode is preserved. Initial offsets other than zero, Insert off, and an opaque following paragraph each have their own captured case. Cursor expectations are derived from the DOS text-mode status line and the saved token boundaries; they are not direct dumps of a document cursor pointer. NorthStar does not yet render or compare a complete screen.

## Reconstructed rules

1. Normalize existing highbit ASCII and remove old A0/soft-break markers. When removal would join two nonspace text runs, insert one ordinary space. A removed trailing break can likewise leave a separating space before hard CRLF or EOF. Never mask the bytes of an opaque framed character; unsupported tokens inside the selected paragraph are rejected.
2. Preserve ordinary spaces, including leading, repeated, and trailing spaces. Fit words by their last nonspace column. A word ending at column 65 fits; the separator after it may occupy column 66 and remains before the new 8D/0A.
3. Mark the byte immediately before an ordinary separator with its highbit. This also applies on the final unwrapped line. A final word without a separator remains unmarked.
4. Rebuild soft breaks, preserve the hard boundary and all other paragraphs, and move the cursor to the resulting paragraph end.
5. Serialize with the bounded record-padding rule described in the [compatibility profiles](COMPATIBILITY_PROFILE.md#carrier-serialization).

These rules combine static derivation and actual output comparisons. Relevant retained REA exports are EEF7 (`ev_41568d5f32cf4fccf95eb9df18f9419094e7efea0c77848090d3f35a7c89c5ba`), EF90 (`ev_9bb8448cbe55961c206a09fe64ffb5a190fbf37276debf8e08590bace4461a54`), and the scanner at 90BA/90CF (`ev_1fbc2d02526652769f086eee8bc95afd6dcae7b7a7a24c273ccb8d844357ad8c`). Runtime files are separate local observations, not REA Evidence records. Original/decompiled code is not included here.

## Compared scenarios

| Cases | Observed behavior |
|---|---|
| old-markers, plain-wrap | Existing formatted and plain text produce the same reconstructed paragraph. |
| short-line, punctuation, column-65 | Final-line marking, punctuation bytes and exact right-margin fit. |
| repeated-spaces, spaces-only | Leading, repeated and trailing ordinary spaces survive. |
| hard-boundary, second-paragraph, opaque-next-paragraph | Selected paragraph changes; surrounding paragraphs remain exact. |
| empty-document, empty-paragraph | Empty input and transition across a hard boundary. |
| missing-separator, soft-space-separator | Removed markers between words require a separator. |
| trailing-soft-break, trailing-spaced-break, trailing-break-before-hard | Trailing cleanup with and without an existing separator. |
| leading-markers, leading-soft-break, indented-soft-break | Initial removable markers and retained indentation. |
| insert-off | Ctrl-B preserves disabled Insert mode. |
| record-126, record-127, record-128, record-129 | Changed active lengths around the 128-byte record boundary. |
| spaces-record-127, trim-padding | Carrier size can change even when active text does not. |

All 27 cases compare the complete saved carrier by SHA-256, final cursor byte offset, Insert mode, and declared formatting. Eleven also repeat Ctrl-B after `document_start`, using a separate DOS run that opened actual Rust output and repeated Ctrl-B without changing its saved bytes. The harness restores the equivalent starting cursor; it does not emulate process restart. Source hashes can be checked with `python tools/verify-local-evidence.py INVESTIGATION_ROOT`.

Six further files from the final Rust release build were opened and saved in the original: the four record-boundary cases, 127 spaces, and trimmed padding. All retained exactly the Rust-generated carrier. This gives 17 reopen/save comparisons, of which eleven also repeated Ctrl-B. A separate menu capture confirms the default `turn right justify off` option changes to `turn right justify on` after Ctrl-O/J. The default document mode and single spacing remain recorded setup conditions rather than a complete internal-state dump.

The local capture directories are `runtime-northstar-ctrlb-01` through `-04` under the retained WordStar investigation. The local `verify-northstar-ctrlb.py` audit checks runtime identities, captured input/output, status-derived cursors, actual Rust output, reopen results and recorded process closure. The early unprefixed exploratory files in `-01` reset justification on file open; except OLD they were not justification-off cases and are excluded from this profile. The N-prefixed comparisons explicitly turn justification off for each open. Quick menu screenshots sometimes precede the delayed help display; the explicit menu-state proof is in `-04`. Long-word and hyphen exploratory outputs are retained but are not presented as implemented features.

## Limits

Hyphens, words longer than 65 columns, indentation that carries the first word beyond column 65, selected-paragraph tabs/styles/extended-character frames, page-marked breaks, malformed data, post-EOF payload, dot commands, other margins/modes/spacing, and arbitrary starting positions return unsupported errors. General original behavior for these inputs is not supplied by this profile. Leading soft breaks were tested directly because an initial static hypothesis suggested different behavior; the observed outputs support cleanup for the tested cases.

Structural navigation, insert-mode toggling, and bounded ASCII insert/overwrite and Enter work on formatted documents. The [editing profile](EDITING.md) includes sequences that reformat, edit, split and reformat again; the [wrapping profile](WRAPPING.md) adds automatic wrapping while typing with justification off. Full right justification, hyphen wrapping, visual selection, and complete DOS save semantics remain open. Windows tests and Linux CI exercise the Rust code; they do not establish a Windows GUI, screen fidelity, physical printing, or tablet support. No CPU, memory, or agent-token performance claim is made by these comparisons.
