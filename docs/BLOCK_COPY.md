# WordStar block marks and copy

Ctrl-K B marks the start of a block and Ctrl-K K marks its exclusive end. Ctrl-K C inserts the marked bytes at the cursor, even with Insert off, and leaves the cursor in place. The source remains marked. Repeated copying at the same cursor copies the current marked range; inserting before or inside it moves the mark positions with the source text. The saved document contains the copied bytes, not the marks.

## DOS observations

The reference is the primary floppy `WS.EXE`, SHA-256 `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`. On 2026-10-16, isolated DOSBox runs `runtime-northstar-block-01`, `-02`, `-03` and `-04` opened synthetic 128-byte documents. Their input manifests, key logs, screenshots and saved carriers are retained under `C:/Projects/rea/investigations/wordstar`. Each `block-copy-*.json` case identifies and hashes its sources. Run `python tools/verify-local-evidence.py C:/Projects/rea/investigations/wordstar` to check them.

| Case | Measured result | Saved SHA-256 |
|---|---|---|
| `block-copy-end` | Mark `BCD` in `ABCDE FGHIJ`, copy at EOF: `ABCDE FGHIJBCD`; cursor stays at byte 11 | `11881b56abaa804b7ea7dcee2bcb39ac1b47fd5fd0832fc3ae9cda5186907624` |
| `block-copy-repeat-end` | Mark `BC` in `ABCD`, copy twice at EOF: `ABCDBCBC`; cursor stays at byte 4 | `3715ccee51e122f6c472f3003c38876018c8b1bca13917cb9650aca39f7a4dba` |
| `block-copy-hard-break` | Mark `B` CRLF `C` in `AB` CRLF `CD`, copy at EOF: `AB` CRLF `CDB` CRLF `C`; cursor stays at byte 6 | `93bb94c2e3ec719f68530a3470093a0d0040b444b893b776dfb92098df9dbc0f` |
| `block-copy-before-source` | Mark `CD` in `ABCDE`, copy at start: `CDABCDE`; cursor stays at byte 0 | `c72a2af3af78a7590b7be5c50c16b653ca3e5f884953ce9994be8db199f9a3d0` |
| `block-copy-inside-source` | Mark `BCD` in `ABCDE`, copy at byte 2: `ABBCDCDE`; cursor stays at byte 2 | `0f34088b40d64c2be0a266dbf1be1d03a4ba81c57131a7ad281d9f9a0dc62011` |
| `block-copy-insert-off` | Mark `BC` in `ABCDE`, Insert off, copy at EOF: `ABCDEBC`; cursor stays at byte 5 | `0c587253abf5ada1508abbe1cb02a31b10051902bd305ec103960a491b2ee127` |
| `block-copy-repeat-before-source` | Mark `CD` in `ABCDE`, copy twice at start: `CDCDABCDE`; cursor stays at byte 0 | `399a31f2f8e69bec31a222b771acf48bcd1fdb65922f01861c57348c6b3db63d` |
| `block-copy-repeat-inside-source` | Mark `BCD` in `ABCDE`, copy twice at byte 2: `ABBBCDCDBCDCDE`; cursor stays at byte 2 | `acebaf25366741b21de62298bcb9130795da737e2a6ae9aa3aa66228058f0e6a` |

The eight comparisons check saved SHA-256, cursor, Insert mode, declared formatting and the resulting mark offsets. Mark positions are derived from the inverse-video selection and from what a repeated copy copied. The DOS runs did not expose every internal editor field. In the first run, a zero-length marked block displayed “The end of the block is at or before the start of the block.” In the second, Ctrl-K C without both marks displayed “You have not yet defined a block. Use ^KB and ^KK.” The input files remained unchanged. These failures are retained as screenshots and covered by atomic core tests, not counted as successful replay cases.

NorthStar generated the `block-copy-hard-break` result as `target/block-reopen.doc`. In a separate `runtime-northstar-block-reopen-05` DOS copy, WordStar opened it as `REBLK.DOC`, displayed its three lines and saved it with the same SHA-256, `93bb94c2e3ec719f68530a3470093a0d0040b444b893b776dfb92098df9dbc0f`. This checks interoperability for that result.

OpenStar's selected cells use inverse video and show the K end marker at the right edge of its row, as seen in the retained DOS captures. It shows a B marker at the right edge of the start row. The original can show an inline `<B>` marker before an end is set; OpenStar's B marker occupies a different position.

## Boundary

The `ws4-block-copy-v1` profile requires document mode, single spacing, margins 1/65 and right justification off. The source and destination lines must contain ordinary one-byte ASCII and fit the right margin. The source may include ordinary hard CRLF. Soft or page-marked breaks, high-bit and control cells, soft spaces, dot-command lines, oversized output and unresolved EOF payload are rejected before mutation. The prototype has a 1 MiB document limit. The copied text and updated mark positions are owned by the core; the Windows adapter only translates Ctrl-K B, K and C and renders the selected state. Later unrelated text mutations clear the marks because their adjustment rules have not been measured. [Block movement and deletion](BLOCK_MOVE_DELETE.md) have their own narrower profile. Block hiding, formatting and interactions with Ctrl-U remain unqualified.
