// SPDX-License-Identifier: GPL-3.0-only
//! Desktop-owned view of the logged directory. Document bytes remain in the core.
use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

const MAX_ENTRIES: usize = 4096;

#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub name: String,
    pub size: Option<u64>,
    pub is_directory: bool,
}

#[derive(Debug, Clone)]
pub struct DirectoryListing {
    pub path: PathBuf,
    pub entries: Vec<DirectoryEntry>,
    pub error: Option<String>,
    pub truncated: bool,
}

impl DirectoryListing {
    pub fn read(path: PathBuf) -> Self {
        let mut listing = Self {
            path,
            entries: Vec::new(),
            error: None,
            truncated: false,
        };
        let read = match fs::read_dir(&listing.path) {
            Ok(read) => read,
            Err(error) => {
                listing.error = Some(format!("Directory unavailable: {error}"));
                return listing;
            }
        };
        if listing.path.parent().is_some() {
            listing.entries.push(DirectoryEntry {
                name: "..".into(),
                size: None,
                is_directory: true,
            });
        }
        for item in read {
            if listing.entries.len() >= MAX_ENTRIES {
                listing.truncated = true;
                break;
            }
            match item {
                Ok(item) => {
                    let metadata = item.metadata().ok();
                    listing.entries.push(DirectoryEntry {
                        name: item.file_name().to_string_lossy().into_owned(),
                        size: metadata
                            .as_ref()
                            .filter(|value| value.is_file())
                            .map(fs::Metadata::len),
                        is_directory: metadata.is_some_and(|value| value.is_dir()),
                    });
                }
                Err(error) => {
                    listing.error = Some(format!("Directory entry unavailable: {error}"));
                }
            }
        }
        listing.entries.sort_by(|a, b| {
            (a.name != "..", a.name.to_ascii_lowercase(), &a.name).cmp(&(
                b.name != "..",
                b.name.to_ascii_lowercase(),
                &b.name,
            ))
        });
        listing
    }

    pub fn page_start(&self, requested: usize, page_size: usize) -> usize {
        if page_size == 0 || self.entries.is_empty() {
            return 0;
        }
        requested.min((self.entries.len() - 1) / page_size * page_size)
    }

    pub fn retain_match(&mut self, pattern: &str) {
        // WordStar's default DOS 8.3 match name means all documents. Modern
        // names remain visible under that default, while explicit patterns
        // use ASCII-case-insensitive * and ? matching.
        if pattern.is_empty() || pattern == "????????.???" {
            return;
        }
        self.entries
            .retain(|entry| entry.name == ".." || wildcard_match(pattern, &entry.name));
    }
}

fn wildcard_match(pattern: &str, name: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let name: Vec<char> = name.chars().collect();
    let (mut p, mut n, mut star, mut retry) = (0, 0, None, 0);
    while n < name.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p].eq_ignore_ascii_case(&name[n])) {
            p += 1;
            n += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            p += 1;
            retry = n;
        } else if let Some(previous) = star {
            p = previous + 1;
            retry += 1;
            n = retry;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

/// Resolve a typed filename against the directory displayed by the opening menu.
/// Windows drive-relative paths such as `C:FILE` have process-specific semantics.
pub fn resolve_path(directory: &Path, typed: &str) -> io::Result<PathBuf> {
    let path = Path::new(typed);
    if path
        .components()
        .next()
        .is_some_and(|component| matches!(component, Component::Prefix(_)))
        && !path.is_absolute()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Use an absolute path after the drive letter",
        ));
    }
    Ok(if path.is_absolute() {
        path.to_path_buf()
    } else {
        directory.join(path)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn directory_entries_are_sorted_and_paths_use_the_logged_directory() {
        let base = std::env::temp_dir().join(format!("openstar-directory-{}", std::process::id()));
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("z.DOC"), b"z").unwrap();
        fs::write(base.join("A.DOC"), b"a").unwrap();
        let listing = DirectoryListing::read(base.clone());
        let names: Vec<_> = listing
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert_eq!(names, vec!["..", "A.DOC", "z.DOC"]);
        assert_eq!(resolve_path(&base, "A.DOC").unwrap(), base.join("A.DOC"));
        fs::remove_file(base.join("z.DOC")).unwrap();
        fs::remove_file(base.join("A.DOC")).unwrap();
        fs::remove_dir(base).unwrap();
    }

    #[test]
    fn explicit_directory_match_filters_without_hiding_parent() {
        let mut listing = DirectoryListing {
            path: PathBuf::from("."),
            entries: vec![
                DirectoryEntry {
                    name: "..".into(),
                    size: None,
                    is_directory: true,
                },
                DirectoryEntry {
                    name: "One.DOC".into(),
                    size: Some(1),
                    is_directory: false,
                },
                DirectoryEntry {
                    name: "two.txt".into(),
                    size: Some(1),
                    is_directory: false,
                },
                DirectoryEntry {
                    name: "Æ.DOC".into(),
                    size: Some(1),
                    is_directory: false,
                },
            ],
            error: None,
            truncated: false,
        };
        listing.retain_match("*.doc");
        assert_eq!(
            listing
                .entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["..", "One.DOC", "Æ.DOC"]
        );
    }
}
