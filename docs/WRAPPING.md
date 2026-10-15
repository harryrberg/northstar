# Automatic wrapping while typing

The `ws4-ascii-wrap-v1` profile reconstructs typing across the default right margin. It uses WordStar Professional 4.00, executable SHA-256 `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`, with document mode, word wrap enabled, CTLNFL=FF, single spacing, margins 1/65 and right justification off. Synthetic cases are retained in `tests/oracles/wrap-*.json`.

## Reconstructed rules

Typing is evaluated one character at a time. A character starting in columns 1–65 follows the existing [local editing rules](EDITING.md). An ordinary space beyond column 65 bypasses both wrapping and the preceding-character highbit update. A later nonspace can trigger normalization and wrapping before that character is inserted or overwrites existing text.

Automatic wrapping normalizes the current physical-line prefix, ending just before the cursor. Earlier physical lines and the suffix stay outside normalization. Existing ASCII highbits are cleared and A0 soft spaces removed, retaining a separating ordinary space when required. At a trailing removed run, the raw byte at the cursor matters: ordinary space suppresses the added separator; A0, a letter or EOF does not. This normalization is shared with Ctrl-B through one implementation, with each caller selecting its own range.

If normalization moves the cursor back within column 65, formatting stops there. Cleared prefix markers remain cleared. Otherwise, the formatter scans through the existing byte at the cursor, inclusively. It marks eligible predecessors of separators and inserts `8D 0A` after the last preceding space when a nonspace would exceed the margin. Leading indentation alone does not provide a break point; an overlong first word is not forcibly split. Ordinary repeated spaces remain intact.

EOF or an existing hard/soft CRLF at the cursor has a special boundary rule: a prefix width of exactly 65 can already cause the last word to move. An ordinary space at the cursor instead marks its predecessor and can end the bounded scan without a break. The rest of that word or line is not scanned. Existing highbits at and beyond the cursor survive normalization, except where typing subsequently overwrites their bytes.

Each inserted soft CRLF advances the saved cursor by two bytes. After formatting, the incoming character is inserted or overwrites at that relocated position. Overwrite retains its insertion fallback at CRLF/EOF. A batch stages all its characters and commits once; an unsupported later branch leaves the original bytes, cursor and modes unchanged.

## Evidence

The shared handler at 86EF–878C saves the cursor, locates the physical-line start, invokes prefix cleanup and conditionally invokes the formatter. 879D–87CF restores the adjusted cursor. Retained REA Evidence: `ev_a01fff6da6bf87618251d9a31a979cf05fceeaea849d3cf911051e57e50e190c`.

EEF7/EF90 establishes the exclusive cleanup boundary and raw following-byte test: `ev_41568d5f32cf4fccf95eb9df18f9419094e7efea0c77848090d3f35a7c89c5ba`. The scanner's inclusive bound at 9048–905C, EOF rule at 9067–9078, marker logic at 90BA and backtracking at 9216 onward are retained with `ev_1fbc2d02526652769f086eee8bc95afd6dcae7b7a7a24c273ccb8d844357ad8c`. Anchor adjustment is supported by `ev_40ebac88e2afd29b902dbe8476326553d7511c37bcd09d3781b1bebbf331ca6c`. Addresses here are original instruction offsets; Ghidra addresses add `0x10000`.

Executed scanner instructions are also retained in `runtime-justify-01/full-justify-logcpu.txt`; that older trace used justification on. The justification-off path and raw EF90 boundary comparisons have separate executed evidence in `runtime-reformat-01/reformat-logcpu.txt`. These sources support static reconstruction. Fresh saved-output comparisons establish the declared justification-off behavior, rather than treating an earlier trace with different settings as equivalent.

Runtime observations remain separate from REA Evidence. The public scenarios pin synthetic inputs, semantic commands and source hashes. Captured before/after DOS status lines establish cursor row/column and Insert mode. Byte offsets are derived from those positions and one-byte text cells on the selected line. Formatting fields are declared setup conditions; intermediate Rust states and complete screen rendering are not compared.

## Comparisons and reopening

All 44 new scenarios compare the complete saved carrier by SHA-256, final cursor byte offset, Insert mode and declared formatting. They bring the combined Enter, Ctrl-B, editing and wrapping suite to 119 scenarios.

| Group | Cases | Compared behavior |
|---|---:|---|
| Margin crossing, batching and spaces | 9 | Crossing column 65, repeated/beyond-margin spaces, deferred wrapping after a space, overwrite sequences and typing several wrapped lines. |
| Long words and indentation | 6 | No forced split of the first word; later separators establish a break point; leading spaces alone do not. |
| Physical-line boundaries and surrounding text | 12 | Insert/overwrite in a long line, EOF, hard/soft breaks, a second physical line, an already long prefix and opaque adjacent lines. |
| Existing markers and A0 cleanup | 10 | Cleared prefix markers, preserved or overwritten anchor markers, cleanup without wrapping, and different raw bytes after a removed run. |
| Ordinary letter/space under the cursor | 5 | The inclusive scan boundary in both Insert modes and across a two-character batch. |
| Combined editing flows | 2 | Wrapping followed by Enter and Ctrl-B, including overwrite-mode transitions. |

Twelve actual Rust release outputs were opened in the DOS original and saved with Ctrl-K/D. Every file retained its exact bytes. The selection covers ordinary end-of-line wrapping, an anchor space in both modes, a marked anchor, cleanup without wrapping, trailing A0 before ordinary/A0 space, an overlong first word, a second physical line, typing multiple wrapped lines and both combined flows. Reopening did not invoke Ctrl-B or edit the text.

The local observation directories are `runtime-northstar-wrap-01` and `-02`; final Rust outputs are in `northstar-replay-wrap-final`; reopening is recorded in `runtime-northstar-wrap-03`. The first run has 24 accepted cases and one excluded hyphen probe. The second has 20 accepted cases. Its shorter end-of-document navigation was first checked against a captured before-state, then used with independently checked before-states for every case.

The local `verify-northstar-wrap.py` audit checks executable identity, input/output/source hashes, fixture commands, captured before/after positions and Insert state, actual Rust output, all twelve reopens and recorded runtime closure. The repository's `tools/verify-local-evidence.py` verifies all 546 source identities named by the combined 119 scenarios. Original binaries, screenshots, execution traces and bulk observations stay outside the public repository.

## Limits

Nonspace typing beyond the margin with right justification enabled is unsupported. Selected-line tabs, styles, extended-character frames, page-marked breaks, malformed input, dot commands, post-EOF payload and alternate margins/modes/spacing remain outside the editing profile. Hyphen-related wrapping and creation of a dot-command line are refused. Ordinary local punctuation typing remains available where it does not enter those branches.

Full Ctrl-B still has its own limits, including rejection of overlong words and arbitrary starting positions. The wrapping profile does not extend those claims. Complete screen rendering, visual navigation/selection, general save semantics and the interactive Windows adapter remain unfinished. These comparisons do not establish full WordStar compatibility or performance characteristics.
