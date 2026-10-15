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

Every typed character must start at a column from 1 through 65. A command containing any later character is rejected atomically. This is a cursor-position bound, not a line-length bound: insertion near the start can extend an already long line. The original also accepts a space at column 66 without marking its predecessor; that retained observation remains outside this implementation. Automatic wrapping is not yet reconstructed.

Enter with Insert on inserts a hard CRLF at the cursor, preserving any existing break. With Insert off it moves past the next hard/soft CRLF without changing bytes; on a final unterminated line it appends CRLF at that line's end, including when invoked from the middle. Selected-line controls, tabs, framed characters, page-marked breaks, malformed data, and dot-command lines are unsupported. Errors leave bytes, cursor and modes unchanged. [Exact rules, comparisons and reopen evidence](EDITING.md).

Token-boundary movement, document-start navigation, and insert-mode toggling support replay. Their use in the declared sequences is compared; arbitrary visual navigation is not qualified.

## Ctrl-B: ws4-ascii-reformat-v1

Reformat scenarios require explicit initial formatting. The supported settings are document mode, single spacing, margins 1/65, and right justification off. Ctrl-B starts at document start or immediately after an ordinary hard CRLF; it rebuilds one paragraph and moves to the next hard paragraph or EOF. The current implementation handles printable ASCII, existing highbit markers, soft spaces and ordinary soft breaks. [The 27 comparisons and exact exclusions](CTRL_B.md) define the evidence boundary.

Formatting settings remain explicit in snapshots and can be compared by replay. Restoring state is a harness operation, not evidence that the original uses the same internal representation. Unsupported settings are retained, then rejected by affected mutations before changing state.

## Carrier serialization

Explicit insertion, Enter and Ctrl-B serialize active bytes with at least two 1A bytes, padded to a multiple of 128 bytes. The old one-byte minimum was corrected using DOS captures at active lengths 126, 127, 128 and 129: their saved sizes are 128, 256, 256 and 256 bytes. The two-byte rule is a reconstruction generalized from these captures, not proof of every original save path or record boundary.

Ctrl-B also normalizes padding when active text is unchanged: 127 spaces grow from a 128-byte to a 256-byte carrier, while a 512-byte carrier containing only ALPHA shrinks to 128 bytes. Inspection, navigation, mode toggling, empty typing, and Enter that only moves the cursor preserve the exact original carrier. Selected Rust outputs have been reopened and saved in the original; see the Ctrl-B and editing reports for the tested scope.

## CLI and resource limits

`northstar inspect FILE` reads at most 64 MiB and reports SHA-256, complete token counts, and a bounded preview. At most 64 tokens and 32 raw bytes per token are displayed; the full carrier is examined. The first padding candidate does not stop tokenization.

`northstar replay CASE.json [--output NEW_FILE]` accepts at most 1 MiB of scenario JSON and 10000 commands. Unknown fields, versions, commands, malformed identities, and malformed hex are rejected. Output is created only after the declared comparisons pass, using the operating system's create-new operation. Existing files are never overwritten. An output I/O failure may leave the newly created file incomplete and is reported as failure.

## Remaining first-milestone work

General Ctrl-B behavior, automatic wrapping while typing, right-justified reformatting, visual cursor behavior, full file-save semantics, and a Windows interactive adapter remain unfinished. CP437 glyph rendering, Unicode, input-method composition, accessibility, undo/recovery, PDF, and printing are separate work. Passing these profiles does not establish those features or general compatibility.
