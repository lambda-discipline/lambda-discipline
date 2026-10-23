//! Source files and byte-offset spans.
//!
//! Every file the compiler sees (a script on disk, a `use`d module, a REPL
//! snippet) is registered in a [`SourceMap`] and addressed by a [`FileId`].
//! Spans are half-open byte ranges into one file, which keeps them `Copy`
//! and cheap to attach to every token, AST node and IR node.

use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Identifies one file registered in a [`SourceMap`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(u32);

impl FileId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A half-open byte range `start..end` inside one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: FileId,
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(file: FileId, range: Range<usize>) -> Self {
        Self {
            file,
            start: range.start as u32,
            end: range.end as u32,
        }
    }

    /// The smallest span covering both `self` and `other` (same file).
    pub fn to(self, other: Span) -> Span {
        debug_assert_eq!(self.file, other.file);
        Span {
            file: self.file,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn range(self) -> Range<usize> {
        self.start as usize..self.end as usize
    }
}

/// Where a file came from; determines how it is displayed and how relative
/// `use` paths inside it are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileOrigin {
    /// A file read by path (from disk, or from in-memory files), stored as
    /// the canonical path the loader resolved.
    Disk(PathBuf),
    /// In-memory text (REPL input, tests). The string is a display name.
    Virtual(String),
}

#[derive(Debug)]
pub struct SourceFile {
    origin: FileOrigin,
    text: Rc<str>,
    /// Byte offset of the start of each line.
    line_starts: Vec<usize>,
}

impl SourceFile {
    fn new(origin: FileOrigin, text: Rc<str>) -> Self {
        let line_starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Self {
            origin,
            text,
            line_starts,
        }
    }

    pub fn origin(&self) -> &FileOrigin {
        &self.origin
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn name(&self) -> String {
        match &self.origin {
            FileOrigin::Disk(path) => display_path(path),
            FileOrigin::Virtual(name) => name.clone(),
        }
    }

    /// The directory relative `use` paths in this file are resolved against.
    pub fn base_dir(&self) -> Option<&Path> {
        match &self.origin {
            FileOrigin::Disk(path) => path.parent(),
            FileOrigin::Virtual(_) => None,
        }
    }

    /// [`base_dir`](Self::base_dir) as a program sees it (`__DIR__`): the
    /// absolute directory of a file on disk, `.` for in-memory text.
    pub fn dir_name(&self) -> String {
        self.base_dir().map_or_else(|| ".".to_owned(), display_path)
    }

    /// 1-based `(line, column)` of a byte offset; columns count characters.
    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let line = self.line_starts.partition_point(|&start| start <= offset) - 1;
        let start = self.line_starts[line];
        let end = offset.min(self.text.len());
        let column = self
            .text
            .get(start..end)
            .map_or(1, |s| s.chars().count() + 1);
        (line + 1, column)
    }
}

/// Strips the `\\?\` prefix Windows adds to canonical paths so messages stay
/// readable and `__DIR__` can be joined with `/` (`\\?\UNC\server\share`
/// becomes `\\server\share`).
fn display_path(path: &Path) -> String {
    let shown = path.display().to_string();
    if let Some(share) = shown.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{share}");
    }
    shown
        .strip_prefix(r"\\?\")
        .map(str::to_owned)
        .unwrap_or(shown)
}

/// Owns the text of every file seen during a session.
#[derive(Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, origin: FileOrigin, text: impl Into<Rc<str>>) -> FileId {
        let id = FileId(self.files.len() as u32);
        self.files.push(SourceFile::new(origin, text.into()));
        id
    }

    pub fn add_virtual(&mut self, name: impl Into<String>, text: impl Into<Rc<str>>) -> FileId {
        self.add(FileOrigin::Virtual(name.into()), text)
    }

    pub fn get(&self, id: FileId) -> &SourceFile {
        &self.files[id.index()]
    }

    /// Finds an already-loaded disk file by its canonical path.
    pub fn find_disk(&self, canonical: &Path) -> Option<FileId> {
        self.files
            .iter()
            .position(|f| matches!(&f.origin, FileOrigin::Disk(p) if p == canonical))
            .map(|i| FileId(i as u32))
    }

    pub fn snippet(&self, span: Span) -> &str {
        &self.get(span.file).text()[span.range()]
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_is_one_based_and_counts_chars() {
        let mut map = SourceMap::new();
        let id = map.add_virtual("t", "ab\nλx\n");
        let file = map.get(id);
        assert_eq!(file.line_col(0), (1, 1));
        assert_eq!(file.line_col(1), (1, 2));
        assert_eq!(file.line_col(3), (2, 1));
        // `λ` is two bytes but one column.
        assert_eq!(file.line_col(5), (2, 2));
        assert_eq!(file.line_col(7), (3, 1));
    }

    #[test]
    fn span_join_covers_both() {
        let mut map = SourceMap::new();
        let id = map.add_virtual("t", "hello world");
        let a = Span::new(id, 0..5);
        let b = Span::new(id, 6..11);
        assert_eq!(a.to(b).range(), 0..11);
        assert_eq!(b.to(a).range(), 0..11);
        assert_eq!(map.snippet(a.to(b)), "hello world");
    }

    #[test]
    fn virtual_files_have_no_base_dir() {
        let mut map = SourceMap::new();
        let id = map.add_virtual("<repl>", "");
        assert_eq!(map.get(id).name(), "<repl>");
        assert!(map.get(id).base_dir().is_none());
    }

    #[test]
    fn find_disk_matches_canonical_path() {
        let mut map = SourceMap::new();
        let path = PathBuf::from("/tmp/a.lbd");
        let id = map.add(FileOrigin::Disk(path.clone()), "");
        assert_eq!(map.find_disk(&path), Some(id));
        assert_eq!(map.find_disk(Path::new("/tmp/b.lbd")), None);
    }
}
