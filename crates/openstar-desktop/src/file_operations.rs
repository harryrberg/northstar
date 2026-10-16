// SPDX-License-Identifier: GPL-3.0-only
//! Opening-menu file operations. A destination is never silently replaced.
use std::{fs, io, path::Path};

fn regular_file(path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Symbolic links are not supported",
        ));
    }
    if fs::metadata(path)?.is_file() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Not a regular file",
        ))
    }
}

pub fn copy(source: &Path, target: &Path) -> io::Result<()> {
    regular_file(source)?;
    let mut input = fs::File::open(source)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    match io::copy(&mut input, &mut output).and_then(|_| output.sync_all()) {
        Ok(_) => Ok(()),
        Err(problem) => {
            drop(output);
            // Retain an incomplete output when cleanup fails so it remains visible.
            if let Err(cleanup) = fs::remove_file(target) {
                return Err(io::Error::new(
                    problem.kind(),
                    format!(
                        "Copy failed: {problem}; incomplete target remains at {}: {cleanup}",
                        target.display()
                    ),
                ));
            }
            Err(problem)
        }
    }
}

pub fn rename(source: &Path, target: &Path) -> io::Result<()> {
    regular_file(source)?;
    // Hard-link creation fails if the target already exists. This prevents a
    // cross-platform rename race from replacing another file. Like DOS rename,
    // it is limited to the same volume.
    fs::hard_link(source, target)?;
    fs::remove_file(source).map_err(|problem| {
        io::Error::new(
            problem.kind(),
            format!(
                "New name {} exists, but the old name could not be removed: {problem}",
                target.display()
            ),
        )
    })
}

pub fn delete(path: &Path) -> io::Result<()> {
    regular_file(path)?;
    fs::remove_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[test]
    fn copy_rename_delete_preserve_bytes_and_existing_destinations() {
        static SERIAL: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "openstar-files-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        let source = dir.join("SOURCE.DOC");
        let copied = dir.join("COPY.DOC");
        let renamed = dir.join("RENAMED.DOC");
        fs::write(&source, [0, 0x1a, 0xff]).unwrap();
        fs::write(&copied, b"existing").unwrap();
        assert_eq!(
            copy(&source, &copied).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(&copied).unwrap(), b"existing");
        assert_eq!(
            rename(&source, &copied).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(&source).unwrap(), [0, 0x1a, 0xff]);
        fs::remove_file(&copied).unwrap();
        copy(&source, &copied).unwrap();
        assert_eq!(fs::read(&copied).unwrap(), fs::read(&source).unwrap());
        rename(&copied, &renamed).unwrap();
        assert!(!copied.exists());
        assert_eq!(fs::read(&renamed).unwrap(), fs::read(&source).unwrap());
        delete(&renamed).unwrap();
        assert!(!renamed.exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
