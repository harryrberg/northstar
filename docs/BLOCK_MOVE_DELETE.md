# WordStar block move and delete

Ctrl-K V moves a marked block to the cursor. In the measured one-line documents with the cursor after the block, the cursor lands at the start of the moved text and the block remains selected at its new position. Ctrl-K Y deletes the marked text, moves the cursor to the former block start, and clears both marks. The saved file contains the resulting text, not the marks.

## DOS observations

The reference is the primary floppy `WS.EXE`, SHA-256 `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`. On 2026-10-16, isolated DOSBox runs `runtime-northstar-move-01` and `-02` used synthetic 128-byte documents. Input manifests, physical key logs, screenshots and saved carriers are retained under `C:/Projects/rea/investigations/wordstar`. Each `block-move-*.json` or `block-delete-*.json` case names and hashes its sources. `python tools/verify-local-evidence.py C:/Projects/rea/investigations/wordstar` checks them.

| Case | Measured result | Saved SHA-256 |
|---|---|---|
| `block-move-end` | Move `BCD` in `ABCDE FGHIJ` to EOF: `AE FGHIJBCD`; cursor and new block start at byte 8 | `2ad6dea2c647fa465644835d59b23baf21bc8d0a071e2789a324e23a2db90b61` |
| `block-move-inline` | Move `BC` in `ABCDE` to the original byte 5: `ADEBC`; cursor and new block start at byte 3 | `aff69b3d65e335e236f9845e29730b63a42911980c6371119aaccc5084b0b293` |
| `block-move-then-copy` | Copy at the moved block start after the first case: `AE FGHIJBCDBCD`; cursor stays at byte 8 and the original moved block is still selected | `8140c9292c27fe0023716a18659acb105828b827ed43c8ae084313b11cffc437` |
| `block-delete-inline` | Delete `BC` in `ABCDE` with cursor after the block: `ADE`; cursor at byte 1 and marks cleared | `aa4a371b0b722edd09ac05a2ded980cc5a1ff23dff93905c4bc046bc14c959a2` |

The cursor and selection positions come from visible one-byte cells and the subsequent copy or error screen, not from internal editor state. After deletion, a further Ctrl-K C displayed “You have not yet defined a block. Use ^KB and ^KK.” A NorthStar-generated `block-move-end` file was opened as `REMOVE.DOC` in the separate `runtime-northstar-move-reopen-03` DOS copy and saved with the same SHA-256, `2ad6dea2c647fa465644835d59b23baf21bc8d0a071e2789a324e23a2db90b61`.

Two other observations prevent a broader claim. Moving `CD` from `ABCDE` to the start produced `CDAB`, with `E` absent from the saved file. Deleting a selection spanning `B` CRLF `C` from `AB` CRLF `CD` produced `A`, not the simple byte deletion result. The action logs, `m2-after-move.png` and `d2-after-delete.png` in `runtime-northstar-move-01` retain these results. Their rules and interactions have not been resolved, so NorthStar rejects those contexts before changing bytes or marks.

## Boundary

The `ws4-block-move-delete-v1` profile requires document mode, single spacing, margins 1/65, right justification off, Insert on, a one-line ordinary ASCII document, a nonempty selection, and a cursor after it. Move requires the cursor at document end. Soft spaces, control and high-bit cells, dot commands, long lines and unresolved EOF payload are rejected. Ctrl-K V retains the relocated marks; Ctrl-K Y clears them. Other editing commands still clear marks because their adjustment rules have not been measured. Move to any position other than document end, movement or deletion in a multiline document, deletion with the cursor before the block, Insert-off interactions, and Ctrl-U interactions remain unqualified.
