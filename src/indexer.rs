use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::graph::build_context_graph;
use crate::heuristics::GuaHeuristic;
use crate::import_graph::{
    build_import_graph, find_connected_components, get_imported_by, ImportGraph,
};
use crate::manifest::ManifestManager;
use crate::output::{write_compact_toml, write_schema_json};
use crate::parser::parse_files;
use crate::scanner::scan_files;
use crate::tree_sitter_parser::{
    extract_calls, extract_consumed_contexts, extract_exports, extract_provides_context,
};
use crate::types::{EnhancedComponent, EnhancedFunction, FileCommunity, FileEntity, FileInfo};

pub struct IndexerStats {
    pub files_scanned: usize,
    pub files_parsed: usize,
    pub functions_found: usize,
    pub components_found: usize,
    pub imports_found: usize,
    pub duration_ms: u128,
}

pub fn run_indexer(
    source_dir: &Path,
    output_dir: &Path,
    nato: Option<&str>,
) -> Result<IndexerStats, Box<dyn Error>> {
    let start = Instant::now();

    let manifest_path = output_dir.join("table_contents/manifest.json");
    let mut manifest_mgr = ManifestManager::new(manifest_path);

    let source_name = source_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let nato_version = nato
        .map(String::from)
        .unwrap_or_else(|| manifest_mgr.get_next_nato(source_name));

    println!("Discovering files...");
    let files = scan_files(source_dir);

    let paths: Vec<PathBuf> = files.iter().map(|f| source_dir.join(&f.path)).collect();

    println!("Parsing files...");
    let (functions, components, imports) = parse_files(&paths);

    println!("Applying heuristics...");
    let (file_entities, context_graph, communities) =
        build_file_entities(&files, &functions, &components, &imports, source_dir);

    println!("Writing TOML output...");
    write_compact_toml(
        &file_entities,
        &communities,
        source_name,
        &nato_version,
        output_dir,
    )?;

    println!("Writing schema.json output...");
    write_schema_json(
        &file_entities,
        &communities,
        source_name,
        &nato_version,
        output_dir,
    )?;

    manifest_mgr.add_version(
        source_name,
        &nato_version,
        files.len(),
        &format!("{}-{}.toml", source_name, nato_version),
    );
    manifest_mgr.save()?;

    let duration = start.elapsed();
    println!("Done in {:.2}s", duration.as_secs_f64());

    Ok(IndexerStats {
        files_scanned: files.len(),
        files_parsed: paths.len(),
        functions_found: functions.len(),
        components_found: components.len(),
        imports_found: imports.len(),
        duration_ms: duration.as_millis(),
    })
}

fn compute_complexity(lines: usize) -> &'static str {
    match lines {
        0..=50 => "TRIVIAL",
        51..=100 => "LOW",
        101..=200 => "MEDIUM",
        201..=500 => "MEDIUM-HIGH",
        501..=1000 => "HIGH",
        _ => "VERY_HIGH",
    }
}

fn build_file_entities(
    files: &[FileInfo],
    functions: &[crate::types::FunctionInfo],
    components: &[crate::types::ComponentInfo],
    imports: &[crate::types::ImportInfo],
    source_dir: &Path,
) -> (
    Vec<FileEntity>,
    crate::graph::ContextGraph,
    Vec<FileCommunity>,
) {
    let mut file_map: HashMap<String, Vec<EnhancedFunction>> = HashMap::new();
    let mut comp_map: HashMap<String, Vec<EnhancedComponent>> = HashMap::new();
    let mut imp_map: HashMap<String, Vec<String>> = HashMap::new();

    for f in functions {
        let relative_file = Path::new(&f.file)
            .strip_prefix(source_dir)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| f.file.clone());
        let mut enhanced = EnhancedFunction {
            name: f.name.clone(),
            file: relative_file.clone(),
            line_start: f.line_start,
            line_end: f.line_end,
            export_name: f.export_name.clone(),
            summary: None,
            confidence: None,
        };
        GuaHeuristic::apply_to_function(&mut enhanced);
        file_map.entry(relative_file).or_default().push(enhanced);
    }

    for c in components {
        let relative_file = Path::new(&c.file)
            .strip_prefix(source_dir)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| c.file.clone());
        let mut enhanced = EnhancedComponent {
            name: c.name.clone(),
            file: relative_file.clone(),
            export_name: c.export_name.clone(),
            summary: None,
            confidence: None,
        };
        GuaHeuristic::apply_to_component(&mut enhanced);
        comp_map.entry(relative_file).or_default().push(enhanced);
    }

    for i in imports {
        let relative_file = Path::new(&i.file)
            .strip_prefix(source_dir)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| i.file.clone());
        imp_map
            .entry(relative_file)
            .or_default()
            .push(i.import_path.clone());
    }

    let file_entities: Vec<FileEntity> = files
        .iter()
        .map(|f| {
            let funcs = file_map.remove(&f.path).unwrap_or_default();
            let comps = comp_map.remove(&f.path).unwrap_or_default();
            let imps = imp_map.remove(&f.path).unwrap_or_default();

            let related_hooks: Vec<String> = imps
                .iter()
                .filter(|i| i.starts_with("use"))
                .cloned()
                .collect();
            let related_components: Vec<String> = imps
                .iter()
                .filter(|i| {
                    let lower = i.to_lowercase();
                    !lower.contains("hook")
                        && !lower.contains("context")
                        && !lower.contains("utils")
                })
                .cloned()
                .collect();

            let source = fs::read_to_string(source_dir.join(&f.path)).unwrap_or_default();
            let exports = extract_exports(&source);
            let calls = extract_calls(&source);
            let provides_context = extract_provides_context(&source);
            let consumes_context = extract_consumed_contexts(&source);

            FileEntity {
                path: f.path.clone(),
                file_type: f.file_type.clone(),
                lines: f.lines,
                complexity: compute_complexity(f.lines).to_string(),
                functions: funcs,
                components: comps,
                imports: imps,
                related_hooks,
                related_components,
                exports,
                calls,
                provides_context,
                consumes_context,
                imported_by: vec![],
            }
        })
        .collect();

    let (_, reverse_graph, _) = build_import_graph(&file_entities);
    let file_entities: Vec<FileEntity> = file_entities
        .into_iter()
        .map(|mut entity| {
            entity.imported_by = get_imported_by(&reverse_graph, &entity.path);
            entity
        })
        .collect();

    let import_graph: ImportGraph = file_entities
        .iter()
        .map(|e| (e.path.clone(), e.imports.clone()))
        .collect();
    let communities = find_connected_components(&file_entities, &import_graph, &reverse_graph);

    let context_graph = build_context_graph(&file_entities);
    (file_entities, context_graph, communities)
}
