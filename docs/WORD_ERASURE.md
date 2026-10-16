# WordStar word erasure

Ctrl-T erases from the cursor to the next measured word boundary. Ctrl-U inserts a copy of the erased text at the current cursor without consuming it. Ctrl-T and Ctrl-Y replace the same buffer; Ctrl-G leaves it alone. This is a bounded reconstruction of WordStar Professional 4.00, not general undo.

## DOS observations

The reference is the primary floppy `WS.EXE`, SHA-256 `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`. Isolated DOSBox runs `runtime-northstar-word-01` through `-04` on 2026-10-16 retained synthetic input construction records, ordered key events, screen captures, and saved files under `C:/Projects/rea/investigations/wordstar`. The named source carriers were regenerated from the pre-action construction records and checked against their recorded hashes. The 27 redistributable `word-*.json` scenarios compare each declared saved SHA-256, cursor, Insert mode, and formatting. `python tools/verify-local-evidence.py C:/Projects/rea/investigations/wordstar` verifies their named source files.

| Position and sequence | Observed result |
|---|---|
| Start of `ONE TWO THREE` | Ctrl-T leaves ` TWO THREE`; the following space remains. |
| Middle of `ONE` | Ctrl-T removes the suffix from the cursor, leaving `ON TWO THREE`. |
| Start of `TWO` in `ONE TWO THREE` | Ctrl-T leaves `ONE THREE`. Ctrl-U restores `ONE TWO THREE`; the deleted interword space is part of the reusable content in this case. |
| On two spaces | Ctrl-T removes both. At the start of a line it removes leading spaces. |
| Before or on comma/period | A word before the punctuation is removed without the punctuation; at the punctuation, Ctrl-T removes that punctuation. A hyphen within `ONE-TWO` stays within the erased word. |
| On hard CRLF | Ctrl-T joins the two lines. Ctrl-U does not restore that break, and the prior Ctrl-Y buffer is cleared. |
| At active EOF | Ctrl-T leaves bytes and the prior buffer unchanged. |
| Buffer interactions | Repeated Ctrl-U inserts repeated copies. Ctrl-Y and Ctrl-T replace each other's saved content. Ctrl-G does not clear Ctrl-T content. Ctrl-T works with Insert off. |

One NorthStar-generated `ONE THREE` carrier from `word-second-word` was opened as `REWORD.DOC` in the DOS original and saved there with the same SHA-256, `d51a21a499f7825503704d4ad688f819e775fb6e7650f15ad5a8e99e3af8887f`. This checks that output's interoperability, not every Ctrl-T path.

## Implementation boundary

The core owns the shared Ctrl-T/Ctrl-Y buffer. The `ws4-word-erasure-v1` profile requires document mode, single spacing, margins 1/65, and right justification off. It accepts ordinary lines of at most 65 ASCII cells containing letters, digits, spaces, commas, periods, and hyphens, plus an ordinary hard CRLF where the joined line fits the margin. Digits are treated as word characters by inference; no digit-specific DOS comparison has been made. High-bit text, soft spaces, other punctuation, control cells, dot-command lines, soft or page-marked breaks, unsupported formatting, long lines, and unresolved EOF payload are rejected before mutation. The comparison cases cover only the recorded positions and sequences; wider word classification and reflow after deletion remain open.
