# Bounded compatibility profiles

This is a bounded prototype, not a complete WordStar editor. The reference is the primary floppy WS.EXE whose SHA-256 is `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`.

## State and ownership

The Rust core owns exact document carrier bytes, a zero-based cursor byte offset, insert mode, and declared formatting (margins, right justification, line spacing, document mode). It keeps the original bytes for byte-based modification detection; this does not claim to reproduce the DOS modified flag. UI and CLI adapters do not reinterpret command semantics. Cursor offsets cannot split a framed character or a CR/LF pair, or move into padding. No universal screen-column or Unicode-index equivalence is assumed.

Parsing partitions every input byte without rewriting it. It distinguishes hard/soft CR paired with ordinary/page-marked LF, 1B/9B framing, high-bit text/control values, soft spaces, tabs, padding candidates, and unresolved input. These are structural interpretations from the local investigation. Recognizing a structure does not establish its rendering or editing behavior.

The first padding token is the bounded editing profile's active end. The entire carrier, including material after that token, is retained. Text mutations reject post-EOF payload. Typing and Enter validate the selected physical line; Ctrl-B validates its selected paragraph. Other active content retains its bytes. Structural navigation and mode toggling do not require text to be editable. Inspection remains available.

## Enter: ws4-ascii-enter-v1

Two synthetic, redistributable JSON scenarios refer to hashed retained DOS evidence:

| Scenario | Reference observation | Compared result |
|---|---|---|
| enter-insert-on-alpha-beta | New document, Insert on, CTLNFL=FF: ALPHA / Enter / BETA / save | All 128 saved bytes via SHA-256; insert mode |
| enter-insert-off-next-line | ALPHA CRLF BETA, start at L01 C01, Insert off, CTLNFL=FF, Enter / save | Unchanged 128 saved bytes; insert mode; byte offset 7 derived from observed L02 C01 |

The insert-on scenario's empty initial carrier is a synthetic representation of a new document, not a captured DOS memory buffer. Final cursor and screen are not compared there. Replay uses semantic commands and restores declared initial state; it does not execute DOS menus, key events, or the original program. Source references are verified separately by `tools/verify-local-evidence.py` against an explicitly selected local investigation directory.

The expected carrier is constructed as ASCII `ALPHA`, `0D 0A`, ASCII `BETA`, and 117 `1A` bytes, then checked against the retained captures. Original software, screenshots, dictionaries, and decompiled code are not included.

## ASCII editing: ws4-ascii-edit-v1

Forty-six additional scenarios qualify typing and Enter in plain or formatted text, including three sequences combining these operations with Ctrl-B. Each compares the saved carrier, final cursor, Insert mode, and declared formatting. The profile requires explicit initial formatting: document mode, single spacing and margins 1/65. Right justification may be on or off for local typing and Enter; Ctrl-B retains its separate justification-off restriction. The reference setup has word wrap enabled and CTLNFL=FF.

`insert_ascii` represents typed printable ASCII and obeys Insert mode. With Insert off it replaces text cells, then inserts any remaining characters before an intact hard/soft CRLF or at EOF. A typed space marks the preceding nonspace byte; ordinary typing preserves existing markers except where overwritten. The selected physical line may contain ASCII with highbit markers and A0 soft spaces. Other lines may contain opaque data and remain exact.

The local typing branch applies to characters starting in columns 1 through 65. This tests the cursor rather than the resulting line length: insertion near the start can extend an already long line. Beyond-margin spaces and automatic wrapping are now implemented under the additional wrapping profile below. A later unsupported branch rejects the entire typed command atomically.

Enter with Insert on inserts a hard CRLF at the cursor, preserving any existing break. With Insert off it moves past the next hard/soft CRLF without changing bytes; on a final unterminated line it appends CRLF at that line's end, including when invoked from the middle. Selected-line controls, tabs, framed characters, page-marked breaks, malformed data, and dot-command lines are unsupported. Errors leave bytes, cursor and modes unchanged. [Exact rules, comparisons and reopen evidence](EDITING.md).

Token-boundary movement, document-start navigation, and insert-mode toggling support replay. Their use in the declared sequences is compared; arbitrary visual navigation is not qualified.

## Ctrl-B: ws4-ascii-reformat-v1

Reformat scenarios require explicit initial formatting. The supported settings are document mode, single spacing, margins 1/65, and right justification off. Ctrl-B starts at document start or immediately after an ordinary hard CRLF; it rebuilds one paragraph and moves to the next hard paragraph or EOF. The current implementation handles printable ASCII, existing highbit markers, soft spaces and ordinary soft breaks. [The 27 comparisons and exact exclusions](CTRL_B.md) define the evidence boundary.

Formatting settings remain explicit in snapshots and can be compared by replay. Restoring state is a harness operation, not evidence that the original uses the same internal representation. Unsupported settings are retained, then rejected by affected mutations before changing state.

## Automatic wrapping: ws4-ascii-wrap-v1

Forty-four additional scenarios compare typing across the right margin with right justification off. An ordinary space beyond column 65 is accepted without marking its predecessor. A nonspace first normalizes the current physical-line prefix, preserving the existing cursor byte and suffix. If the cursor remains beyond the margin, a bounded scanner places soft CRLFs and relocates it before ordinary insert/overwrite takes place. This is a distinct operation from full-paragraph Ctrl-B.

The scanner includes the existing byte under the cursor, so a space, letter or EOF there can produce different wrapping. Long first words are retained without forced splitting. Cleanup can also reduce the column enough to avoid wrapping. Original before/after states qualify these rules in Insert on/off cases, with preserved adjacent physical lines, and in two sequences combining wrapping, Enter, overwrite and Ctrl-B. [Exact rules, source evidence and reopen comparisons](WRAPPING.md).

Nonspace typing beyond the margin with right justification on, hyphen wrapping and the existing unsupported document/formatting conditions return explicit errors. Every typed batch stages its mutations and commits once.

## Vertical navigation: ws4-vertical-navigation-v1

Seven scenarios compare Ctrl-E/Ctrl-X movement with retained DOS status and synthetic input identities. On ordinary one-cell physical lines, the current column is clamped to the destination's end. The clamped column is used by the next move; there is no retained preferred column. Empty lines, the first and last line, and one observed soft-break transition are included. The core moves without changing carrier bytes or Insert mode. The Windows adapter routes Ctrl-E/Ctrl-X and Up/Down to the same core commands. [Observed sequences and limits](NAVIGATION.md) record the evidence. Page-marked breaks, dot commands, and opaque selected lines are rejected until their navigation semantics are qualified.

## Word navigation: ws4-word-navigation-v1

Eighteen scenarios compare Ctrl-A/Ctrl-F movement with retained DOS status and unchanged synthetic carrier bytes. Word movement is line-local. The measured punctuation cases distinguish comma as a separator from an internal hyphen that stays in a word. Ctrl-F on a hard CRLF reaches the next line start, but Ctrl-F from the preceding word does not cross the break. The core owns cursor movement; Windows Ctrl-A/Ctrl-F select the commands. [Observed cases and limits](WORD_NAVIGATION.md) record source identities. Other punctuation, marked text, soft breaks, and opaque selected lines remain unqualified.

## Literal search: ws4-ascii-find-v1

Seventeen synthetic scenarios compare Ctrl-Q F and Ctrl-L with retained DOS status screens and unchanged synthetic carrier bytes. Forward search, overlapping repetition, case sensitivity, whole words, backward search, and failure cursor positions are covered at declared starting positions. [Observed rules and limits](FIND.md) define the profile. The core owns search state; the Windows adapter displays the WordStar prompt.

## Search and replace: ws4-ascii-replace-v1

Twelve synthetic scenarios compare Ctrl-Q A, Y/N answers, the `N`, `W`, and `U` options, Ctrl-L repetition, no-match behavior, and changes in word length against retained DOS screens and saved carriers. Replacement changes one occurrence per invocation and leaves the cursor at its start. The core owns the pending answer and keeps the carrier unchanged until `Y`. [Observed cases, DOS reopen check and limits](REPLACEMENT.md) define the bounded alphabetic ASCII profile. Other replacement options and UTF-8 replacement remain unqualified.

## Ctrl-G deletion: ws4-delete-character-v1

Five synthetic scenarios compare Ctrl-G with retained DOS screenshots and saved bytes. On an ordinary text cell it removes that byte and keeps the cursor position. On a hard CRLF it joins two short ordinary lines. At EOF it has no effect. One high-bit text cell and Insert-off deletion are included. Windows Ctrl-G routes to the core command. [Observed cases and limits](DELETION.md) record source identities and the DOS reopen check. Soft-break deletion remains unqualified.

## Backspace and Delete: ws4-backspace-delete-v1

Ten synthetic scenarios compare physical Backspace and Delete with retained DOS screenshots and saved bytes. Backspace removes the previous byte and moves the cursor back; Delete removes the byte under the cursor and leaves the cursor in place. Both can join an ordinary hard break. Document boundaries, Insert-off mode and high-bit text are included. The desktop routes both keys through the core; Delete shares Ctrl-G's deletion command. [Observations, source identities, DOS reopen check and limits](BACKSPACE_DELETE.md) define the scope.

## Block marks and copy: ws4-block-copy-v1

Eight synthetic scenarios compare Ctrl-K B, Ctrl-K K and Ctrl-K C against retained DOS screenshots and saved bytes. The marks define a half-open byte range. Copy inserts those bytes at the cursor, leaves the cursor in place and retains the selection for repeated copying. Copy before and inside the selection, an ordinary hard CRLF, and Insert-off mode are covered. Incomplete or empty blocks are rejected atomically. [Observed cases, source identities, DOS reopen check and limits](BLOCK_COPY.md) define the scope. Block hiding and later editing while marked remain unqualified.

## Block move and delete: ws4-block-move-delete-v1

Twenty synthetic scenarios compare Ctrl-K V and Ctrl-K Y with retained DOS screens and saved files. On an ordinary ASCII physical line, move relocates the block and its marks to a cursor position before or after the source. Delete clears the marks and can span one ordinary hard CRLF; it requires the cursor at or after the end mark. An end mark exactly one active byte before EOF is rejected because the DOS original loses that final byte in the measured move and delete cases. [Observed cases, DOS reopen checks and unresolved behavior](BLOCK_MOVE_DELETE.md) define the boundary. Cross-line movement and deletion across multiple breaks remain unqualified.

## Ctrl-Y and Ctrl-U: ws4-line-erasure-v1

Fourteen synthetic scenarios compare whole-line deletion and reuse of the deleted-line buffer with retained DOS runs. Ctrl-Y deletes an ordinary physical line, including its hard CRLF when present, and moves to the line start. Ctrl-U inserts a copy of the buffer at the current cursor, leaves the cursor and Insert mode unchanged, and keeps the buffer for later use. Ctrl-G does not fill or clear this buffer in the observed sequences. [Cases, source identities, interoperability check, and limits](ERASURE.md) define the scope. The core owns the buffer; CLI and desktop adapters select commands without reproducing its rules.

## Ctrl-T word erasure: ws4-word-erasure-v1

Twenty-seven synthetic scenarios compare Ctrl-T and its shared Ctrl-U buffer with retained DOS runs. At a word, Ctrl-T removes text from the cursor to a measured boundary; at spaces it removes the space run. On an ordinary hard break it joins the lines and clears the buffer. At EOF it leaves the buffer alone. Punctuation and adjacent-space cases are qualified only where measured. [Exact observations, source identities, interoperability check, and limits](WORD_ERASURE.md) define the scope. The core owns all erasure state.

## Carrier serialization

Explicit insertion, deletion, Enter and Ctrl-B serialize active bytes with at least two 1A bytes, padded to a multiple of 128 bytes. The old one-byte minimum was corrected using DOS captures at active lengths 126, 127, 128 and 129: their saved sizes are 128, 256, 256 and 256 bytes. The two-byte rule is a reconstruction generalized from these captures, not proof of every original save path or record boundary.

Ctrl-B also normalizes padding when active text is unchanged: 127 spaces grow from a 128-byte to a 256-byte carrier, while a 512-byte carrier containing only ALPHA shrinks to 128 bytes. Inspection, navigation, mode toggling, empty typing, and Enter that only moves the cursor preserve the exact original carrier. Selected Rust outputs have been reopened and saved in the original; see the Ctrl-B and editing reports for the tested scope.

## CLI and resource limits

`openstar inspect FILE` reads at most 64 MiB and reports SHA-256, complete token counts, and a bounded preview. At most 64 tokens and 32 raw bytes per token are displayed; the full carrier is examined. The first padding candidate does not stop tokenization.

`openstar replay CASE.json [--output NEW_FILE]` accepts at most 1 MiB of scenario JSON and 10000 commands. Unknown fields, versions, commands, malformed identities, and malformed hex are rejected. Output is created only after the declared comparisons pass, using the operating system's create-new operation. Existing files are never overwritten. An output I/O failure may leave the newly created file incomplete and is reported as failure.

## Remaining first-milestone work

General Ctrl-B behavior, right-justified and hyphen-aware wrapping, vertical movement outside the declared profile, and full file-save semantics remain unfinished. Ctrl-K S now saves the current file and creates a `.BAK` in the bounded local-file flow; concurrent writes and crash recovery remain unverified. [Observed save sequence and open questions](SAVING.md). The Windows interactive prototype has a bounded 80x25 screen comparison, including three scroll positions through line 14; later automatic page numbering is unknown. CP437 glyph rendering outside the tested screen, Unicode, input-method composition, accessibility, undo/recovery, PDF, and printing are separate work. Passing these profiles does not establish those features or general compatibility.
