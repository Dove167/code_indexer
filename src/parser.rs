use rayon::prelude::*;
use regex::Regex;
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{ComponentInfo, FunctionInfo, ImportInfo};

pub fn parse_file(
    path: &Path,
) -> Result<(Vec<FunctionInfo>, Vec<ComponentInfo>, Vec<ImportInfo>), Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    let file = path.to_string_lossy().to_string();

    let functions = extract_functions(&source, &file);
    let components = extract_components(&source, &file);
    let imports = extract_imports(&source, &file);

    Ok((functions, components, imports))
}

fn extract_functions(source: &str, file: &str) -> Vec<FunctionInfo> {
    let mut functions = Vec::new();

    // Match function declarations: function name() or async function name()
    let func_regex =
        Regex::new(r"(?m)^\s*(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*\(").unwrap();
    for cap in func_regex.captures_iter(source) {
        if let Some(name) = cap.get(1) {
            let line = source[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
            functions.push(FunctionInfo {
                name: name.as_str().to_string(),
                file: file.to_string(),
                line_start: line,
                line_end: line,
                export_name: None, // detection would need more complex regex
            });
        }
    }

    // Match arrow functions: const name = () => or let name = () =>
    let arrow_regex = Regex::new(
        r"(?m)^\s*(?:export\s+)?(?:const|let|var)\s+(\w+)\s*=\s*(?:\([^)]*\)|[^=])\s*=>",
    )
    .unwrap();
    for cap in arrow_regex.captures_iter(source) {
        if let Some(name) = cap.get(1) {
            let line = source[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
            functions.push(FunctionInfo {
                name: name.as_str().to_string(),
                file: file.to_string(),
                line_start: line,
                line_end: line,
                export_name: Some(name.as_str().to_string()),
            });
        }
    }

    // Match method definitions in classes: name() {
    let method_regex = Regex::new(r"(?m)^\s*(?:async\s+)?(\w+)\s*\([^)]*\)\s*\{").unwrap();
    for cap in method_regex.captures_iter(source) {
        if let Some(name) = cap.get(1) {
            // Skip constructor and private methods
            let name_str = name.as_str();
            if !name_str.starts_with('_') && name_str != "constructor" {
                let line = source[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
                functions.push(FunctionInfo {
                    name: name_str.to_string(),
                    file: file.to_string(),
                    line_start: line,
                    line_end: line,
                    export_name: None,
                });
            }
        }
    }

    functions
}

fn extract_components(source: &str, file: &str) -> Vec<ComponentInfo> {
    let mut components = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Match JSX components: <PascalCase followed by space or >
    // This matches things like <Modal>, <AppointmentCard>, <DashboardPage>
    let component_regex = Regex::new(r"<([A-Z][a-zA-Z0-9]*)(?:\s|[/>])").unwrap();
    for cap in component_regex.captures_iter(source) {
        if let Some(name) = cap.get(1) {
            let name_str = name.as_str().to_string();
            // Skip common HTML elements that happen to be PascalCase (though rare)
            if !seen.contains(&name_str) && !is_common_html_element(&name_str) {
                seen.insert(name_str.clone());
                components.push(ComponentInfo {
                    name: name_str,
                    file: file.to_string(),
                    export_name: None,
                });
            }
        }
    }

    components
}

fn is_common_html_element(name: &str) -> bool {
    matches!(
        name,
        "Area"
            | "Base"
            | "Br"
            | "Col"
            | "Colgroup"
            | "Data"
            | "Datagrid"
            | "Details"
            | "Dialog"
            | "Embed"
            | "Figcaption"
            | "Figure"
            | "Footer"
            | "Frame"
            | "H1"
            | "H2"
            | "H3"
            | "H4"
            | "H5"
            | "H6"
            | "Header"
            | "Hr"
            | "Iframe"
            | "Img"
            | "Input"
            | "Ins"
            | "Kbd"
            | "Keygen"
            | "Label"
            | "Legend"
            | "Li"
            | "Link"
            | "Main"
            | "Map"
            | "Mark"
            | "Math"
            | "Menu"
            | "Menuitem"
            | "Meta"
            | "Meter"
            | "Nav"
            | "Nobr"
            | "Noscript"
            | "Object"
            | "Ol"
            | "Optgroup"
            | "Option"
            | "Output"
            | "Param"
            | "Progress"
            | "Queue"
            | "Rp"
            | "Rt"
            | "Ruby"
            | "S"
            | "Samp"
            | "Script"
            | "Section"
            | "Select"
            | "Shadow"
            | "Small"
            | "Source"
            | "Spacer"
            | "Span"
            | "Strong"
            | "Sub"
            | "Sup"
            | "Table"
            | "Tbody"
            | "Td"
            | "Template"
            | "Textarea"
            | "Tfoot"
            | "Th"
            | "Thead"
            | "Time"
            | "Title"
            | "Tr"
            | "Track"
            | "U"
            | "Ul"
            | "Var"
            | "Video"
            | "Wb"
            | "Webview"
    )
}

fn extract_imports(source: &str, file: &str) -> Vec<ImportInfo> {
    let mut imports = Vec::new();

    let import_regex =
        Regex::new(r#"import\s+(?:(\w+)\s*,)?\s*(?:\{([^}]+)\})?\s*from\s*["']([^"']+)["']"#)
            .unwrap();

    for cap in import_regex.captures_iter(source) {
        let raw_path = cap
            .get(3)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        if let Some(name) = cap.get(1) {
            imports.push(ImportInfo {
                file: file.to_string(),
                import_name: name.as_str().to_string(),
                import_path: raw_path.clone(),
            });
        }

        if let Some(named) = cap.get(2) {
            for part in named.as_str().split(',') {
                let trimmed = part.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let actual_name = if let Some(idx) = trimmed.find(" as ") {
                    &trimmed[..idx]
                } else {
                    trimmed
                };
                imports.push(ImportInfo {
                    file: file.to_string(),
                    import_name: actual_name.trim().to_string(),
                    import_path: raw_path.clone(),
                });
            }
        }
    }

    imports
}

pub fn parse_files(paths: &[PathBuf]) -> (Vec<FunctionInfo>, Vec<ComponentInfo>, Vec<ImportInfo>) {
    let results: Vec<(Vec<FunctionInfo>, Vec<ComponentInfo>, Vec<ImportInfo>)> = paths
        .par_iter()
        .filter_map(|path| match parse_file(path) {
            Ok(result) => Some(result),
            Err(_) => None,
        })
        .collect();

    let mut all_functions = Vec::new();
    let mut all_components = Vec::new();
    let mut all_imports = Vec::new();

    for (funcs, comps, imps) in results {
        all_functions.extend(funcs);
        all_components.extend(comps);
        all_imports.extend(imps);
    }

    (all_functions, all_components, all_imports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_functions() {
        let source = r#"
function hello() {}
export function world() {}
const foo = () => {};
"#;
        let funcs = extract_functions(source, "test.ts");
        assert!(!funcs.is_empty());
    }

    #[test]
    fn test_extract_imports() {
        let source = r#"
import React from 'react';
import { useState, useEffect } from 'react';
import Foo, { Bar, Baz as Qux } from 'module';
"#;
        let imports = extract_imports(source, "test.ts");
        assert!(imports.len() >= 5); // React, useState, useEffect, Foo, Bar, Baz (Qux is alias)
    }

    #[test]
    fn test_extract_components() {
        let source = r#"
<Modal>
  <AppointmentCard />
  <Dashboard />
</Modal>
"#;
        let components = extract_components(source, "test.tsx");
        assert!(components.iter().any(|c| c.name == "Modal"));
        assert!(components.iter().any(|c| c.name == "AppointmentCard"));
        assert!(components.iter().any(|c| c.name == "Dashboard"));
    }
}
