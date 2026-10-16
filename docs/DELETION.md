# WordStar character deletion

Ctrl-G deletes the character under the cursor in the bounded WordStar 4.00 document profile. It leaves the cursor at the same byte position and does not change Insert mode. On an ordinary hard CRLF, it removes both break bytes and joins two short lines. At the active document end it does nothing. Windows Ctrl-G selects the core command; the Backspace and Delete keys remain unqualified in legacy mode.

Ctrl-G does not fill the buffer used by Ctrl-U. [Ctrl-Y line erasure](ERASURE.md) and [Ctrl-T word erasure](WORD_ERASURE.md) have separate observed profiles.

## DOS observations

The reference is the primary floppy `WS.EXE`, SHA-256 `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`. On 2026-10-16, `runtime-northstar-del-01` opened five 128-byte synthetic documents in an isolated DOSBox copy. The retained action log, before and after screenshots, exact input carriers and saved output files are under `C:/Projects/rea/investigations/wordstar`. `python tools/verify-local-evidence.py C:/Projects/rea/investigations/wordstar` verifies the sources named by the five `delete-*.json` CLI cases.

| Case | Initial state and Ctrl-G result | Saved bytes |
|---|---|---|
| `delete-ordinary` | `ABCDE` at L01 C02 becomes `ACDE`, still C02 | `27f16abd4838201af57adb1c0878f4a6f75ad8f2262d8785c91864222dbe7f60` |
| `delete-join-hard-break` | `AB` CRLF `CD` at L01 C03 becomes `ABCD`, still C03 | `b51b9b2248bd8c0736934385ca4ed1c1ec788d04d6b15ccb89f8307b0e38a6b4` |
| `delete-at-eof` | `AB` at L01 C03 stays unchanged | `edb9a210d68647962c250949b13c4c16f55bb065428a0b5983458d405e8d8219` |
| `delete-high-bit-text` | Bytes `41 C2 43` at L01 C02 become `AC`, still C02 | `a4c62b281857a9b0f55e1944900cb9b3ebf2bad053aa27501a508eacf330888a` |
| `delete-insert-off` | `ABC` at L01 C02, Insert off, becomes `AC`, still C02 | `a4c62b281857a9b0f55e1944900cb9b3ebf2bad053aa27501a508eacf330888a` |

Each saved file contains 1A padding to 128 bytes. The five comparisons check saved SHA-256, final byte cursor, Insert mode and declared formatting. The displayed cursor and text were checked in the named DOS screenshots. The observed bytes are from the DOSBox copy, not decompiled predictions.

`runtime-northstar-del-02` opened the NorthStar-generated `delete-join-hard-break` output as `REJOIN.DOC`. WordStar displayed `ABCD` and saved it with the same SHA-256, `b51b9b2248bd8c0736934385ca4ed1c1ec788d04d6b15ccb89f8307b0e38a6b4`. This verifies reopen for that result, not all deletion cases.

## Boundary

The profile requires document mode, single spacing, margins 1/65, and right justification off. The core accepts printable one-byte text, including high-bit markers, on the selected line. A hard-break join also requires an ordinary next line and a combined length no greater than the tested right margin of 65. It rejects soft or page-marked break deletion, soft spaces, selected control cells, framed characters, dot-command lines, unresolved EOF payload, and unsupported formatting before changing state. Deletion on longer lines, Backspace, the DOS Delete key, and interaction with automatic reflow require separate DOS observations. [Ctrl-T word erasure](WORD_ERASURE.md) and [Ctrl-Y line erasure](ERASURE.md) have their own profiles. The profile is `ws4-delete-character-v1`; CLI replay accepts only `delete_character` under that profile.
