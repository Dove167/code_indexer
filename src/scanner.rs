use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::FileInfo;

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "dist",
    "build",
    "target",
    ".DS_Store",
];

const INCLUDE_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx"];

pub fn discover_files(source_dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(source_dir)
        .into_iter()
        .filter_entry(|e| {
            if let Some(name) = e.file_name().to_str() {
                !SKIP_DIRS.contains(&name)
            } else {
                true
            }
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            if let Some(ext) = e.path().extension().and_then(|e| e.to_str()) {
                INCLUDE_EXTENSIONS.contains(&ext)
            } else {
                false
            }
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

pub fn infer_file_type(path: &str) -> String {
    let path_str = path.to_lowercase();

    if path_str.contains("pages/") {
        if path_str.ends_with("modal.tsx") {
            return "component".to_string();
        }
        return "page".to_string();
    }
    if path_str.contains("/components/")
        && (path_str.ends_with(".tsx") || path_str.ends_with(".jsx") || path_str.ends_with(".js"))
    {
        return "component".to_string();
    }
    if path_str.contains("/hooks/") && (path_str.ends_with(".ts") || path_str.ends_with(".js")) {
        return "hook".to_string();
    }
    if path_str.contains("/utils/") && (path_str.ends_with(".ts") || path_str.ends_with(".js")) {
        return "utility".to_string();
    }
    if path_str.contains("/contexts/") && (path_str.ends_with(".tsx") || path_str.ends_with(".jsx"))
    {
        return "context".to_string();
    }
    if path_str.contains("/lib/") && (path_str.ends_with(".ts") || path_str.ends_with(".js")) {
        return "lib".to_string();
    }
    if path_str.contains("/routes/") && (path_str.ends_with(".ts") || path_str.ends_with(".js")) {
        return "route".to_string();
    }
    if path_str.contains("/services/") && (path_str.ends_with(".ts") || path_str.ends_with(".js")) {
        return "service".to_string();
    }
    if path_str.contains("/controllers/")
        && (path_str.ends_with(".ts") || path_str.ends_with(".js"))
    {
        return "controller".to_string();
    }
    if path_str.contains("/middleware/") && (path_str.ends_with(".ts") || path_str.ends_with(".js"))
    {
        return "middleware".to_string();
    }
    if path_str.contains("/config/")
        || path_str.ends_with("config.ts")
        || path_str.ends_with("config.js")
    {
        return "config".to_string();
    }

    "unknown".to_string()
}

pub fn count_lines(path: &Path) -> usize {
    fs::read_to_string(path)
        .map(|content| content.lines().count())
        .unwrap_or(0)
}

pub fn scan_files(source_dir: &Path) -> Vec<FileInfo> {
    let files = discover_files(source_dir);
    files
        .par_iter()
        .filter_map(|path| {
            let relative_path = path
                .strip_prefix(source_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");

            let file_type = infer_file_type(&relative_path);

            let file_info = FileInfo {
                path: relative_path,
                file_type,
                lines: count_lines(path),
            };
            Some(file_info)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_file_type() {
        assert_eq!(infer_file_type("pages/Dashboard.tsx"), "page");
        assert_eq!(infer_file_type("pages/LoginModal.tsx"), "component");
        assert_eq!(infer_file_type("components/Modal.tsx"), "component");
        assert_eq!(infer_file_type("hooks/useAuth.ts"), "hook");
        assert_eq!(infer_file_type("utils/helper.ts"), "utility");
        assert_eq!(infer_file_type("contexts/AuthContext.tsx"), "context");
        assert_eq!(infer_file_type("lib/api.ts"), "lib");
        assert_eq!(infer_file_type("routes/index.ts"), "route");
        assert_eq!(infer_file_type("services/user.ts"), "service");
        assert_eq!(infer_file_type("controllers/user.ts"), "controller");
        assert_eq!(infer_file_type("middleware/auth.ts"), "middleware");
        assert_eq!(infer_file_type("config/index.ts"), "config");
        assert_eq!(infer_file_type("other/random.tsx"), "unknown");
    }
}
