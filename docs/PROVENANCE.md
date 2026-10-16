# Analysis provenance

The analysis owner is the separate REA investigation at `C:\Projects\rea\investigations\wordstar` on the originating workstation. That path is a local reference, not a portable dependency or a publicly accessible evidence archive.

The primary target is the floppy edition of WordStar Professional 4.00. Primary WS.EXE SHA-256: `535235e6a392d344948d065a5553798e6119b2a0c25ccbaca2221b6b4860904d`.

The investigation retains REA Evidence, direct Ghidra exports, format models, and DOSBox runtime captures with distinct provenance. Its final aggregate verification reported 44 passing checks. This verifies the declared investigation artifacts; it does not establish complete program semantics or compatibility of this new implementation.

`analysis-baseline.json` pins the hashes of selected local summary documents at project establishment. It does not pin every underlying artifact. Before using a specific finding, resolve and record its actual source evidence and identity.

Useful starting reports in that directory:

- REPORT.md and STATUS.md: current aggregate findings and limitations.
- DOCUMENT_COVERAGE.md: document reader coverage and preserved controls.
- ENTER_TRACE.md, MODE_TRACES.md, REFORMAT_TRACE.md: selected observed editing behavior.
- CTRL_N_TRACE.md and PAGE_SCAN_TRACE.md: qualified control and page behavior.
- PRINTER_PAGES.md and printer reports: selected layout/output observations.

Known limits include incomplete global message classification, arbitrary indirect/error paths, complete spelling suggestion behavior, and unrestricted file compatibility. The investigation contains no buildable reconstruction.

No original program or bulk evidence is redistributed by establishing this repository. Original OpenStar contributions are licensed under GPL-3.0-only. This does not grant rights to original WordStar software, decompiled output, dictionaries, or other third-party material; any proposed redistribution needs a separate provenance and rights assessment.
