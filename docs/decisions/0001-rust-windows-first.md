# ADR 0001: Portable Rust core, Windows 11 first

Status: accepted by the project owner on 2026-10-15.

## Context

The project owner identifies the reconstruction method and its resulting fidelity to the original as the central differentiator. The audience includes authors seeking distraction-free writing without a dedicated DOS machine. Future clients may include desktop computers, tablets, e-paper devices, and Emacs.

Existing REA investigation outputs provide pseudocode, format decoders, and selected controlled DOS behavior. They do not constitute original source, a buildable reconstruction, or complete program semantics.

## Decision

Implement the first application in Rust for Windows 11. Keep document and editing semantics in a portable library. Select a Windows presentation layer only after a small input/rendering experiment. Flutter/Dart is no longer the default for the complete implementation; Flutter remains a possible future client.

Use the DOS original as a behavioral reference. Compare declared scenarios rather than guessing from decompiler output. Modern capabilities must be explicit extensions with their own tests.

Future platform support is an architectural objective, not a current compatibility claim. BOOX/Android is a candidate for an early e-paper experiment. Kindle and reMarkable require device-specific feasibility work. Emacs needs an explicit document-state ownership design.

## Consequences

The first deliverable prioritizes faithful editing and file interoperability over broad platform packaging. A common core can support multiple clients, but bindings, builds, rendering, input methods, accessibility, and device testing remain real work.

PDF layout and platform printing are separate from legacy printer-driver emulation. Licensing, final branding, GUI toolkit, Unicode representation, and modern/legacy mode boundaries remain open decisions.
