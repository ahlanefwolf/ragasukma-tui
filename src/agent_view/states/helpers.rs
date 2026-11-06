use super::*;

pub(super) fn collect_recursive_tag_items(
    root: &Path,
    current: &Path,
    depth: usize,
    is_multi_root: bool,
    root_name: &str,
    out: &mut Vec<TagItem>,
) {
    if depth > 12 {
        return;
    }

    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.')
            || name == "target"
            || name == "node_modules"
            || name == ".git"
            || name == ".cargo"
        {
            continue;
        }

        if path.is_dir() {
            dirs.push((name, path));
        } else {
            files.push((name, path));
        }
    }

    dirs.sort_by(|a, b| a.0.cmp(&b.0));
    files.sort_by(|a, b| a.0.cmp(&b.0));

    for (name, path) in dirs {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let clean_path = if is_multi_root {
            format!("{}/{}/", root_name, rel_path)
        } else {
            format!("{}/", rel_path)
        };

        out.push(TagItem {
            path: clean_path,
            name,
            is_dir: true,
        });

        collect_recursive_tag_items(root, &path, depth + 1, is_multi_root, root_name, out);
    }

    for (name, path) in files {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let clean_path = if is_multi_root {
            format!("{}/{}", root_name, rel_path)
        } else {
            rel_path
        };

        out.push(TagItem {
            path: clean_path,
            name,
            is_dir: false,
        });
    }
}

pub(super) fn collect_visible_children(
    dir: &Path,
    depth: usize,
    expanded_dirs: &HashSet<String>,
    nodes: &mut Vec<WorkspaceFileNode>,
    parent_path: Option<String>,
) {
    if depth > 5 {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(_) => return,
    };

    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }

        if path.is_dir() {
            dirs.push((name, path));
        } else {
            files.push((name, path));
        }
    }

    dirs.sort_by(|a, b| a.0.cmp(&b.0));
    files.sort_by(|a, b| a.0.cmp(&b.0));

    for (name, path) in dirs {
        let path_str = path.to_string_lossy().to_string();
        let is_expanded = expanded_dirs.contains(&path_str);
        nodes.push(WorkspaceFileNode {
            path: path_str.clone(),
            name,
            is_dir: true,
            depth,
            is_expanded,
            parent_path: parent_path.clone(),
        });

        if is_expanded {
            collect_visible_children(&path, depth + 1, expanded_dirs, nodes, Some(path_str));
        }
    }

    for (name, path) in files {
        nodes.push(WorkspaceFileNode {
            path: path.to_string_lossy().to_string(),
            name,
            is_dir: false,
            depth,
            is_expanded: false,
            parent_path: parent_path.clone(),
        });
    }
}
