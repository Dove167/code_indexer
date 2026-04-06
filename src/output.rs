use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::graph::ContextGraph;
use crate::types::{FileCommunity, FileEntity};

#[derive(Serialize)]
struct MetaSection {
    generated_at: String,
    source: String,
    nato: String,
}

#[derive(Serialize)]
struct SchemaJson {
    schema_version: String,
    meta: MetaSection,
    communities: Vec<CommunitySchema>,
    files: HashMap<String, FileSchema>,
}

#[derive(Serialize)]
struct FileSchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    provides_context: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    consumes_context: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    exports: Vec<ExportSchema>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    calls: Vec<CallSchema>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    imported_by: Vec<String>,
    hotspot_score: i64,
}

#[derive(Serialize)]
struct ExportSchema {
    name: String,
    kind: String,
    signature: String,
    parameters: Vec<ParameterSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    return_type: Option<String>,
    line_start: usize,
    line_end: usize,
}

#[derive(Serialize)]
struct ParameterSchema {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    param_type: Option<String>,
    required: bool,
}

#[derive(Serialize)]
struct CallSchema {
    target: String,
    confidence: String,
    call_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    heuristic_note: Option<String>,
}

#[derive(Serialize)]
struct CommunitySchema {
    id: usize,
    files: Vec<String>,
    size: usize,
}

pub fn write_compact_toml(
    file_entities: &[FileEntity],
    communities: &[FileCommunity],
    source_name: &str,
    nato_version: &str,
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut tbl = toml::Table::new();
    tbl.insert(
        "schema_version".to_string(),
        toml::Value::String("4.0".to_string()),
    );

    let total_files = file_entities.len();
    let total_functions: usize = file_entities.iter().map(|f| f.functions.len()).sum();
    let total_components: usize = file_entities.iter().map(|f| f.components.len()).sum();

    let mut meta = toml::Table::new();
    meta.insert(
        "generated_at".to_string(),
        toml::Value::String(chrono::Utc::now().to_rfc3339()),
    );
    meta.insert(
        "source".to_string(),
        toml::Value::String(source_name.to_string()),
    );
    meta.insert(
        "nato".to_string(),
        toml::Value::String(nato_version.to_string()),
    );
    meta.insert(
        "manifest_path".to_string(),
        toml::Value::String("table_contents/manifest.json".to_string()),
    );
    tbl.insert("meta".to_string(), toml::Value::Table(meta));

    let mut map = toml::Table::new();
    map.insert(
        "total_files".to_string(),
        toml::Value::Integer(total_files as i64),
    );
    map.insert(
        "total_functions".to_string(),
        toml::Value::Integer(total_functions as i64),
    );
    map.insert(
        "total_components".to_string(),
        toml::Value::Integer(total_components as i64),
    );
    tbl.insert("map".to_string(), toml::Value::Table(map));

    let mut landmarks = toml::Table::new();
    let mut hotspots: Vec<&FileEntity> = file_entities.iter().collect();
    hotspots.sort_by_key(|f| f.imported_by.len());
    hotspots.reverse();
    let top_hotspots: Vec<_> = hotspots.into_iter().take(10).collect();
    let mut landmark_arr = toml::value::Array::new();
    for f in top_hotspots {
        let mut entry = toml::Table::new();
        entry.insert("file".to_string(), toml::Value::String(f.path.clone()));
        entry.insert(
            "hotspot".to_string(),
            toml::Value::Integer(f.imported_by.len() as i64),
        );
        entry.insert("type".to_string(), toml::Value::String(f.file_type.clone()));
        landmark_arr.push(toml::Value::Table(entry));
    }
    landmarks.insert("top_files".to_string(), toml::Value::Array(landmark_arr));

    let mut domains: toml::Table = toml::Table::new();

    fn get_top_level_dir(path: &str) -> Option<String> {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() > 1 && !parts[0].is_empty() && !parts[0].starts_with('.') {
            Some(parts[0].to_string())
        } else {
            None
        }
    }

    fn get_subdirs(files: &[&FileEntity]) -> Vec<String> {
        let mut subdirs: std::collections::HashSet<String> = std::collections::HashSet::new();
        for f in files {
            let parts: Vec<&str> = f.path.split('/').collect();
            if parts.len() > 2 {
                subdirs.insert(parts[1].to_string());
            }
        }
        let mut v: Vec<String> = subdirs.into_iter().collect();
        v.sort();
        v
    }

    fn generate_domain_desc(subdirs: &[String]) -> String {
        if subdirs.is_empty() {
            return "Root level files".to_string();
        }
        let preview = subdirs
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        if subdirs.len() > 5 {
            format!("{}...", preview)
        } else {
            preview
        }
    }

    let mut domain_map: std::collections::HashMap<String, Vec<&FileEntity>> =
        std::collections::HashMap::new();
    for f in file_entities {
        if let Some(tld) = get_top_level_dir(&f.path) {
            domain_map.entry(tld).or_default().push(f);
        }
    }

    let mut domain_names: Vec<String> = domain_map.keys().cloned().collect();
    domain_names.sort();

    for name in domain_names {
        let files = domain_map.remove(&name).unwrap_or_default();
        let subdirs = get_subdirs(&files);

        let mut domain_tbl = toml::Table::new();
        domain_tbl.insert(
            "files".to_string(),
            toml::Value::Integer(files.len() as i64),
        );
        domain_tbl.insert(
            "desc".to_string(),
            toml::Value::String(generate_domain_desc(&subdirs)),
        );
        let hooks = files.iter().filter(|f| f.file_type == "hook").count();
        let components = files.iter().filter(|f| f.file_type == "component").count();
        let pages = files.iter().filter(|f| f.file_type == "page").count();
        let utils = files
            .iter()
            .filter(|f| f.file_type == "utility" || f.file_type == "lib")
            .count();
        let types = files
            .iter()
            .filter(|f| f.file_type == "unknown" && f.path.contains("/types/"))
            .count();
        domain_tbl.insert("hooks".to_string(), toml::Value::Integer(hooks as i64));
        domain_tbl.insert(
            "components".to_string(),
            toml::Value::Integer(components as i64),
        );
        domain_tbl.insert("pages".to_string(), toml::Value::Integer(pages as i64));
        domain_tbl.insert("utils".to_string(), toml::Value::Integer(utils as i64));
        domain_tbl.insert("types".to_string(), toml::Value::Integer(types as i64));
        domains.insert(name, toml::Value::Table(domain_tbl));
    }

    landmarks.insert("domains".to_string(), toml::Value::Table(domains));
    tbl.insert("landmarks".to_string(), toml::Value::Table(landmarks));

    let mut territories = toml::Table::new();
    let significant_communities: Vec<_> = communities.iter().filter(|c| c.size > 1).collect();
    let singleton_count = communities.iter().filter(|c| c.size == 1).count();
    territories.insert(
        "singleton_count".to_string(),
        toml::Value::Integer(singleton_count as i64),
    );
    let mut community_arr = toml::value::Array::new();
    for c in significant_communities {
        let mut entry = toml::Table::new();
        entry.insert(
            "name".to_string(),
            toml::Value::String(c.name.clone().unwrap_or_default()),
        );
        entry.insert("size".to_string(), toml::Value::Integer(c.size as i64));
        entry.insert(
            "desc".to_string(),
            toml::Value::String(c.description.clone().unwrap_or_default()),
        );
        let example_files: Vec<String> = c.files.iter().take(3).cloned().collect();
        entry.insert(
            "examples".to_string(),
            toml::Value::Array(example_files.into_iter().map(toml::Value::String).collect()),
        );
        community_arr.push(toml::Value::Table(entry));
    }
    territories.insert("clusters".to_string(), toml::Value::Array(community_arr));
    tbl.insert("territories".to_string(), toml::Value::Table(territories));

    let mut files_tbl = toml::Table::new();

    fn add_file_entry(file_entry: &mut toml::Table, f: &FileEntity, include_detail: bool) {
        file_entry.insert("type".to_string(), toml::Value::String(f.file_type.clone()));
        file_entry.insert(
            "hotspot".to_string(),
            toml::Value::Integer(f.imported_by.len() as i64),
        );
        if !f.exports.is_empty() && include_detail {
            let exports: Vec<String> = f.exports.iter().map(|e| e.name.clone()).collect();
            file_entry.insert(
                "exports".to_string(),
                toml::Value::Array(exports.into_iter().map(toml::Value::String).collect()),
            );
        }
    }

    for f in file_entities {
        let mut file_entry = toml::Table::new();
        add_file_entry(&mut file_entry, f, false);
        files_tbl.insert(f.path.clone(), toml::Value::Table(file_entry));
    }
    tbl.insert("files".to_string(), toml::Value::Table(files_tbl));

    let output_filename = format!("{}-{}.toml", source_name, nato_version);
    let output_path = output_dir.join("table_contents").join(&output_filename);
    fs::create_dir_all(output_path.parent().unwrap())?;
    let content = toml::to_string_pretty(&tbl)?;
    fs::write(&output_path, content)?;

    let mut detail_tbl = toml::Table::new();
    detail_tbl.insert(
        "schema_version".to_string(),
        toml::Value::String("4.0".to_string()),
    );
    detail_tbl.insert(
        "note".to_string(),
        toml::Value::String("Full detail - query this section for specific files".to_string()),
    );

    let mut detail_files = toml::Table::new();
    for f in file_entities {
        let mut file_entry = toml::Table::new();
        file_entry.insert("type".to_string(), toml::Value::String(f.file_type.clone()));
        file_entry.insert("lines".to_string(), toml::Value::Integer(f.lines as i64));
        file_entry.insert(
            "complexity".to_string(),
            toml::Value::String(f.complexity.clone()),
        );
        file_entry.insert(
            "hotspot".to_string(),
            toml::Value::Integer(f.imported_by.len() as i64),
        );

        if !f.imports.is_empty() {
            let imports_arr: Vec<toml::Value> = f
                .imports
                .iter()
                .map(|s| toml::Value::String(s.clone()))
                .collect();
            file_entry.insert("imports".to_string(), toml::Value::Array(imports_arr));
        }
        if !f.imported_by.is_empty() {
            let imported_by_arr: Vec<toml::Value> = f
                .imported_by
                .iter()
                .map(|s| toml::Value::String(s.clone()))
                .collect();
            file_entry.insert(
                "imported_by".to_string(),
                toml::Value::Array(imported_by_arr),
            );
        }
        if !f.exports.is_empty() {
            let mut exports_arr: Vec<toml::Value> = Vec::new();
            for exp in &f.exports {
                let mut exp_tbl = toml::Table::new();
                exp_tbl.insert("name".to_string(), toml::Value::String(exp.name.clone()));
                exp_tbl.insert(
                    "kind".to_string(),
                    toml::Value::String(format!("{:?}", exp.kind)),
                );
                exp_tbl.insert(
                    "signature".to_string(),
                    toml::Value::String(exp.signature.clone()),
                );
                exp_tbl.insert(
                    "line_start".to_string(),
                    toml::Value::Integer(exp.line_start as i64),
                );
                exp_tbl.insert(
                    "line_end".to_string(),
                    toml::Value::Integer(exp.line_end as i64),
                );
                exports_arr.push(toml::Value::Table(exp_tbl));
            }
            file_entry.insert("exports".to_string(), toml::Value::Array(exports_arr));
        }
        if !f.calls.is_empty() {
            let mut calls_arr: Vec<toml::Value> = Vec::new();
            for call in &f.calls {
                let mut call_tbl = toml::Table::new();
                call_tbl.insert(
                    "target".to_string(),
                    toml::Value::String(call.target.clone()),
                );
                call_tbl.insert(
                    "confidence".to_string(),
                    toml::Value::String(call.confidence.to_gua_string()),
                );
                calls_arr.push(toml::Value::Table(call_tbl));
            }
            file_entry.insert("calls".to_string(), toml::Value::Array(calls_arr));
        }
        if let Some(ref provides_ctx) = f.provides_context {
            file_entry.insert(
                "provides_context".to_string(),
                toml::Value::String(provides_ctx.clone()),
            );
        }
        if !f.consumes_context.is_empty() {
            let consumes_arr: Vec<toml::Value> = f
                .consumes_context
                .iter()
                .map(|s| toml::Value::String(s.clone()))
                .collect();
            file_entry.insert(
                "consumes_context".to_string(),
                toml::Value::Array(consumes_arr),
            );
        }
        detail_files.insert(f.path.clone(), toml::Value::Table(file_entry));
    }
    detail_tbl.insert("files".to_string(), toml::Value::Table(detail_files));

    let detail_filename = format!("{}-{}.detail.toml", source_name, nato_version);
    let detail_path = output_dir.join("table_contents").join(&detail_filename);
    let detail_content = toml::to_string_pretty(&detail_tbl)?;
    fs::write(&detail_path, detail_content)?;

    Ok(())
}

pub fn write_rich_toml(
    file_entities: &[FileEntity],
    communities: &[FileCommunity],
    context_graph: &ContextGraph,
    source_name: &str,
    nato_version: &str,
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut tbl = toml::Table::new();
    tbl.insert(
        "schema_version".to_string(),
        toml::Value::String("3.1".to_string()),
    );
    let mut communities_tbl = toml::Table::new();
    for community in communities {
        let mut community_entry = toml::Table::new();
        let files_arr: Vec<toml::Value> = community
            .files
            .iter()
            .map(|s| toml::Value::String(s.clone()))
            .collect();
        community_entry.insert("files".to_string(), toml::Value::Array(files_arr));
        community_entry.insert(
            "size".to_string(),
            toml::Value::Integer(community.size as i64),
        );
        if let Some(ref name) = community.name {
            community_entry.insert("name".to_string(), toml::Value::String(name.clone()));
        }
        if let Some(ref description) = community.description {
            community_entry.insert(
                "description".to_string(),
                toml::Value::String(description.clone()),
            );
        }
        communities_tbl.insert(
            format!("{}", community.id),
            toml::Value::Table(community_entry),
        );
    }
    tbl.insert(
        "communities".to_string(),
        toml::Value::Table(communities_tbl),
    );
    let mut meta = toml::Table::new();
    meta.insert(
        "generated_at".to_string(),
        toml::Value::String(chrono::Utc::now().to_rfc3339()),
    );
    meta.insert(
        "source".to_string(),
        toml::Value::String(source_name.to_string()),
    );
    meta.insert(
        "nato".to_string(),
        toml::Value::String(nato_version.to_string()),
    );
    meta.insert(
        "manifest_path".to_string(),
        toml::Value::String("table_contents/manifest.json".to_string()),
    );
    tbl.insert("meta".to_string(), toml::Value::Table(meta));
    let mut stats = toml::Table::new();
    stats.insert(
        "total_files".to_string(),
        toml::Value::Integer(file_entities.len() as i64),
    );
    let total_functions: usize = file_entities.iter().map(|f| f.functions.len()).sum();
    let total_components: usize = file_entities.iter().map(|f| f.components.len()).sum();
    stats.insert(
        "total_functions".to_string(),
        toml::Value::Integer(total_functions as i64),
    );
    stats.insert(
        "total_components".to_string(),
        toml::Value::Integer(total_components as i64),
    );
    tbl.insert("stats".to_string(), toml::Value::Table(stats));
    let mut files_tbl = toml::Table::new();
    for f in file_entities {
        let mut file_entry = toml::Table::new();
        file_entry.insert("type".to_string(), toml::Value::String(f.file_type.clone()));
        file_entry.insert("lines".to_string(), toml::Value::Integer(f.lines as i64));
        file_entry.insert(
            "complexity".to_string(),
            toml::Value::String(f.complexity.clone()),
        );
        if !f.functions.is_empty() {
            let mut funcs_arr: Vec<toml::Value> = Vec::new();
            for func in &f.functions {
                let mut func_tbl = toml::Table::new();
                func_tbl.insert("name".to_string(), toml::Value::String(func.name.clone()));
                func_tbl.insert(
                    "line_start".to_string(),
                    toml::Value::Integer(func.line_start as i64),
                );
                func_tbl.insert(
                    "line_end".to_string(),
                    toml::Value::Integer(func.line_end as i64),
                );
                if let Some(ref export) = func.export_name {
                    func_tbl.insert("export".to_string(), toml::Value::String(export.clone()));
                }
                if let Some(ref summary) = func.summary {
                    func_tbl.insert("summary".to_string(), toml::Value::String(summary.clone()));
                }
                if let Some(ref confidence) = func.confidence {
                    func_tbl.insert(
                        "confidence".to_string(),
                        toml::Value::String(confidence.to_string()),
                    );
                }
                funcs_arr.push(toml::Value::Table(func_tbl));
            }
            file_entry.insert("functions".to_string(), toml::Value::Array(funcs_arr));
        }
        if !f.components.is_empty() {
            let mut comps_arr: Vec<toml::Value> = Vec::new();
            for comp in &f.components {
                let mut comp_tbl = toml::Table::new();
                comp_tbl.insert("name".to_string(), toml::Value::String(comp.name.clone()));
                if let Some(ref export) = comp.export_name {
                    comp_tbl.insert("export".to_string(), toml::Value::String(export.clone()));
                }
                if let Some(ref summary) = comp.summary {
                    comp_tbl.insert("summary".to_string(), toml::Value::String(summary.clone()));
                }
                if let Some(ref confidence) = comp.confidence {
                    comp_tbl.insert(
                        "confidence".to_string(),
                        toml::Value::String(confidence.to_string()),
                    );
                }
                comps_arr.push(toml::Value::Table(comp_tbl));
            }
            file_entry.insert("components".to_string(), toml::Value::Array(comps_arr));
        }
        if !f.imports.is_empty() {
            let imports_arr: Vec<toml::Value> = f
                .imports
                .iter()
                .map(|s| toml::Value::String(s.clone()))
                .collect();
            file_entry.insert("imports".to_string(), toml::Value::Array(imports_arr));
        }
        if !f.related_hooks.is_empty() {
            let hooks_arr: Vec<toml::Value> = f
                .related_hooks
                .iter()
                .map(|s| toml::Value::String(s.clone()))
                .collect();
            file_entry.insert("related_hooks".to_string(), toml::Value::Array(hooks_arr));
        }
        if !f.related_components.is_empty() {
            let comps_arr: Vec<toml::Value> = f
                .related_components
                .iter()
                .map(|s| toml::Value::String(s.clone()))
                .collect();
            file_entry.insert(
                "related_components".to_string(),
                toml::Value::Array(comps_arr),
            );
        }
        if !f.imported_by.is_empty() {
            let imported_by_arr: Vec<toml::Value> = f
                .imported_by
                .iter()
                .map(|s| toml::Value::String(s.clone()))
                .collect();
            file_entry.insert(
                "imported_by".to_string(),
                toml::Value::Array(imported_by_arr),
            );
        }
        file_entry.insert(
            "hotspot_score".to_string(),
            toml::Value::Integer(f.imported_by.len() as i64),
        );
        if let Some(ref provides_ctx) = f.provides_context {
            file_entry.insert(
                "provides_context".to_string(),
                toml::Value::String(provides_ctx.clone()),
            );
        }
        if !f.consumes_context.is_empty() {
            let consumes_arr: Vec<toml::Value> = f
                .consumes_context
                .iter()
                .map(|s| toml::Value::String(s.clone()))
                .collect();
            file_entry.insert(
                "consumes_context".to_string(),
                toml::Value::Array(consumes_arr),
            );
        }
        if !f.exports.is_empty() {
            let mut exports_arr: Vec<toml::Value> = Vec::new();
            for exp in &f.exports {
                let mut exp_tbl = toml::Table::new();
                exp_tbl.insert("name".to_string(), toml::Value::String(exp.name.clone()));
                exp_tbl.insert(
                    "kind".to_string(),
                    toml::Value::String(format!("{:?}", exp.kind)),
                );
                exp_tbl.insert(
                    "signature".to_string(),
                    toml::Value::String(exp.signature.clone()),
                );
                exp_tbl.insert(
                    "line_start".to_string(),
                    toml::Value::Integer(exp.line_start as i64),
                );
                exp_tbl.insert(
                    "line_end".to_string(),
                    toml::Value::Integer(exp.line_end as i64),
                );
                exports_arr.push(toml::Value::Table(exp_tbl));
            }
            file_entry.insert("exports".to_string(), toml::Value::Array(exports_arr));
        }
        if !f.calls.is_empty() {
            let mut calls_arr: Vec<toml::Value> = Vec::new();
            for call in &f.calls {
                let mut call_tbl = toml::Table::new();
                call_tbl.insert(
                    "target".to_string(),
                    toml::Value::String(call.target.clone()),
                );
                call_tbl.insert(
                    "confidence".to_string(),
                    toml::Value::String(call.confidence.to_gua_string()),
                );
                call_tbl.insert(
                    "call_type".to_string(),
                    toml::Value::String(format!("{:?}", call.call_type)),
                );
                if let Some(ref note) = call.heuristic_note {
                    call_tbl.insert(
                        "heuristic_note".to_string(),
                        toml::Value::String(note.clone()),
                    );
                }
                calls_arr.push(toml::Value::Table(call_tbl));
            }
            file_entry.insert("calls".to_string(), toml::Value::Array(calls_arr));
        }
        if let Some(ref provides_ctx) = f.provides_context {
            let consumed_by = context_graph.get_consumed_by(provides_ctx);
            if !consumed_by.is_empty() {
                let mut consumed_by_arr: Vec<toml::Value> = Vec::new();
                for consumer_file in &consumed_by {
                    let mut consumer_tbl = toml::Table::new();
                    consumer_tbl.insert(
                        "file".to_string(),
                        toml::Value::String(consumer_file.clone()),
                    );
                    consumed_by_arr.push(toml::Value::Table(consumer_tbl));
                }
                file_entry.insert(
                    "consumed_by".to_string(),
                    toml::Value::Array(consumed_by_arr),
                );
            }
        }
        files_tbl.insert(f.path.clone(), toml::Value::Table(file_entry));
    }
    tbl.insert("files".to_string(), toml::Value::Table(files_tbl));
    let output_filename = format!("{}-{}.toml", source_name, nato_version);
    let output_path = output_dir.join("table_contents").join(&output_filename);
    fs::create_dir_all(output_path.parent().unwrap())?;
    let content = toml::to_string_pretty(&tbl)?;
    fs::write(&output_path, content)?;
    Ok(())
}

pub fn write_schema_json(
    file_entities: &[FileEntity],
    communities: &[FileCommunity],
    source_name: &str,
    nato_version: &str,
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut files_map = HashMap::new();

    for entity in file_entities {
        let exports: Vec<ExportSchema> = entity
            .exports
            .iter()
            .map(|e| ExportSchema {
                name: e.name.clone(),
                kind: format!("{:?}", e.kind),
                signature: e.signature.clone(),
                parameters: vec![],
                return_type: None,
                line_start: e.line_start,
                line_end: e.line_end,
            })
            .collect();

        let calls: Vec<CallSchema> = entity
            .calls
            .iter()
            .map(|c| CallSchema {
                target: c.target.clone(),
                confidence: c.confidence.to_gua_string(),
                call_type: format!("{:?}", c.call_type),
                heuristic_note: c.heuristic_note.clone(),
            })
            .collect();

        files_map.insert(
            entity.path.clone(),
            FileSchema {
                provides_context: entity.provides_context.clone(),
                consumes_context: entity.consumes_context.clone(),
                exports,
                calls,
                imported_by: entity.imported_by.clone(),
                hotspot_score: entity.imported_by.len() as i64,
            },
        );
    }

    let communities_schema: Vec<CommunitySchema> = communities
        .iter()
        .map(|c| CommunitySchema {
            id: c.id,
            files: c.files.clone(),
            size: c.size,
        })
        .collect();

    let schema = SchemaJson {
        schema_version: "3.1".to_string(),
        meta: MetaSection {
            generated_at: chrono::Utc::now().to_rfc3339(),
            source: source_name.to_string(),
            nato: nato_version.to_string(),
        },
        communities: communities_schema,
        files: files_map,
    };

    let output_filename = format!("{}-{}.schema.json", source_name, nato_version);
    let output_path = output_dir.join("table_contents").join(&output_filename);

    let content = serde_json::to_string_pretty(&schema)?;
    fs::write(&output_path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::ContextGraph;
    use crate::types::{Confidence, EnhancedFunction, FileEntity};
    use tempfile::TempDir;

    #[test]
    fn test_write_rich_toml() {
        let files = vec![FileEntity {
            path: "src/pages/Dashboard.tsx".to_string(),
            file_type: "page".to_string(),
            lines: 500,
            complexity: "HIGH".to_string(),
            functions: vec![EnhancedFunction {
                name: "Dashboard".to_string(),
                file: "src/pages/Dashboard.tsx".to_string(),
                line_start: 91,
                line_end: 200,
                export_name: None,
                summary: Some("Main component - Admin dashboard".to_string()),
                confidence: Some(Confidence::High),
            }],
            components: vec![],
            imports: vec!["useAuth".to_string(), "Modal".to_string()],
            related_hooks: vec!["useAuth".to_string()],
            related_components: vec!["Modal".to_string()],
            exports: vec![],
            calls: vec![],
            provides_context: None,
            consumes_context: vec![],
            imported_by: vec!["Navbar.tsx".to_string()],
        }];

        let temp_dir = TempDir::new().unwrap();
        let graph = ContextGraph::new();
        write_rich_toml(&files, &[], &graph, "frontend", "alpha", temp_dir.path()).unwrap();

        let output_path = temp_dir
            .path()
            .join("table_contents")
            .join("frontend-alpha.toml");
        let content = fs::read_to_string(&output_path).unwrap();

        assert!(content.contains("schema_version = \"3.1\""));
        assert!(content.contains("source = \"frontend\""));
        assert!(content.contains("\"src/pages/Dashboard.tsx\""));
        assert!(content.contains("complexity = \"HIGH\""));
        assert!(content.contains("Dashboard"));
        assert!(content.contains("related_hooks"));
    }
}
