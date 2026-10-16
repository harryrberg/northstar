// SPDX-License-Identifier: GPL-3.0-only
//! Local inspection and retained-observation replay, not an interactive editor.

mod scenario;

use openstar_core::{Document, TokenKind};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{env, error::Error, fs::File, io::Read, path::Path, process::ExitCode};

// Application resource bounds, not legacy format restrictions.
const DOCUMENT_LIMIT: usize = 64 * 1024 * 1024;
const SCENARIO_LIMIT: usize = 1024 * 1024;

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(
            format!("input exceeds the application resource limit of {limit} bytes").into(),
        );
    }
    Ok(bytes)
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        result.push(DIGITS[(byte >> 4) as usize] as char);
        result.push(DIGITS[(byte & 15) as usize] as char);
    }
    result
}

fn inspect(path: &Path) -> Result<serde_json::Value, Box<dyn Error>> {
    let doc = Document::from_bytes(read_bounded(path, DOCUMENT_LIMIT)?);
    let mut preview = Vec::new();
    let mut tokens = 0;
    let mut unresolved = 0;
    for token in doc.tokens() {
        tokens += 1;
        if matches!(
            token.kind,
            TokenKind::UnresolvedFrame | TokenKind::UnpairedCarriageReturn | TokenKind::Unknown
        ) {
            unresolved += 1;
        }
        if preview.len() < 64 {
            let raw = &doc.bytes()[token.range.clone()];
            preview.push(json!({
                "token": token,
                "raw_hex_prefix": hex(&raw[..raw.len().min(32)]),
                "raw_prefix_complete": raw.len() <= 32,
            }));
        }
    }
    Ok(json!({
        "schema_version": 1,
        "operation": "inspect",
        "source_path": path,
        "source_sha256": sha256(doc.bytes()),
        "carrier_bytes": doc.bytes().len(),
        "editing_profile_active_end": doc.active_end(),
        "token_count": tokens,
        "unresolved_token_count": unresolved,
        "token_preview_complete": tokens <= preview.len(),
        "token_preview": preview,
        "target_executed": false,
        "source_modified": false,
        "limitations": [
            "Structural interpretation only; no legacy layout, dot-command execution, or glyph rendering.",
            "The active-end field is the bounded editing profile's first padding candidate, not a universal EOF claim.",
            "Preview contains at most 64 tokens and 32 raw bytes per token; counts examine the full carrier."
        ]
    }))
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let Some(operation) = args.next() else {
        return Err(
            "usage: openstar inspect FILE | openstar replay CASE.json [--output NEW_FILE]".into(),
        );
    };
    if operation == "--version" && args.next().is_none() {
        println!(
            "OpenStar 4.0 ({}) | bounded compatibility prototype",
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }
    let input = args.next().ok_or("input path is required")?;
    let report = if operation == "inspect" {
        if args.next().is_some() {
            return Err("inspect accepts one input path".into());
        }
        inspect(Path::new(&input))?
    } else if operation == "replay" {
        let output = match args.next() {
            None => None,
            Some(flag) if flag == "--output" => {
                Some(args.next().ok_or("--output requires a new file path")?)
            }
            Some(_) => return Err("unknown replay argument".into()),
        };
        if args.next().is_some() {
            return Err("unexpected trailing replay arguments".into());
        }
        let spec = scenario::parse(&read_bounded(Path::new(&input), SCENARIO_LIMIT)?)?;
        scenario::replay(spec, output.as_deref().map(Path::new))?
    } else {
        return Err("unknown operation; use inspect or replay".into());
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("openstar: {error}");
            ExitCode::FAILURE
        }
    }
}
