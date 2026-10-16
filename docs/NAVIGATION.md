# Vertical WordStar navigation

The bounded WordStar 4.00 profile supports Ctrl-E and Ctrl-X for one physical line up or down. The Rust core owns the cursor movement. Windows Up and Down arrows select the same commands as a convenience. Navigation does not change document bytes, Insert mode, or the saved checkpoint.

The destination keeps the current one-based column when it fits. Otherwise it lands just after the destination line's last text cell. That resulting column becomes the starting column for the next move. It does not remember the earlier, wider column. An empty line therefore lands in column 1. Movement at the first or last physical line leaves the cursor in place. Ordinary hard CRLF and soft 8D 0A breaks delimit lines without being split by the cursor.

## DOS observations

The reference is the primary floppy `WS.EXE`, SHA-256 `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`. Each run used an isolated DOSBox copy and synthetic files. The retained images, action logs, and exact input bytes are under `C:/Projects/rea/investigations/wordstar`; they are not redistributed here. `python tools/verify-local-evidence.py C:/Projects/rea/investigations/wordstar` checks the source identities recorded in the seven `nav-*.json` scenarios.

In `runtime-northstar-nav-01`, `NAV1.DOC` has SHA-256 `0370f3f91599a1b662bb66efb0a09fd5a95e94b1443fd382baddc831e9d99050`. From line 1, column 11, Ctrl-X reached the one-character line at L02 C02. Another Ctrl-X reached the long line at L03 C02. Two Ctrl-E commands returned through L02 C02 to L01 C02. Three retained 4,000-byte DOS screens at lines 12 through 14 also match OpenStar after actual `NextLine` commands, including scrolling.

In `runtime-northstar-nav-04`, the 128-byte input `ABCDE\r\nX\r\n\r\nLONG LINE HERE` plus 1A padding has SHA-256 `137240c5ba4d5e9be8147fa2ae135d39fc94ee215ba8e9ec2ee29f5ff6d4a2c3`. From L01 C05, consecutive Ctrl-X commands reached L02 C02, L03 C01, and L04 C01. Another Ctrl-X left L04 C01 unchanged. Ctrl-E then returned through L03 C01 and L02 C01 to L01 C01. The DOS session was stopped without a save; the input file retained its hash. The seven CLI scenarios compare final cursor, exact carrier bytes, Insert mode, and declared formatting. The desktop workflow also checks Ctrl keys, arrow keys, and exact reopen after Save As.

In `runtime-northstar-nav-03`, a separate 128-byte input with a soft break has SHA-256 `3506567d32feff6f02554179ee2204ff868fd305de473f24560f39579f4e50a6`. Ctrl-X from the start of `SOFT FIRST` reached L05 C01 at `SOFT SECOND LONG` across the 8D 0A break. The same file contained a later page-marked break. On a subsequent save, WordStar changed its 8A byte to 0A; that save is not a navigation comparison.

## Boundary

The supported current and destination lines contain one-byte text or soft spaces. Moving across a page-marked break, a dot-command line, or an opaque token in either selected line returns an explicit error without moving or changing the carrier. Other columns across soft breaks, horizontal scrolling on very long lines, page transitions, and special control-cell behavior need separate DOS comparisons. The CLI profile is `ws4-vertical-navigation-v1`; it accepts only the two vertical commands. UTF-8 mode keeps its separate visual-row movement policy.
