//! Filesystem visibility filtering for the workspace file tree / tag autocomplete.
//!
//! Mirrors how a text editor decides what to show: a path is hidden when it is
//! matched by the ignore rules that apply to it (`.gitignore`, `.ignore`,
//! `.git/info/exclude`, and the user's global gitignore). There is NO hardcoded
//! list of folder names here -- the source of truth is the ignore files that
//! already live in the project.
//!
//! To keep it cheap (editors feel light because they are lazy / cached), the
//! matcher is cached: we build one [`Gitignore`] per root and reuse it. If the
//! ignore files change on disk, call [`reset`] to drop the cache.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use ignore::gitignore::{Gitignore, GitignoreBuilder};

thread_local! {
    /// Cache keyed by workspace root. Each entry is rebuilt only when the set of
    /// ignore files (or their mtimes) under that root changes.
    static MATCHERS: RefCell<HashMap<PathBuf, CachedMatcher>> = RefCell::new(HashMap::new());
}

struct CachedMatcher {
    matcher: Gitignore,
    /// Fingerprint of the ignore files that fed the matcher, used to detect
    /// changes without re-reading everything on every listing.
    fingerprint: Vec<(PathBuf, Option<SystemTime>)>,
}

/// Drop the whole cache so the next query re-reads ignore files from disk.
/// Call this after the user edits `.gitignore` / adds a folder, etc.
#[allow(dead_code)]
pub(super) fn reset() {
    MATCHERS.with(|m| m.borrow_mut().clear());
}

/// Returns `true` when `path` should be hidden from the tree because an ignore
/// rule (gitignore-style) matches it. `root` is the workspace folder the tree
/// is rooted at; `is_dir` tells the matcher how to read directory-only rules.
pub(super) fn is_ignored(root: &Path, path: &Path, is_dir: bool) -> bool {
    // Always hide VCS metadata directories. This is the single universal rule
    // every editor applies and it is required for `.gitignore` itself to make
    // sense (git never tracks its own object store).
    if is_vcs_metadata(path) {
        return true;
    }

    let root = canonical_root(root);
    let rel = match path.strip_prefix(&root) {
        Ok(r) => r.to_path_buf(),
        // Path lives outside the root; nothing we can decide here.
        Err(_) => return false,
    };
    if rel.as_os_str().is_empty() {
        return false;
    }

    MATCHERS.with(|cache| {
        let mut cache = cache.borrow_mut();

        let needs_rebuild = match cache.get(&root) {
            Some(entry) => entry.fingerprint != fingerprint(&root),
            None => true,
        };
        if needs_rebuild {
            cache.insert(root.clone(), build(&root));
        }

        let entry = match cache.get(&root) {
            Some(e) => e,
            None => return false,
        };
        entry
            .matcher
            .matched_path_or_any_parents(&rel, is_dir)
            .is_ignore()
    })
}

fn is_vcs_metadata(path: &Path) -> bool {
    path.components().any(|c| {
        let name = c.as_os_str().to_string_lossy();
        name == ".git" || name == ".hg" || name == ".svn"
    })
}

fn canonical_root(root: &Path) -> PathBuf {
    std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf())
}

/// Collect the ignore files that can affect `root` (from the filesystem root
/// down to `root`) so parent-level `.gitignore`s are respected like git does.
fn ignore_files_for(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut current = root;
    let mut chain: Vec<&Path> = Vec::new();
    loop {
        chain.push(current);
        match current.parent() {
            Some(p) => current = p,
            None => break,
        }
    }
    // Walk from the outermost ancestor down to the root so more specific
    // ignores override broader ones (gitignore precedence).
    for dir in chain.into_iter().rev() {
        for name in [".gitignore", ".ignore"] {
            let f = dir.join(name);
            if f.is_file() {
                files.push(f);
            }
        }
    }

    // Repo-local excludes.
    let git_exclude = root.join(".git/info/exclude");
    if git_exclude.is_file() {
        files.push(git_exclude);
    }

    // User's global gitignore (respects core.excludesFile when set).
    if let Some(g) = global_gitignore().filter(|g| g.is_file()) {
        files.push(g);
    }

    files
}

fn global_gitignore() -> Option<PathBuf> {
    // `ignore` already knows the platform default location; ask git config env
    // is overkill here, so prefer the documented XDG / HOME locations.
    if let Ok(p) = std::env::var("GIT_IGNORE_GLOBAL") {
        return Some(PathBuf::from(p));
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))?;
    let xdg = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    let candidates = [
        xdg.join("git/ignore"),
        home.join(".config/git/ignore"),
        home.join(".gitignore_global"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

fn fingerprint(root: &Path) -> Vec<(PathBuf, Option<SystemTime>)> {
    ignore_files_for(root)
        .into_iter()
        .map(|p| {
            let mtime = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
            (p, mtime)
        })
        .collect()
}

fn build(root: &Path) -> CachedMatcher {
    let files = ignore_files_for(root);
    let mut builder = GitignoreBuilder::new(root);
    for f in &files {
        // `add` returns Option<Error>; ignore malformed lines silently like git.
        let _ = builder.add(f);
    }
    let matcher = builder.build().unwrap_or_else(|_| Gitignore::empty());
    let fingerprint = files
        .into_iter()
        .map(|p| {
            let mtime = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
            (p, mtime)
        })
        .collect();
    CachedMatcher {
        matcher,
        fingerprint,
    }
}
