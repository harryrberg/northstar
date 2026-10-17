// SPDX-License-Identifier: GPL-3.0-only
use crate::{hex, sha256};
use openstar_core::{Command, Document, Editor, Formatting};
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
    formatting: Option<Formatting>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    carrier_sha256: String,
    cursor_byte: Option<usize>,
    insert: Option<bool>,
    formatting: Option<Formatting>,
    find_found: Option<bool>,
    replace_pending: Option<bool>,
    block_begin: Option<usize>,
    block_end: Option<usize>,
    block_visible: Option<bool>,
    marks_cleared: Option<bool>,
}

pub fn parse(bytes: &[u8]) -> Result<Scenario, Box<dyn Error>> {
    let spec: Scenario = serde_json::from_slice(bytes)?;
    if spec.schema_version != 1
        || !matches!(
            spec.profile.as_str(),
            "ws4-ascii-enter-v1"
                | "ws4-hard-break-v1"
                | "ws4-ascii-reformat-v1"
                | "ws4-ascii-edit-v1"
                | "ws4-ascii-wrap-v1"
                | "ws4-vertical-navigation-v1"
                | "ws4-horizontal-boundary-v1"
                | "ws4-quick-jump-v1"
                | "ws4-word-navigation-v1"
                | "ws4-delete-character-v1"
                | "ws4-line-erasure-v1"
                | "ws4-word-erasure-v1"
                | "ws4-ascii-find-v1"
                | "ws4-ascii-replace-v1"
                | "ws4-backspace-delete-v1"
                | "ws4-block-copy-v1"
                | "ws4-block-move-delete-v1"
                | "ws4-block-display-v1"
                | "ws4-line-right-erasure-v1"
                | "ws4-line-left-erasure-v1"
        )
    {
        return Err("unsupported scenario schema or compatibility profile".into());
    }
    if spec.profile != "ws4-ascii-enter-v1" && spec.initial.formatting.is_none() {
        return Err("editing scenarios require explicit initial formatting".into());
    }
    if spec.profile == "ws4-ascii-enter-v1" && spec.commands.contains(&Command::Reformat) {
        return Err("reformat is outside the declared Enter profile".into());
    }
    if (spec.profile == "ws4-hard-break-v1"
        && spec
            .commands
            .iter()
            .any(|command| !matches!(command, Command::InsertHardBreak)))
        || (spec.profile != "ws4-hard-break-v1"
            && spec.commands.contains(&Command::InsertHardBreak))
    {
        return Err("Ctrl-N requires its declared hard-break profile".into());
    }
    let find = |command: &Command| {
        matches!(
            command,
            Command::FindLiteral { .. }
                | Command::RepeatFind
                | Command::DocumentStart
                | Command::NextCharacter
        )
    };
    let replace = |command: &Command| {
        matches!(
            command,
            Command::ReplaceLiteral { .. }
                | Command::AnswerReplace { .. }
                | Command::RepeatFind
                | Command::DocumentStart
                | Command::NextCharacter
        )
    };
    if (spec.profile == "ws4-ascii-find-v1" && spec.commands.iter().any(|command| !find(command)))
        || (spec.profile == "ws4-ascii-replace-v1"
            && spec.commands.iter().any(|command| !replace(command)))
        || (!matches!(
            spec.profile.as_str(),
            "ws4-ascii-find-v1" | "ws4-ascii-replace-v1"
        ) && spec.commands.iter().any(|command| {
            matches!(
                command,
                Command::FindLiteral { .. }
                    | Command::ReplaceLiteral { .. }
                    | Command::AnswerReplace { .. }
                    | Command::RepeatFind
            )
        }))
    {
        return Err("find and replace commands require their declared profiles".into());
    }
    if matches!(
        spec.profile.as_str(),
        "ws4-ascii-find-v1" | "ws4-ascii-replace-v1"
    ) != spec.expected.find_found.is_some()
    {
        return Err("find or replace profile requires a declared found result".into());
    }
    if (spec.profile == "ws4-ascii-replace-v1") != spec.expected.replace_pending.is_some() {
        return Err("replace profile requires a declared pending state".into());
    }
    let vertical = |command: &Command| matches!(command, Command::NextLine | Command::PreviousLine);
    if (spec.profile == "ws4-vertical-navigation-v1"
        && spec.commands.iter().any(|command| !vertical(command)))
        || (spec.profile != "ws4-vertical-navigation-v1" && spec.commands.iter().any(vertical))
    {
        return Err("vertical movement requires its declared navigation profile".into());
    }
    let horizontal = |command: &Command| matches!(command, Command::LineStart | Command::LineEnd);
    if (spec.profile == "ws4-horizontal-boundary-v1"
        && spec.commands.iter().any(|command| !horizontal(command)))
        || (spec.profile != "ws4-horizontal-boundary-v1" && spec.commands.iter().any(horizontal))
    {
        return Err("line boundary movement requires its declared navigation profile".into());
    }
    let quick_jump = |command: &Command| {
        matches!(
            command,
            Command::DocumentEnd
                | Command::JumpBlockBegin
                | Command::JumpBlockEnd
                | Command::MarkBlockBegin
                | Command::MarkBlockEnd
                | Command::NextCharacter
        )
    };
    if (spec.profile == "ws4-quick-jump-v1"
        && spec.commands.iter().any(|command| !quick_jump(command)))
        || (spec.profile != "ws4-quick-jump-v1"
            && spec.commands.iter().any(|command| {
                matches!(
                    command,
                    Command::DocumentEnd | Command::JumpBlockBegin | Command::JumpBlockEnd
                )
            }))
    {
        return Err("quick jumps require their declared navigation profile".into());
    }
    let word_navigation =
        |command: &Command| matches!(command, Command::NextWord | Command::PreviousWord);
    if (spec.profile == "ws4-word-navigation-v1"
        && spec
            .commands
            .iter()
            .any(|command| !word_navigation(command)))
        || (spec.profile != "ws4-word-navigation-v1" && spec.commands.iter().any(word_navigation))
    {
        return Err("word movement requires its declared navigation profile".into());
    }
    let deletion = |command: &Command| matches!(command, Command::DeleteCharacter);
    if (spec.profile == "ws4-delete-character-v1"
        && spec.commands.iter().any(|command| !deletion(command)))
        || (!matches!(
            spec.profile.as_str(),
            "ws4-delete-character-v1"
                | "ws4-line-erasure-v1"
                | "ws4-word-erasure-v1"
                | "ws4-backspace-delete-v1"
        ) && spec.commands.iter().any(deletion))
    {
        return Err("character deletion requires its declared profile".into());
    }
    let backspace_delete =
        |command: &Command| matches!(command, Command::Backspace | Command::DeleteCharacter);
    if (spec.profile == "ws4-backspace-delete-v1"
        && spec
            .commands
            .iter()
            .any(|command| !backspace_delete(command)))
        || (spec.profile != "ws4-backspace-delete-v1"
            && spec
                .commands
                .iter()
                .any(|command| matches!(command, Command::Backspace)))
    {
        return Err("Backspace requires its declared profile".into());
    }
    let block_copy = |command: &Command| {
        matches!(
            command,
            Command::MarkBlockBegin
                | Command::MarkBlockEnd
                | Command::CopyBlock
                | Command::MoveBlock
                | Command::DeleteBlock
                | Command::NextCharacter
                | Command::PreviousCharacter
                | Command::DocumentStart
                | Command::ToggleInsert
        )
    };
    let block_display = |command: &Command| {
        matches!(
            command,
            Command::MarkBlockBegin
                | Command::MarkBlockEnd
                | Command::ToggleBlockDisplay
                | Command::NextCharacter
                | Command::DocumentStart
        )
    };
    if (spec.profile == "ws4-block-display-v1"
        && (spec.commands.iter().any(|command| !block_display(command))
            || !spec.commands.iter().any(|command| {
                matches!(
                    command,
                    Command::MarkBlockBegin | Command::MarkBlockEnd | Command::ToggleBlockDisplay
                )
            })
            || spec.expected.block_visible.is_none()))
        || (spec.profile != "ws4-block-display-v1"
            && (spec.commands.contains(&Command::ToggleBlockDisplay)
                || spec.expected.block_visible.is_some()))
    {
        return Err("block display requires its declared profile and expected visibility".into());
    }
    if (matches!(
        spec.profile.as_str(),
        "ws4-block-copy-v1" | "ws4-block-move-delete-v1"
    ) && spec.commands.iter().any(|command| !block_copy(command)))
        || (!matches!(
            spec.profile.as_str(),
            "ws4-block-copy-v1"
                | "ws4-block-move-delete-v1"
                | "ws4-quick-jump-v1"
                | "ws4-block-display-v1"
        ) && spec.commands.iter().any(|command| {
            matches!(
                command,
                Command::MarkBlockBegin
                    | Command::MarkBlockEnd
                    | Command::CopyBlock
                    | Command::MoveBlock
                    | Command::DeleteBlock
            )
        }))
        || (spec.profile == "ws4-block-copy-v1"
            && spec
                .commands
                .iter()
                .any(|command| matches!(command, Command::MoveBlock | Command::DeleteBlock)))
    {
        return Err("block commands require their declared profile".into());
    }
    if spec.profile == "ws4-block-copy-v1"
        && (spec.expected.block_begin.is_none() || spec.expected.block_end.is_none())
    {
        return Err("block copy profile requires expected mark positions".into());
    }
    if spec.profile == "ws4-block-move-delete-v1"
        && ((spec.expected.marks_cleared == Some(true))
            == (spec.expected.block_begin.is_some() && spec.expected.block_end.is_some()))
    {
        return Err(
            "block move/delete profile requires either retained mark positions or cleared marks"
                .into(),
        );
    }
    let line_erasure = |command: &Command| {
        matches!(
            command,
            Command::DeleteLine
                | Command::UneraseLine
                | Command::DeleteCharacter
                | Command::NextCharacter
                | Command::InsertAscii { .. }
                | Command::ToggleInsert
        )
    };
    let right_erasure = |command: &Command| {
        matches!(
            command,
            Command::DeleteToLineEnd | Command::UneraseLine | Command::NextCharacter
        )
    };
    if (spec.profile == "ws4-line-right-erasure-v1"
        && (spec.commands.iter().any(|command| !right_erasure(command))
            || !spec.commands.contains(&Command::DeleteToLineEnd)))
        || (spec.profile != "ws4-line-right-erasure-v1"
            && spec.commands.contains(&Command::DeleteToLineEnd))
    {
        return Err("Ctrl-Q Y requires its declared line-right erasure profile".into());
    }
    let left_erasure =
        |command: &Command| matches!(command, Command::DeleteToLineStart | Command::UneraseLine);
    if (spec.profile == "ws4-line-left-erasure-v1"
        && (spec.commands.iter().any(|command| !left_erasure(command))
            || !spec.commands.contains(&Command::DeleteToLineStart)))
        || (spec.profile != "ws4-line-left-erasure-v1"
            && spec.commands.contains(&Command::DeleteToLineStart))
    {
        return Err("Ctrl-Q Del requires its declared line-left erasure profile".into());
    }
    let word_erasure = |command: &Command| {
        matches!(
            command,
            Command::DeleteWord
                | Command::UneraseLine
                | Command::DeleteLine
                | Command::DeleteCharacter
                | Command::NextCharacter
                | Command::ToggleInsert
        )
    };
    if (spec.profile == "ws4-line-erasure-v1"
        && spec.commands.iter().any(|command| !line_erasure(command)))
        || (!matches!(
            spec.profile.as_str(),
            "ws4-line-erasure-v1"
                | "ws4-word-erasure-v1"
                | "ws4-line-right-erasure-v1"
                | "ws4-line-left-erasure-v1"
        ) && spec
            .commands
            .iter()
            .any(|command| matches!(command, Command::DeleteLine | Command::UneraseLine)))
    {
        return Err("line erasure requires its declared profile".into());
    }
    if (spec.profile == "ws4-word-erasure-v1"
        && spec.commands.iter().any(|command| !word_erasure(command)))
        || (spec.profile != "ws4-word-erasure-v1"
            && spec
                .commands
                .iter()
                .any(|command| matches!(command, Command::DeleteWord)))
    {
        return Err("word erasure requires its declared profile".into());
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
    let mut editor = Editor::restore_with_formatting(
        Document::from_bytes(input),
        spec.initial.cursor_byte,
        spec.initial.insert,
        spec.initial.formatting.unwrap_or_default(),
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
    if spec
        .expected
        .block_begin
        .is_some_and(|x| Some(x) != snapshot.block_begin)
    {
        mismatches.push("block_begin");
    }
    if spec
        .expected
        .block_end
        .is_some_and(|x| Some(x) != snapshot.block_end)
    {
        mismatches.push("block_end");
    }
    if spec.expected.marks_cleared == Some(true)
        && (snapshot.block_begin.is_some() || snapshot.block_end.is_some())
    {
        mismatches.push("marks_cleared");
    }
    if spec
        .expected
        .block_visible
        .is_some_and(|visible| visible != snapshot.block_visible)
    {
        mismatches.push("block_visible");
    }
    if spec
        .expected
        .find_found
        .is_some_and(|x| Some(x) != editor.last_find_result())
    {
        mismatches.push("find_found");
    }
    if spec
        .expected
        .replace_pending
        .is_some_and(|x| x != snapshot.replace_pending)
    {
        mismatches.push("replace_pending");
    }
    if spec
        .expected
        .formatting
        .is_some_and(|x| x != snapshot.formatting)
    {
        mismatches.push("formatting");
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
        "find_found": editor.last_find_result(),
        "compared_fields": {"carrier_sha256": true, "cursor_byte": spec.expected.cursor_byte.is_some(), "insert": spec.expected.insert.is_some(), "formatting": spec.expected.formatting.is_some(), "find_found": spec.expected.find_found.is_some(), "replace_pending": spec.expected.replace_pending.is_some(), "block_begin": spec.expected.block_begin.is_some(), "block_end": spec.expected.block_end.is_some(), "block_visible": spec.expected.block_visible.is_some(), "marks_cleared": spec.expected.marks_cleared == Some(true)},
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
