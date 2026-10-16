# WordStar Backspace and Delete

In the bounded WordStar Professional 4.00 profile, Backspace removes the byte before the cursor and moves the cursor back one byte. Delete removes the byte under the cursor and keeps the cursor in place, as Ctrl-G does in the measured cases. Backspace at document start and Delete at the active end leave the document and cursor unchanged. Both keys can remove an ordinary hard CRLF and join two short lines. Insert mode does not change these results.

## DOS observations

The reference is the primary floppy `WS.EXE`, SHA-256 `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`. On 2026-10-16, `runtime-northstar-backdel-01` opened ten synthetic 128-byte documents in an isolated DOSBox copy. The retained input manifest, action log, screenshots and saved outputs are under `C:/Projects/rea/investigations/wordstar`. Each `backdel-*.json` case names and hashes its sources. Run `python tools/verify-local-evidence.py C:/Projects/rea/investigations/wordstar` to check them.

| Case | Starting state and observed result | Saved SHA-256 |
|---|---|---|
| `backdel-backspace-middle` | `ABCDE`, cursor at byte 3: `ABDE`, cursor 2 | `20cc1a93a4f585d68d90921ab618bb6a2a7d4ada38509ab6df5317e2a448e832` |
| `backdel-backspace-start` | `AB`, cursor 0: unchanged | `edb9a210d68647962c250949b13c4c16f55bb065428a0b5983458d405e8d8219` |
| `backdel-backspace-join` | `AB` CRLF `CD`, cursor 4: `ABCD`, cursor 2 | `b51b9b2248bd8c0736934385ca4ed1c1ec788d04d6b15ccb89f8307b0e38a6b4` |
| `backdel-backspace-insert-off` | `ABC`, cursor 2, Insert off: `AC`, cursor 1 | `a4c62b281857a9b0f55e1944900cb9b3ebf2bad053aa27501a508eacf330888a` |
| `backdel-backspace-high-bit` | `41 C2 43`, cursor 2: `AC`, cursor 1 | `a4c62b281857a9b0f55e1944900cb9b3ebf2bad053aa27501a508eacf330888a` |
| `backdel-delete-middle` | `ABCDE`, cursor 3: `ABCE`, cursor 3 | `f48fb48831a948efd30b50f704a8759086a34de0c87d83fd91780d4522243759` |
| `backdel-delete-eof` | `AB`, cursor 2: unchanged | `edb9a210d68647962c250949b13c4c16f55bb065428a0b5983458d405e8d8219` |
| `backdel-delete-join` | `AB` CRLF `CD`, cursor 2: `ABCD`, cursor 2 | `b51b9b2248bd8c0736934385ca4ed1c1ec788d04d6b15ccb89f8307b0e38a6b4` |
| `backdel-delete-insert-off` | `ABC`, cursor 1, Insert off: `AC`, cursor 1 | `a4c62b281857a9b0f55e1944900cb9b3ebf2bad053aa27501a508eacf330888a` |
| `backdel-delete-high-bit` | `41 C2 43`, cursor 1: `AC`, cursor 1 | `a4c62b281857a9b0f55e1944900cb9b3ebf2bad053aa27501a508eacf330888a` |

The comparisons check saved bytes, cursor, Insert mode and declared formatting. Cursor byte positions are derived from DOS line and column status for these one-byte cells. The DOS run did not expose all internal formatting state. All saved files use 1A padding. NorthStar generated `backdel-backspace-middle` as `target/backdel-reopen.doc`; `runtime-northstar-backdel-reopen-01` opened it as `REBACK.DOC` in the original, displayed `ABDE`, and saved it with the same SHA-256, `20cc1a93a4f585d68d90921ab618bb6a2a7d4ada38509ab6df5317e2a448e832`. This checks interoperability for one result.

## Boundary

The profile `ws4-backspace-delete-v1` requires document mode, single spacing, margins 1/65 and right justification off. The selected line must fit within 65 bytes and contain qualified one-byte text. A hard-break join requires an ordinary following line and a combined length within 65 bytes. The core rejects soft or page-marked breaks, soft spaces, selected control cells, framed characters, dot-command lines, unresolved EOF payload and unsupported formatting before changing state. Long lines, automatic reflow, interaction with Ctrl-U and other unmeasured contexts remain unqualified. The desktop maps physical Backspace to a separate core command and physical Delete to the same deletion command as Ctrl-G. CLI replay accepts only these two commands under this profile.
