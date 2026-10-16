# WordStar literal search

The legacy editor maps Ctrl-Q F to the observed `Find what?` and `Option(s)?` prompts. Ctrl-L repeats the last search. The Rust core owns the query, options, cursor change, and success state. The Windows adapter collects the keys and displays the prompt or failure message. Search leaves the carrier bytes, Insert mode, and saved checkpoint unchanged.

The supported options are `W` for whole words, `U` for ignoring ASCII letter case, and `B` for searching backwards. They can be combined. A new forward search includes a match at the cursor; Ctrl-L starts one byte after the current match, which permits overlapping matches. A new backward search and its repetition exclude the current cursor position. The cursor moves to the start of a match. If a forward search fails, it moves to active EOF; if a backward search fails, it moves to document start. The first failure says `Could not find`; a failed Ctrl-L says `End of search for`. Escape dismisses the message.

The DOS run showed case-sensitive search by default. `U` matched `alpha` to `ALPHA`. On `BANANA`, searching `ANA` reached columns 2 and 4. `W` skipped `CAT` inside `CATALOG`; combined `W U` found `CAT`, `cat`, and `CAT` in order. These are observations at the named starting positions, not a general claim about all WordStar search text.

## Evidence

The reference executable is the primary floppy `WS.EXE`, SHA-256 `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`. On 2026-10-16, two synthetic 128-byte documents were opened in an isolated DOSBox copy. `FIND1.DOC` held `ALPHA BETA ALPHA` followed by CRLF and `BETA ALPHA`; `FIND2.DOC` held `BANANA CAT CATALOG cat` followed by CRLF and `CAT`. The retained `runtime-northstar-find-01` directory under `C:/Projects/rea/investigations/wordstar` contains the exact input hashes, action log, screenshots, and copied carriers. `FIND2.DOC` was explicitly saved after the search run, and both copied carriers match their starting bytes.

Seventeen `find-*.json` scenarios compare the final cursor, found/not-found state, carrier hash, Insert mode, and declared formatting. Cursor bytes are derived from the DOS line and column display on one-byte text. Each case identifies its screenshot and source hashes. `python tools/verify-local-evidence.py C:/Projects/rea/investigations/wordstar` verifies those local references. The original executable and screenshots are not distributed in this repository.

## Boundary

This profile searches literal printable ASCII of up to 64 bytes in documents containing only ordinary one-byte ASCII and hard CRLF before EOF padding. The query limit bounds the prototype UI and work; it is not a measured WordStar limit. The core rejects high-bit text, soft spaces or breaks, page markers, tabs, control cells, dot-command lines, and post-EOF payload before changing search state. The `G` and wildcard options are visible in the prompt but are not implemented; selecting them reports that limit. `W` currently treats ASCII letters, digits and underscore as word characters. Only the measured `CAT`/`CATALOG` boundary is qualified; punctuation and hyphen boundaries need separate comparison. Search over formatted or multilingual WordStar files, replacement, and UTF-8 search remain unqualified.

The CLI profile is `ws4-ascii-find-v1`. It accepts `find_literal`, `repeat_find`, `document_start`, and `next_character` for the observed starting states. `find_found` in the expected result distinguishes a hit from a boundary reached on failure.
