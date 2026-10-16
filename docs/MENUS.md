# Menu reconstruction status

The reference is the primary WordStar Professional 4.00 `WS.EXE` identified in [the compatibility profile](COMPATIBILITY_PROFILE.md). OpenStar draws the observed 80-column menu surfaces, but visible entries do not yet imply implemented commands.

## Opening Menu

| Entry | OpenStar behavior |
| --- | --- |
| D | Open or create a legacy document at the typed path. |
| F | Hide the directory, or ask for a match name before showing it. The default match name shows all modern filenames. |
| J | Open help. An opening-menu letter selects a topic. The explanations are OpenStar text, not copied WordStar help. |
| L | Choose an existing logged directory. Relative document and Save As paths use it. |
| O | Copy one regular file to a new name without replacing an existing target. |
| E | Rename one regular file on the same volume to a new name. |
| Y | Delete one regular file after a Y/N confirmation. |
| X | Exit. |
| N, P, M, I, T, C, R, Esc | Displayed but not implemented. |

F2 starts a new UTF-8 document, F3 opens one, and F4 offers recovery. These are OpenStar extensions, separate from the WordStar 4.0 menu.

The opening menu and D prompt show a four-column directory. PgUp/PgDn browse directory pages in the Opening Menu. The listing is bounded to 4096 entries and discloses truncation. OpenStar accepts long filenames, uses ASCII-case-insensitive `*` and `?` matching for explicit patterns, and sorts by filename. These choices are not yet qualified as WordStar's DOS 8.3 filtering, ordering, size display, or directory navigation. Opening an unlisted file by typing its name still works. L keeps a selected path in application state rather than changing the process-wide working directory.

## Edit Menu

Ctrl-K, Ctrl-Q, Ctrl-O, and Ctrl-P show separate Block & Save, Quick, Onscreen Format, and Print Controls menus in the original menu area. The document and bottom status rows remain visible. The menu surfaces were transcribed from retained DOS screenshots; command behavior is separate.

Ctrl-K S/D/Q/B/K/C/V/Y and Ctrl-Q R/F/A/S/D/B/K/C have the bounded behaviors in the [Windows prototype guide](WINDOWS_PROTOTYPE.md). Other Ctrl-K and Ctrl-Q choices, and all Ctrl-O and Ctrl-P choices, currently report that the command is not implemented. Direct editing commands and the function-key line have the limits in the same guide. Native UTF-8 mode retains its own key map and does not display the legacy prefix menus.

## Evidence and limits

The isolated DOS runs in `C:/Projects/rea/investigations/wordstar/runtime-openstar-menu-03` capture F off, F on with its match-name question, Enter with the default match, and L with the directory question. The run in `runtime-openstar-menus-05` opens `CHAPTER1.DOC` from the copied program disk and captures Ctrl-K/Q/O/P after a one-second wait. [File operation evidence](FILE_OPERATIONS.md) records O, E and Y. The source manifest and ordered actions are retained with each run. Earlier opening and D screens are in `runtime-openstar-ui-help-02`; editor Help is in `runtime-spell-01`. These observations establish the visible menus and prompts in those starting states. They do not establish the behavior of every menu command, option, error, or file-system state. No screenshot or original executable is committed here.

A separate isolated `runtime-openstar-protect-01` run captured C asking whether to change protection, but a second C invocation still reported the file as unprotected after Y. The DOSBox host filesystem did not provide a reliable protection result. C remains unimplemented.

The menu build is incomplete. Other file operations, printing, formatting menus, shorthand, spelling, many navigation commands, original help levels, and most footer keys still need separate behavioral observations, implementation, and differential checks.
