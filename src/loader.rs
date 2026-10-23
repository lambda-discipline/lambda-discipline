//! Reads source files and expands `use` items.
//!
//! `use "path"` splices the definitions and top-level expressions of another
//! file in at that point. Paths are relative to the directory of the file that
//! contains the `use` (the working directory for REPL input). Each file is
//! included at most once per compilation, so diamonds and cycles are fine.
//!
//! Files are read through a [`SourceProvider`]: [`FileSystem`] for real
//! programs, [`MemoryFiles`] where reading the disk would be wrong (tests,
//! fuzzing).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::diagnostics::Diagnostic;
use crate::source::{FileId, FileOrigin, SourceMap, Span};
use crate::syntax::ast::Item;
use crate::syntax::parse_module;

/// The items of a compilation unit, with every `use` already expanded.
#[derive(Debug, Default)]
pub struct Loaded {
    /// The file the unit was loaded from, unless it could not be read.
    pub root: Option<FileId>,
    pub items: Vec<Item>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Where the loader reads files from.
pub trait SourceProvider {
    /// The path that identifies `path`'s file, so that each file is
    /// included once however it is spelled. Fails if there is no such file.
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;

    /// The text of the file at a canonical path.
    fn read(&self, path: &Path) -> io::Result<String>;
}

/// Reads files from disk.
#[derive(Debug, Clone, Copy, Default)]
pub struct FileSystem;

impl SourceProvider for FileSystem {
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        fs::canonicalize(path)
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }
}

/// In-memory files. Paths are compared after resolving `.` and `..`
/// lexically; an empty set makes every `use` fail without touching the disk.
#[derive(Debug, Clone, Default)]
pub struct MemoryFiles {
    files: HashMap<PathBuf, String>,
}

impl MemoryFiles {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds (or replaces) the file at `path`.
    pub fn insert(&mut self, path: impl AsRef<Path>, text: impl Into<String>) {
        self.files.insert(normalize(path.as_ref()), text.into());
    }
}

impl SourceProvider for MemoryFiles {
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        let path = normalize(path);
        if self.files.contains_key(&path) {
            Ok(path)
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "no such file"))
        }
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such file"))
    }
}

/// Resolves `.` and `..` without looking at the disk.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// Loads a file from disk as the root of a compilation unit.
pub fn load_file(sources: &mut SourceMap, path: &Path) -> Loaded {
    load_file_from(sources, &FileSystem, path)
}

/// Loads in-memory text (REPL input, tests) as the root of a compilation
/// unit; its `use` items read from disk.
pub fn load_text(sources: &mut SourceMap, name: &str, text: &str) -> Loaded {
    load_text_from(sources, &FileSystem, name, text)
}

/// [`load_file`], reading every file through `provider`.
pub fn load_file_from(
    sources: &mut SourceMap,
    provider: &dyn SourceProvider,
    path: &Path,
) -> Loaded {
    let mut loader = Loader::new(sources, provider);
    match loader.read(path, None) {
        Ok(file) => {
            loader.loaded.root = Some(file);
            loader.expand(file);
        }
        Err(diagnostic) => loader.loaded.diagnostics.push(*diagnostic),
    }
    loader.loaded
}

/// [`load_text`], reading `use`d files through `provider`.
pub fn load_text_from(
    sources: &mut SourceMap,
    provider: &dyn SourceProvider,
    name: &str,
    text: &str,
) -> Loaded {
    let file = sources.add_virtual(name, text);
    let mut loader = Loader::new(sources, provider);
    loader.loaded.root = Some(file);
    loader.expand(file);
    loader.loaded
}

struct Loader<'a> {
    sources: &'a mut SourceMap,
    provider: &'a dyn SourceProvider,
    included: HashSet<PathBuf>,
    loaded: Loaded,
}

impl<'a> Loader<'a> {
    fn new(sources: &'a mut SourceMap, provider: &'a dyn SourceProvider) -> Self {
        Self {
            sources,
            provider,
            included: HashSet::new(),
            loaded: Loaded::default(),
        }
    }

    /// Reads and registers a file. `used_at` is the `use` item that asked for it.
    fn read(
        &mut self,
        path: &Path,
        used_at: Option<(Span, &str)>,
    ) -> Result<FileId, Box<Diagnostic>> {
        // Name the file as the user wrote it, not as it was resolved.
        let shown = used_at.map_or_else(
            || path.display().to_string(),
            |(_, written)| written.to_owned(),
        );
        let fail = |message: String| {
            let diagnostic = Diagnostic::error(message);
            Box::new(match used_at {
                Some((span, _)) => diagnostic.at(span).with_label("imported here"),
                None => diagnostic,
            })
        };
        let canonical = self
            .provider
            .canonicalize(path)
            .map_err(|e| fail(format!("cannot find `{shown}`: {e}")))?;
        let text = self
            .provider
            .read(&canonical)
            .map_err(|e| fail(format!("cannot read `{shown}`: {e}")))?;
        self.included.insert(canonical.clone());
        Ok(self.sources.add(FileOrigin::Disk(canonical), text))
    }

    fn expand(&mut self, file: FileId) {
        let source = self.sources.get(file);
        let base = source.base_dir().map(Path::to_path_buf);
        let (module, mut diagnostics) = parse_module(file, source.text());
        self.loaded.diagnostics.append(&mut diagnostics);
        for item in module.items {
            let Item::Use { path, span } = item else {
                self.loaded.items.push(item);
                continue;
            };
            let target = match &base {
                Some(dir) => dir.join(&path),
                None => PathBuf::from(&path),
            };
            if self
                .provider
                .canonicalize(&target)
                .is_ok_and(|c| self.included.contains(&c))
            {
                continue;
            }
            match self.read(&target, Some((span, &path))) {
                Ok(used) => self.expand(used),
                Err(diagnostic) => self.loaded.diagnostics.push(*diagnostic),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::ast::Item;

    fn def_names(loaded: &Loaded) -> Vec<String> {
        loaded
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Def(def) => Some(def.name.name.to_string()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn use_is_relative_to_the_including_file_and_included_once() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path().join("lib");
        fs::create_dir(&lib).unwrap();
        fs::write(lib.join("a.lbd"), "use \"b.lbd\"\na = 1").unwrap();
        fs::write(lib.join("b.lbd"), "use \"a.lbd\"\nb = 2").unwrap();
        fs::write(
            dir.path().join("main.lbd"),
            "use \"lib/a.lbd\"\nuse \"lib/b.lbd\"\nmain = 3",
        )
        .unwrap();

        let mut sources = SourceMap::new();
        let loaded = load_file(&mut sources, &dir.path().join("main.lbd"));
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert_eq!(def_names(&loaded), ["b", "a", "main"]);
    }

    #[test]
    fn missing_use_target_is_reported_at_the_use() {
        let mut sources = SourceMap::new();
        let loaded = load_text(
            &mut sources,
            "<test>",
            "use \"definitely/missing.lbd\"\nx = 1",
        );
        assert_eq!(loaded.diagnostics.len(), 1);
        let diagnostic = &loaded.diagnostics[0];
        assert!(
            diagnostic
                .message
                .starts_with("cannot find `definitely/missing.lbd`")
        );
        assert_eq!(diagnostic.span.unwrap().range(), 0..28);
        assert_eq!(def_names(&loaded), ["x"]);
    }

    #[test]
    fn missing_root_file_has_no_span() {
        let mut sources = SourceMap::new();
        let loaded = load_file(&mut sources, Path::new("no/such/file.lbd"));
        assert_eq!(loaded.diagnostics.len(), 1);
        assert!(loaded.diagnostics[0].span.is_none());
    }

    #[test]
    fn memory_files_resolve_relative_paths_and_include_once() {
        let mut files = MemoryFiles::new();
        files.insert("lib/a.lbd", "use \"../lib/./b.lbd\"\na = 1");
        files.insert("lib/b.lbd", "use \"a.lbd\"\nb = 2");
        let mut sources = SourceMap::new();
        let loaded = load_text_from(
            &mut sources,
            &files,
            "<test>",
            "use \"lib/a.lbd\"\nuse \"lib/b.lbd\"\nmain = 3",
        );
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert_eq!(def_names(&loaded), ["b", "a", "main"]);
    }

    #[test]
    fn empty_memory_files_never_read_the_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("real.lbd");
        fs::write(&path, "x = 1").unwrap();
        let mut sources = SourceMap::new();
        let text = format!("use {:?}\ny = 2", path.to_str().unwrap());
        let loaded = load_text_from(&mut sources, &MemoryFiles::new(), "<test>", &text);
        assert_eq!(loaded.diagnostics.len(), 1);
        assert!(loaded.diagnostics[0].message.starts_with("cannot find"));
        assert_eq!(def_names(&loaded), ["y"]);
    }

    #[test]
    fn parse_errors_from_used_files_are_collected() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("bad.lbd"), "x = (").unwrap();
        fs::write(dir.path().join("main.lbd"), "use \"bad.lbd\"\ny = 1").unwrap();
        let mut sources = SourceMap::new();
        let loaded = load_file(&mut sources, &dir.path().join("main.lbd"));
        assert_eq!(loaded.diagnostics.len(), 1);
        assert_eq!(def_names(&loaded), ["y"]);
    }
}
