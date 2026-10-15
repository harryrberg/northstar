# First compatibility profile: ws4-ascii-enter-v1

This is a bounded prototype, not a complete WordStar editor. The reference is the primary floppy WS.EXE whose SHA-256 is `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`.

## State and ownership

The Rust core owns exact document carrier bytes, a zero-based cursor byte offset, and the insert flag. It keeps the original bytes for modification detection. UI and CLI adapters do not reinterpret command semantics. Cursor offsets cannot split a framed character or a CR/LF pair, or move into padding. No screen-column or Unicode-index equivalence is assumed.

Parsing partitions every input byte without rewriting it. It distinguishes hard/soft CR paired with ordinary/page-marked LF, 1B/9B framing, high-bit text/control values, soft spaces, tabs, padding candidates, and unresolved input. These are structural interpretations from the local investigation. Recognizing a structure does not establish its rendering or editing behavior.

The first padding token is the bounded editing profile's active end. The entire carrier, including material after that token, is retained. Editing rejects unsupported tokens and post-EOF payload rather than discarding them. Inspection remains available.

## Qualified comparisons

Two synthetic, redistributable JSON scenarios refer to hashed retained DOS evidence:

| Scenario | Reference observation | Compared result |
|---|---|---|
| enter-insert-on-alpha-beta | New document, Insert on, CTLNFL=FF: ALPHA / Enter / BETA / save | All 128 saved bytes via SHA-256; insert mode |
| enter-insert-off-next-line | ALPHA CRLF BETA, start at L01 C01, Insert off, CTLNFL=FF, Enter / save | Unchanged 128 saved bytes; insert mode; byte offset 7 derived from observed L02 C01 |

The insert-on scenario's empty initial carrier is a synthetic representation of a new document, not a captured DOS memory buffer. Final cursor and screen are not compared there. Replay uses semantic commands and restores declared initial state; it does not execute DOS menus, key events, or the original program. Source references are verified separately by `tools/verify-local-evidence.py` against an explicitly selected local investigation directory.

The expected carrier is constructed as ASCII `ALPHA`, `0D 0A`, ASCII `BETA`, and 117 `1A` bytes, then checked against the retained captures. Original software, screenshots, dictionaries, and decompiled code are not included.

## Implemented policy beyond the two comparisons

Short printable ASCII insertion, token-boundary movement, document-start navigation, and insert-mode toggling support the replay harness. Their general behavior is not yet individually qualified against DOS. Editing error returns leave state unchanged. Overwrite typing, overwrite Enter at a final line without CRLF, and insertion that would grow a line beyond 64 ASCII columns return explicit unsupported errors. This limit keeps the prototype away from unimplemented default-margin wrapping; it is not a general WordStar format limit.

After an insertion the prototype serializes active bytes plus at least one 1A byte to a multiple of 128 bytes. Only the ALPHA/Enter/BETA result has a retained save comparison. Arbitrary record-boundary behavior and save/reopen interoperability are still unqualified. A document that is never changed retains its original carrier exactly.

## CLI and resource limits

`northstar inspect FILE` reads at most 64 MiB and reports SHA-256, complete token counts, and a bounded preview. At most 64 tokens and 32 raw bytes per token are displayed; the full carrier is examined. The first padding candidate does not stop tokenization.

`northstar replay CASE.json [--output NEW_FILE]` accepts at most 1 MiB of scenario JSON and 10000 commands. Unknown fields, versions, commands, malformed identities, and malformed hex are rejected. Output is created only after the declared comparisons pass, using the operating system's create-new operation. Existing files are never overwritten. An output I/O failure may leave the newly created file incomplete and is reported as failure.

## Remaining first-milestone work

Ctrl-B, wrapping/justification, overwrite typing, visual cursor behavior, full file-save semantics, original-program reopen, and a Windows interactive adapter remain unfinished. CP437 glyph rendering, Unicode, input-method composition, accessibility, undo/recovery, PDF, and printing are separate work. Passing this profile does not establish those features or general compatibility.
