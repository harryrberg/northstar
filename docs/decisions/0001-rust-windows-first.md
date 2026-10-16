# ADR 0001: Portable Rust core, Windows 11 first

Status: accepted by the project owner on 2026-10-15.

## Context

The project owner identifies the reconstruction method and its resulting fidelity to the original as the central differentiator. The audience includes authors seeking distraction-free writing without a dedicated DOS machine. Future clients may include desktop computers, tablets, e-paper devices, and Emacs.

Existing REA investigation outputs provide pseudocode, format decoders, and selected controlled DOS behavior. They do not constitute original source, a buildable reconstruction, or complete program semantics.

## Decision

Implement the first application in Rust for Windows 11. Keep document and editing semantics in a portable library. The input and rendering experiment selected Iced 0.14 for the Windows prototype. It handles WordStar Ctrl prefixes, key repeat, focus changes, a retained 80x25 screen, and full-screen rendering. This choice does not bind later clients to Iced. Flutter/Dart is no longer the default for the complete implementation; Flutter remains a possible future client.

Use IBM Plex Mono as the default screen font so the 80x25 layout scales smoothly on high-resolution displays. Offer a VGA 8x16 outline recreation for readers who prefer its original pixel shapes. Keep screen cells and command semantics independent of the selected font. The current adapter scales the full grid to fit the window while retaining its 80-column layout.

Use the DOS original as a behavioral reference. Compare declared scenarios rather than guessing from decompiler output. Modern capabilities must be explicit extensions with their own tests.

Future platform support is an architectural objective, not a current compatibility claim. BOOX/Android is a candidate for an early e-paper experiment. Kindle and reMarkable require device-specific feasibility work. Emacs needs an explicit document-state ownership design.

## Consequences

The first deliverable prioritizes faithful editing and file interoperability over broad platform packaging. A common core can support multiple clients, but bindings, builds, rendering, input methods, accessibility, and device testing remain real work.

PDF layout and platform printing are separate from legacy printer-driver emulation. The project is named NorthStar (`northstar` on GitHub) and is open source and non-commercial. The owner selected GPL-3.0-only and public GitHub development on 2026-10-15. Native UTF-8 documents are specified separately in [ADR 0002](0002-utf8-documents.md). Legacy WordStar editing remains ASCII-only, and real IME behavior still needs platform verification.

The owner renamed the project and repository to OpenStar in October 2026. The earlier name above records the decision as originally made.
