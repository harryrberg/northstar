// SPDX-License-Identifier: GPL-3.0-only
use northstar_core::{Command, Document, Editor};

fn editor(active: &[u8], cursor: usize) -> Editor {
    let mut carrier = active.to_vec();
    carrier.resize(128, 0x1a);
    Editor::restore(Document::from_bytes(carrier), cursor, true).unwrap()
}

#[test]
fn backspace_rejects_unqualified_breaks_and_opaque_cells_atomically() {
    for (active, cursor) in [
        (&b"AB\x8d\x0aCD"[..], 4),
        (&b"AB\x09CD"[..], 3),
        (&b".CMD\r\nTEXT"[..], 6),
    ] {
        let mut editor = editor(active, cursor);
        let carrier = editor.document().bytes().to_vec();
        let before = editor.snapshot();
        assert!(editor.apply(&Command::Backspace).is_err());
        assert_eq!(editor.document().bytes(), carrier);
        assert_eq!(editor.snapshot(), before);
    }
}
