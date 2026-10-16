# WordStar block move and delete

Ctrl-K V moves a marked block to the cursor. In the measured same-line cases, it leaves the cursor at the start of the moved text and retains the marks around it. Ctrl-K Y deletes the marked text, moves the cursor to its former start, and clears both marks. Marks affect the screen and editor state, not the saved file.

## DOS observations

The reference is the primary floppy `WS.EXE`, SHA-256 `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`. On 2026-10-16, isolated DOSBox runs `runtime-northstar-move-01`, `-02`, `runtime-northstar-block-boundary-04`, `-05`, and `-06` used synthetic 128-byte documents. Input manifests, physical key logs, screenshots, and saved carriers are retained under `C:/Projects/rea/investigations/wordstar`. The 20 `block-move-*.json` and `block-delete-*.json` cases in `tests/oracles/` identify and hash their sources. Run `python tools/verify-local-evidence.py C:/Projects/rea/investigations/wordstar` to verify those identities.

The first four cases cover moving a block to EOF, moving it after the source, copying after a move, and deleting on one line. The 16 additional cases cover moves before and after the source on the same line, moves in a document with a following line, a subsequent copy after moving before the source, deletion within a line, deletion through EOF, and deletion across one ordinary hard CRLF. Representative saved results are:

| Case | Starting text and action | Saved text |
|---|---|---|
| `block-move-before-source` | Move `CD` in `ABCDEF` to byte 0 | `CDABEF` |
| `block-move-after-interior` | Move `BC` in `ABCDEFGH` to byte 5 | `ADEBCFGH` |
| `block-move-before-then-copy` | Move `CD` to byte 0, then copy it there | `CDCDABEF` |
| `block-move-source-to-eof` | Move `CDE` in `ABCDE` to byte 0 | `CDEAB` |
| `block-delete-across-break` | Delete `B` CRLF `C` from `AB` CRLF `CDE` | `ADE` |
| `block-delete-across-break-following` | Same deletion with a following CRLF `FG` | `ADE` CRLF `FG` |
| `block-delete-through-break` | Delete `B` CRLF from `AB` CRLF `CD` | `ACD` |

The cursor and selection positions are derived from visible one-byte cells and, where applicable, a subsequent copy. After deletion, a further Ctrl-K C displayed “You have not yet defined a block. Use ^KB and ^KK.” A NorthStar-generated moved-block file was reopened as `REMOVE.DOC` in `runtime-northstar-move-reopen-03` and saved with the same 128-byte SHA-256. In `runtime-northstar-block-reopen-07`, the DOS original opened and saved NorthStar-generated `REMOVB.DOC` and `REDELC.DOC` without changing either file. Their SHA-256 values were respectively `238dae72cbb6a9bf9e026aa7c8b785493e319d4363c2c204a7fcd568a9cf0b02` and `356b9a0fcfdade85f51519838b9370405844fca3b17abbc0e03833d17a5a88` before and after.

An important exception remains: when the end mark is exactly one active byte before EOF, the DOS original also removes that final byte. For example, deleting `CD` from `ABCDE` yielded `AB`, and moving it to the start yielded `CDAB`; `E` was lost in both saved files. Deleting `B` CRLF `CD` from `AB` CRLF `CDE` yielded `A`. By contrast, an end mark at EOF and an end mark two bytes before EOF behaved as ordinary byte operations in the measured cases. The mechanism of the one-byte loss is unknown. OpenStar rejects that end-mark position before mutation; it does not silently produce the conventional byte result.

## Boundary

The `ws4-block-move-delete-v1` profile requires document mode, single spacing, margins 1/65, right justification off, Insert on, a nonempty selection, and ordinary ASCII on every affected physical line. A move requires source and destination on the same physical line, with the destination strictly outside the marked range. Deletion requires the cursor at or after the end mark and may cross at most one ordinary hard CRLF. Neither command accepts an end mark one active byte before EOF. Soft spaces, control and high-bit cells, dot commands, long affected lines, soft or page-marked breaks, and unresolved EOF payload are rejected. Ctrl-K V retains relocated marks; Ctrl-K Y clears them. Other editing commands still clear marks because their adjustment rules have not been measured. Cross-line movement, deletion across multiple breaks, deletion with the cursor before the end mark, Insert-off interactions, and Ctrl-U interactions remain unqualified.
