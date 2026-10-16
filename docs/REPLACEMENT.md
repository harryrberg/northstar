# WordStar search and replace

Ctrl-Q A asks for `Find what?`, then `Replace with?`, then `Option(s)?`. Without `N`, a found word produces `Replace Y/N`. `Y` replaces that one occurrence; `N` leaves it unchanged. Both return to the editor with the cursor at the start of the occurrence. Ctrl-L searches for the next occurrence and asks again. With `N` in the options, one occurrence is replaced without a question; Ctrl-L performs the next replacement. `W` restricts matches to whole words, and `U` ignores ASCII letter case in the measured sequence. A failed initial search says `Could not find`; a failed repeat says `End of search for`. After the message is dismissed, the cursor is at EOF.

## Evidence

The reference is the primary floppy WordStar Professional 4.00 `WS.EXE`, SHA-256 `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`. On 2026-10-16, isolated DOSBox runs `runtime-northstar-replace-03` through `-11` and `-13` used synthetic 128-byte carriers. Each run retains its starting byte identity, physical key actions, screenshots and saved file under `C:/Projects/rea/investigations/wordstar`. The 12 `replace-*.json` cases in `tests/oracles/` name and hash the relevant sources. `python tools/verify-local-evidence.py C:/Projects/rea/investigations/wordstar` checks their identities.

| Measured sequence | Saved active text | Cursor after the last command |
|---|---|---|
| `ALPHA BETA ALPHA`, replace first `ALPHA` with `OMEGA` after `Y` | `OMEGA BETA ALPHA` | byte 0 |
| `CAT CAT CAT`, replace `CAT` with `DOG` using option `N`, then Ctrl-L | `DOG DOG CAT` | byte 4 |
| `ALPHA BETA ALPHA`, replace both `ALPHA` occurrences with `X` | `X BETA X` | byte 7 |
| `ALPHA BETA ALPHA`, answer `N` at the first match and `Y` after Ctrl-L | `ALPHA BETA OMEGA` | byte 11 |
| `CAT CATALOG cat`, find `cat` with `W U`, answer `Y`, repeat and answer `Y` | `DOG CATALOG DOG` | byte 12 |
| `CAT DOG`, replace `CAT` with `WOLVES` | `WOLVES DOG` | byte 0 |
| `ALPHA` CRLF `ALPHA`, replace both with `OMEGA` | `OMEGA` CRLF `OMEGA` | byte 7 |
| `ALPHA BETA ALPHA`, start at byte 6 and replace the next `ALPHA` | `ALPHA BETA OMEGA` | byte 11 |

One case stops at the first `Replace Y/N` question and confirms that the file is unchanged until an answer. Another confirms that a missing `ZETA` leaves `ALPHA` unchanged and ends at byte 5. Repeating after replacing the only `ALPHA` reports the end of search and ends at byte 5. Full 128-byte carrier hashes, cursor and pending-question state are compared in the named cases.

A NorthStar-generated `OMEGA` CRLF `OMEGA` file was copied into `runtime-northstar-replace-reopen-12`, opened and saved by the DOS original. Its SHA-256 remained `819d30c025fb2b3683a41c7c0d03c4e94725dbfac713ba4aaa797bd51dc2cffb` before and after. This verifies byte interoperability for that file, not every replacement or save path.

## Boundary

The `ws4-ascii-replace-v1` profile accepts nonempty ASCII alphabetic find and replacement words of at most 64 bytes. It requires Insert on, document mode, single spacing, margins 1/65, right justification off, no active block marks, ordinary one-byte ASCII with hard CRLF, and a resulting affected physical line no longer than 65 bytes. The replacement happens at one occurrence per invocation; `N` suppresses the question, not the need for Ctrl-L to continue. The core owns the pending question and rejects unrelated editing commands until it is answered. Rejected operations leave bytes, cursor and search state unchanged.

The `B`, `G`, `?`, `A`, and `R` options, empty replacement, spaces, punctuation, high-bit or control cells, soft breaks, dot commands, marked blocks, Insert off, and automatic wrapping after replacement are not qualified. The UI permits Escape or Ctrl-U to decline a pending replacement safely; that cancellation shortcut is an OpenStar behavior, not a measured DOS equivalence. Search and replace in UTF-8 mode remains separate work.
