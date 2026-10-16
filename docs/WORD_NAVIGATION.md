# WordStar word navigation

The bounded WordStar 4.00 profile maps Ctrl-A to `previous_word` and Ctrl-F to `next_word`. The Rust core moves the cursor; the Windows adapter only selects the command. These commands leave document bytes, Insert mode, and the saved checkpoint unchanged.

On an ordinary physical line, a word begins at the first letter or digit after a separator and may contain an internal hyphen. Ctrl-F moves to the next word start on that line. Ctrl-A moves to the current word start from inside a word, or to the preceding word start when already at a start. A comma or one or more spaces separates words. From a measured space, Ctrl-F moves to the following word and Ctrl-A moves to the preceding word. Ctrl-F from a comma moves to the following word. If no destination exists on the line, the cursor stays where it is. The measured `ONE-TWO THREE` case treats `ONE-TWO` as one word for Ctrl-F. Other positions within these character classes follow the inferred rule until compared with DOS.

The high-bit word-end marker produced by ordinary typing does not stop movement. In measured cases, high-bit letters count as letters, including a synthetic marked letter inside a word. A marked comma or hyphen followed by an ordinary space, and a plain or marked period followed by an ordinary space, separate words. The core recognizes these cases without changing the original bytes.

At an ordinary hard CRLF, Ctrl-F moves from the break to the next line's first column. Ctrl-F from the only word before that break stays on its line. Ctrl-A at the beginning of the next line also stays there. This is line-local movement.

## DOS comparison

The reference is the primary floppy `WS.EXE`, SHA-256 `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`. Eighteen synthetic 128-byte documents were opened and saved in an isolated DOSBox copy on 2026-10-16. The retained `runtime-northstar-wordnav-01` directory under `C:/Projects/rea/investigations/wordstar` contains the exact inputs, action log, screenshots, and saved files. The saved bytes matched their initial SHA-256 identities in all 18 cases. The source files are not redistributed here.

Nine further cases in `runtime-openstar-wordnav-highbit-02` and `runtime-openstar-wordnav-punct-01` compare marked letters, marked comma and hyphen, and plain or marked periods before spaces. The screenshots establish the resulting L/C positions; all nine saved carriers match their starting SHA-256 identities. The `navword-*.json` scenarios compare those observations with the Rust result, including cursor, carrier bytes, Insert mode, and declared formatting. Together with the earlier cases, they cover repeated Ctrl-F, Ctrl-A at document start, word start, inside a word, on a space and at EOF, Ctrl-F on a space and at EOF, commas, an internal hyphen, leading and repeated spaces, and hard-break positions. Cursor byte offsets are derived from the DOS L/C display on one-byte text. `python tools/verify-local-evidence.py C:/Projects/rea/investigations/wordstar` verifies the retained source identities.

## Boundary

This profile qualifies ASCII letters and digits with or without high-bit markers, internal unmarked hyphens, spaces, commas, ordinary hard CRLF, and the measured separator cases above on the selected physical line. Other punctuation, marked punctuation outside these cases, soft spaces, tabs, dot-command lines, and opaque tokens are rejected before movement. Crossing soft or page-marked breaks is unqualified. The selected line is checked even when movement would leave the cursor in place. This profile makes no claim about other WordStar editions, multilingual legacy text, or UTF-8 mode. CLI replay accepts only `next_word` and `previous_word` under `ws4-word-navigation-v1`.
