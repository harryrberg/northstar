# ADR 0002: Separate native UTF-8 documents

Status: implemented for a bounded editing flow.

## Context

WordStar 4.0 documents are byte-oriented and may contain control and formatting data. Interpreting them as UTF-8 would change their meaning or lose bytes. Authors also need modern text input, including Norwegian letters.

## Decision

The user explicitly chooses the document mode. The D opening-menu command and an unqualified file argument use the WordStar 4.0 byte-preserving editor. F2 starts a new UTF-8 document. F3 opens an existing UTF-8 file or starts one at a missing path. `--utf8 FILE` opens an existing UTF-8 file from the command line. Invalid UTF-8 is rejected in that mode. There is no automatic detection or conversion between modes.

Native documents are plain UTF-8 text. New documents have no byte-order mark, and Enter inserts LF. Existing valid UTF-8 bytes, including a byte-order mark, decomposed text and mixed line endings, remain unchanged unless edited. The core owns text, a cursor at an extended grapheme boundary, and the saved checkpoint. Insertion, movement and deletion do not normalize stored text. The screen may compose a grapheme for display; this never changes the file.

The fixed 80x25 display shows one-cell graphemes. A grapheme that cannot be represented by one supported cell appears as a square placeholder while its exact UTF-8 remains available and saved. This is a display limitation, not conversion. The modern editor does not claim WordStar behavioral compatibility.

## Current boundary

The implemented flow includes typed text, Ctrl-V clipboard text, IME commit events, Left/Right or Ctrl-S/Ctrl-D movement, Backspace/Delete, Enter, Ctrl-Q R, saving to the current path, Save As to a new path, and reopen. IME preedit is not shown; real IME behavior still needs platform verification. Same-path saving creates a `.bak` beside the document and compares the current file bytes with those last opened or saved. Overwrite, word wrap, vertical movement, undo and crash recovery are not implemented in UTF-8 mode. The desktop read limit is 1 MiB; inserted text is also bounded to that size. Control input other than tab, CR and LF is rejected. Existing files are never overwritten by Save As. See [save behavior](../SAVING.md) for failure and concurrent-write limits.

Conversion between native and WordStar documents needs a separate, explicit import/export contract. In particular, it must report characters and formatting that cannot be represented in the selected legacy profile.
