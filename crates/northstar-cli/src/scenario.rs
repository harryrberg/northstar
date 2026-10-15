// SPDX-License-Identifier: GPL-3.0-only
use crate::{hex, sha256};
use northstar_core::{Command, Document, Editor};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{error::Error, fs::OpenOptions, io::Write, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    schema_version: u32,
    id: String,
    profile: String,
    provenance: Provenance,
    initial: Initial,
    commands: Vec<Command>,
    expected: Expected,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    classification: String,
    executable_sha256: String,
    sources: Vec<Source>,
    limitations: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Source {
    path: String,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Initial {
    carrier_hex: String,
    cursor_byte: usize,
    insert: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    carrier_sha256: String,
    cursor_byte: Option<usize>,
    insert: Option<bool>,
}

pub fn parse(bytes: &[u8]) -> Result<Scenario, Box<dyn Error>> {
    let spec: Scenario = serde_json::from_slice(bytes)?;
    if spec.schema_version != 1 || spec.profile != "ws4-ascii-enter-v1" {
        return Err("unsupported scenario schema or compatibility profile".into());
    }
    if spec.commands.len() > 10_000 {
        return Err("scenario exceeds the 10000-command resource bound".into());
    }
    fn digest(value: &str) -> bool {
        value.len() == 64
            && value
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    }
    if !digest(&spec.expected.carrier_sha256)
        || !digest(&spec.provenance.executable_sha256)
        || spec.provenance.sources.is_empty()
        || spec
            .provenance
            .sources
            .iter()
            .any(|s| !digest(&s.sha256) || s.path.is_empty())
    {
        return Err("scenario needs a lowercase SHA-256 identity for expected bytes, reference executable, and each source".into());
    }
    Ok(spec)
}

fn decode_hex(value: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if !value.len().is_multiple_of(2) || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(
            "carrier_hex must contain complete hexadecimal bytes without separators".into(),
        );
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair)?;
            Ok(u8::from_str_radix(text, 16)?)
        })
        .collect()
}

pub fn replay(spec: Scenario, output: Option<&Path>) -> Result<serde_json::Value, Box<dyn Error>> {
    let input = decode_hex(&spec.initial.carrier_hex)?;
    let input_sha = sha256(&input);
    let mut editor = Editor::restore(
        Document::from_bytes(input),
        spec.initial.cursor_byte,
        spec.initial.insert,
    )?;
    let mut steps = Vec::new();
    for (index, command) in spec.commands.iter().enumerate() {
        editor
            .apply(command)
            .map_err(|e| format!("command {index}: {e}"))?;
        steps.push(json!({"command": command, "state": editor.snapshot()}));
    }
    let actual_hash = sha256(editor.document().bytes());
    let snapshot = editor.snapshot();
    let mut mismatches = Vec::new();
    if actual_hash != spec.expected.carrier_sha256 {
        mismatches.push("carrier_sha256");
    }
    if spec
        .expected
        .cursor_byte
        .is_some_and(|x| x != snapshot.cursor_byte)
    {
        mismatches.push("cursor_byte");
    }
    if spec.expected.insert.is_some_and(|x| x != snapshot.insert) {
        mismatches.push("insert");
    }
    if !mismatches.is_empty() {
        return Err(format!(
            "scenario {} differs in {} (actual carrier SHA-256: {actual_hash})",
            spec.id,
            mismatches.join(", ")
        )
        .into());
    }
    if let Some(path) = output {
        // create_new is enforced by the OS, including if input and output alias.
        // An I/O failure may leave this newly created file incomplete; never retry
        // by overwriting it and never remove a path whose identity is not owned.
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(editor.document().bytes())?;
        file.sync_all()?;
    }
    Ok(json!({
        "schema_version": 1,
        "operation": "replay",
        "scenario": spec.id,
        "profile": spec.profile,
        "passed": true,
        "input_sha256": input_sha,
        "carrier_sha256": actual_hash,
        "carrier_bytes": editor.document().bytes().len(),
        "active_hex": hex(&editor.document().bytes()[..editor.document().active_end()]),
        "state": snapshot,
        "steps": steps,
        "compared_fields": {"carrier_sha256": true, "cursor_byte": spec.expected.cursor_byte.is_some(), "insert": spec.expected.insert.is_some()},
        "output_path": output,
        "target_executed": false,
        "provenance": spec.provenance,
        "limitations": [
            "Local Rust replay against declared retained observations; no original-program execution occurs.",
            "Source references are retained, not fetched or revalidated by this command.",
            "Unspecified expected state is not compared; intermediate states are implementation observations."
        ]
    }))
}
