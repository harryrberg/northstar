# OpenStar project instructions

Read README.md and docs/decisions/0001-rust-windows-first.md before implementation.

## Model selection

Before each task execution and each substantive iteration, reassess the model and reasoning effort against the actual work. Apply the same rule to subagents, selecting their model and effort explicitly when supported. Do not equate higher effort with verified fidelity; preserve independent evidence and tests.

Use GPT-6.1-Sol/high as the normal implementation starting point, and consider GPT-6-Astra/xhigh for difficult reconstruction or unresolved semantic analysis. Reassess these choices as model availability and task evidence change. Routine documentation may need less effort.

When the required model or effort cannot be changed automatically in the active session, tell the user the exact recommended settings before dependent work and let the user set them. Editing a configuration file does not prove that the active conversation changed models. Do not claim an unverified switch.

## Authority and scope

OpenStar is an open-source, non-commercial, fidelity-driven WordStar Professional 4.00 reconstruction in Rust, with Windows 11 as the first deliverable. Preserve portability for later desktop, tablet, e-paper, and Emacs clients. Keep this repository separate from REA.

Do not replace original behavior with a convenient approximation without recording the difference. Distinguish observed behavior, static derivations, hypotheses, and unknowns. Function export counts are not behavioral coverage.

## Architecture

The core owns document state and editing semantics. UI adapters translate input and display state; they must not duplicate command interpretation. Windows APIs, toolkit types, file dialogs, printer APIs, and device refresh mechanisms stay outside the core. Explicitly decide state ownership for Emacs; do not maintain two authoritative buffers.

Preserve legacy bytes and unknown control data. New Unicode documents need a separately specified representation and conversion policy. Separate compatibility behavior from modern extensions.

## Evidence and validation

Use docs/FIDELITY.md. Each claimed compatible behavior needs named evidence, exact starting conditions, and appropriate comparison tests. Use synthetic redistributable fixtures where possible. Do not commit original software, decompiled source, dictionaries, screenshots, or bulk investigation output by default.

Keep original uploads immutable. Any original-program execution must use isolated copies and record input identities and intentional changes. Validate save/reopen interoperability before reporting an editing milestone complete. Never overwrite user documents in tests.

Use focused meaningful checks for the actual change. Record untested platforms and unknown semantics. Do not present an empty scaffold or a passing byte round trip as a complete editor.

Use Conventional Commit subjects. The owner selected GPL-3.0-only and authorized public GitHub development on 2026-10-15. Preserve that license for original project contributions; do not apply it to the original WordStar software or third-party material. Do not change licensing or repository visibility without a new owner decision.
