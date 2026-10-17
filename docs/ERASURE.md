# WordStar line erasure and unerase

Ctrl-Y deletes the current physical line into a reusable line buffer. It removes the line's hard CRLF when present, or just the text on the final line, then puts the cursor at the line start. Ctrl-U inserts a copy of that buffer at the current cursor without moving the cursor or consuming the buffer. It is not a general undo command: Ctrl-G character deletion does not fill this buffer.

## DOS observations

The reference is WordStar Professional 4.00 from the primary floppy edition, `WS.EXE` SHA-256 `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`. Runs `runtime-northstar-undo-01` through `-03` used isolated DOSBox copies on 2026-10-16. Each run retained exact synthetic input carriers, ordered key actions, before and after screenshots, and saved output files under `C:/Projects/rea/investigations/wordstar`. The 14 redistributable `erasure-*.json` cases name those sources and compare saved bytes, final cursor, Insert mode, and declared formatting. Run `python tools/verify-local-evidence.py C:/Projects/rea/investigations/wordstar` to check their source identities.

The cases establish these distinct behaviors:

| Case group | Observation |
|---|---|
| Ctrl-G then Ctrl-U | One or two character deletions remained deleted; Ctrl-U had no line buffer to insert. |
| Ctrl-Y on text | From the middle of `ABCDE`, Ctrl-Y removed the whole line and moved from C02 to C01. Ctrl-U restored its bytes at C01. |
| Ctrl-Y at a hard break | Deleting `AB` CRLF before `CD` left `CD` at L01 C01. Ctrl-U inserted `AB` CRLF and restored the two lines. |
| Empty line | A middle empty line lost its CRLF and was restored by Ctrl-U. Ctrl-Y on an empty final line had no effect. |
| Buffer lifetime | Typing `X`, deleting a character with Ctrl-G, and saving did not clear the Ctrl-Y buffer in the tested sequences. A new Ctrl-Y replaced it. |
| Repeated Ctrl-U | Each press inserted another copy, leaving the buffer available. |
| Cursor and Insert mode | Moving to C02 before Ctrl-U inserted the deleted line at C02 and kept the cursor there. With Insert off, Ctrl-U still inserted the line. |

The measured saved outputs include `XABCD` after Ctrl-Y, typing `X`, and Ctrl-U (SHA-256 `337ab10404f493ed55a19a97f4e5761997d565bb877a8606c23f516f37c96511`), and `AB` CRLF `AB` CRLF `CD` after two Ctrl-U commands (SHA-256 `06ddf525e72db4322e129d194a8a8c54420803111a8d01ec8ca38e3b1e462fa8`). The source cases identify the remaining outputs individually.

In `runtime-northstar-undo-04`, the NorthStar-generated `XABCD` carrier was opened as `REERASE.DOC` in WordStar. The DOS screen displayed `XABCD`, and saving there kept the same SHA-256. This is one interoperability check, not proof of every line-erasure path.

## Implementation boundary

The Rust core owns the shared erasure buffer. Saving the document does not clear it; opening a different document creates a new editor with an empty buffer. Ctrl-Y and Ctrl-U are supported on ordinary lines at or within the 65-column right margin, in document mode with single spacing and right justification off. This profile accepts hard CRLF and unmarked one-byte ASCII. The core rejects soft and page-marked breaks, high-bit markers, soft spaces, dot-command lines, opaque tokens, unsupported formatting, long lines, and insertion of a deleted line at a hard-break token before changing state. [Ctrl-T word erasure](WORD_ERASURE.md) and [Ctrl-Q Y line-suffix erasure](LINE_RIGHT.md) also use this buffer, with separately measured rules. Block erasure and its interaction with Ctrl-U remain unqualified. The CLI profile is `ws4-line-erasure-v1`.
