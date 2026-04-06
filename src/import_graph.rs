use std::collections::{HashMap, HashSet};

use single_clustering::community_search::leiden::partition::{
    ModularityPartition, VertexPartition,
};
use single_clustering::community_search::leiden::{LeidenConfig, LeidenOptimizer};
use single_clustering::network::grouping::VectorGrouping;
use single_clustering::network::CSRNetwork;

use crate::types::{FileCommunity, FileEntity};

pub type ImportGraph = HashMap<String, Vec<String>>;
pub type DependentGraph = HashMap<String, Vec<String>>;

fn normalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

fn strip_extension(path: &str) -> &str {
    path.strip_suffix(".tsx")
        .or(path.strip_suffix(".ts"))
        .or(path.strip_suffix(".jsx"))
        .or(path.strip_suffix(".js"))
        .unwrap_or(path)
}

fn resolve_import_to_file(
    import_path: &str,
    importer_file: &str,
    entities: &[FileEntity],
) -> Option<String> {
    if !import_path.starts_with('.') {
        return None;
    }

    let importer_dir = importer_file
        .rsplit_once('/')
        .map(|(dir, _)| dir)
        .unwrap_or("");

    let joined = if importer_dir.is_empty() {
        import_path.to_string()
    } else {
        format!("{}/{}", importer_dir, import_path)
    };

    let base = normalize_path(&joined);

    let extensions = [".tsx", ".ts", ".jsx", ".js", "/index.tsx", "/index.ts", ""];
    for ext in &extensions {
        let candidate = if ext.is_empty() {
            base.clone()
        } else {
            format!("{}{}", base, ext)
        };
        let candidate = normalize_path(&candidate);
        for entity in entities {
            if entity.path == candidate
                || strip_extension(&entity.path) == strip_extension(&candidate)
            {
                return Some(entity.path.clone());
            }
        }
    }

    None
}

pub fn build_import_graph(
    file_entities: &[FileEntity],
) -> (ImportGraph, DependentGraph, HashMap<String, usize>) {
    let mut forward: ImportGraph = HashMap::new();
    let mut reverse: DependentGraph = HashMap::new();
    let mut hotspot_map: HashMap<String, usize> = HashMap::new();

    for entity in file_entities {
        forward.insert(entity.path.clone(), vec![]);
        reverse.insert(entity.path.clone(), vec![]);
        hotspot_map.insert(entity.path.clone(), 0);
    }

    for entity in file_entities {
        for import_path in &entity.imports {
            if let Some(target) = resolve_import_to_file(import_path, &entity.path, file_entities) {
                if let Some(fwd) = forward.get_mut(&entity.path) {
                    if !fwd.contains(&target) {
                        fwd.push(target.clone());
                    }
                }
                if let Some(rev) = reverse.get_mut(&target) {
                    if !rev.contains(&entity.path) {
                        rev.push(entity.path.clone());
                        *hotspot_map.entry(target.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    (forward, reverse, hotspot_map)
}

pub fn get_imported_by(reverse_graph: &DependentGraph, target: &str) -> Vec<String> {
    reverse_graph.get(target).cloned().unwrap_or_default()
}

pub fn get_hotspot_score(hotspot_map: &HashMap<String, usize>, target: &str) -> usize {
    hotspot_map.get(target).copied().unwrap_or(0)
}

fn normalize_import_module(import: &str) -> String {
    let normalized = import.trim_start_matches("./").trim_start_matches("../");
    let normalized = normalized
        .strip_suffix(".tsx")
        .or(normalized.strip_suffix(".ts"))
        .or(normalized.strip_suffix(".jsx"))
        .or(normalized.strip_suffix(".js"))
        .unwrap_or(normalized);
    normalized.replace("/", "_")
}

fn compute_jaccard(set1: &HashSet<String>, set2: &HashSet<String>) -> f64 {
    if set1.is_empty() && set2.is_empty() {
        return 0.0;
    }
    let intersection = set1.intersection(set2).count() as f64;
    let union = set1.union(set2).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn extract_keywords(files: &[String]) -> Vec<String> {
    let mut keywords: HashSet<String> = HashSet::new();
    for path in files {
        let parts: Vec<&str> = path.split('/').collect();
        for part in parts {
            if !part.is_empty()
                && part != "src"
                && !part.starts_with(char::is_numeric)
                && !part.ends_with(".tsx")
                && !part.ends_with(".ts")
                && !part.ends_with(".jsx")
                && !part.ends_with(".js")
            {
                let cleaned = part.replace('-', " ").replace('_', " ");
                if !cleaned.is_empty() && cleaned.len() > 1 {
                    keywords.insert(cleaned.to_lowercase());
                }
            }
        }
    }
    keywords.into_iter().collect()
}

fn determine_area(files: &[String]) -> String {
    let mut area_counts: HashMap<&str, usize> = HashMap::new();
    for path in files {
        if path.contains("/admin/") {
            *area_counts.entry("admin").or_insert(0) += 1;
        } else if path.contains("/client/") {
            *area_counts.entry("client").or_insert(0) += 1;
        } else if path.contains("/shared/") {
            *area_counts.entry("shared").or_insert(0) += 1;
        }
    }
    area_counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(area, _)| area.to_string())
        .unwrap_or_else(|| "app".to_string())
}

fn determine_type(files: &[String]) -> String {
    let mut type_counts: HashMap<&str, usize> = HashMap::new();
    for path in files {
        if path.contains("/components/") || path.ends_with(".tsx") {
            if path.contains("/hooks/") {
                *type_counts.entry("hooks").or_insert(0) += 1;
            } else if path.contains("/pages/") {
                *type_counts.entry("pages").or_insert(0) += 1;
            } else if path.contains("/utils/") {
                *type_counts.entry("utils").or_insert(0) += 1;
            } else if path.contains("/types/") {
                *type_counts.entry("types").or_insert(0) += 1;
            } else if path.contains("/lib/") {
                *type_counts.entry("lib").or_insert(0) += 1;
            } else if path.contains("/routes/") {
                *type_counts.entry("routes").or_insert(0) += 1;
            } else if path.contains("/contexts/") {
                *type_counts.entry("contexts").or_insert(0) += 1;
            } else {
                *type_counts.entry("components").or_insert(0) += 1;
            }
        } else if path.ends_with(".ts") {
            if path.contains("/hooks/") {
                *type_counts.entry("hooks").or_insert(0) += 1;
            } else if path.contains("/utils/") {
                *type_counts.entry("utils").or_insert(0) += 1;
            } else if path.contains("/types/") {
                *type_counts.entry("types").or_insert(0) += 1;
            } else {
                *type_counts.entry("utils").or_insert(0) += 1;
            }
        }
    }
    type_counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(t, _)| t.to_string())
        .unwrap_or_else(|| "misc".to_string())
}

fn find_domain_keywords(files: &[String]) -> Vec<String> {
    let domain_terms = [
        "appointment",
        "document",
        "strata",
        "user",
        "survey",
        "timeline",
        "inspector",
        "availability",
        "company",
        "holiday",
        "booking",
        "property",
        "activation",
        "permission",
        "notification",
        "auth",
        "login",
        "password",
        "profile",
    ];
    let keywords = extract_keywords(files);
    domain_terms
        .iter()
        .filter(|term| keywords.iter().any(|k| k.contains(*term)))
        .map(|s| s.to_string())
        .collect()
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn name_community(files: &[String]) -> (Option<String>, Option<String>) {
    if files.is_empty() {
        return (None, None);
    }

    let area = determine_area(files);
    let file_type = determine_type(files);
    let domains = find_domain_keywords(files);

    let name = if domains.is_empty() {
        match file_type.as_str() {
            "components" => Some(format!("{} UI Components", area)),
            "hooks" => Some(format!("{} Hooks", area)),
            "pages" => Some(format!("{} Pages", area)),
            "utils" => Some(format!("{} Utilities", area)),
            "types" => Some(format!("{} Types", area)),
            "lib" => Some(format!("{} Library", area)),
            "routes" => Some(format!("{} Routing", area)),
            "contexts" => Some(format!("{} Contexts", area)),
            _ => Some(format!("{} {}", area, file_type)),
        }
    } else if domains.len() == 1 {
        Some(format!("{} {}", area, domains[0]))
    } else {
        let primary = domains
            .iter()
            .take(2)
            .cloned()
            .collect::<Vec<_>>()
            .join(" & ");
        Some(format!("{} {}", area, primary))
    };

    let description = if domains.is_empty() {
        Some(format!("{} {} files", capitalize_first(&area), file_type))
    } else {
        Some(format!(
            "{} functionality related to {}",
            capitalize_first(&area),
            domains.join(", ")
        ))
    };

    (name, description)
}

pub fn find_connected_components(
    files: &[FileEntity],
    _import_graph: &ImportGraph,
    _reverse_graph: &DependentGraph,
) -> Vec<FileCommunity> {
    let node_count = files.len();
    if node_count == 0 {
        return vec![];
    }

    let import_sets: Vec<HashSet<String>> = files
        .iter()
        .map(|entity| {
            entity
                .imports
                .iter()
                .map(|imp| normalize_import_module(imp))
                .collect()
        })
        .collect();

    let mut edges: Vec<(usize, usize, f64)> = Vec::new();
    const JACCARD_THRESHOLD: f64 = 0.08;
    const MAX_NEIGHBORS: usize = 15;

    for i in 0..node_count {
        let mut similarities: Vec<(usize, f64)> = Vec::new();
        for j in 0..node_count {
            if i == j {
                continue;
            }
            let jaccard = compute_jaccard(&import_sets[i], &import_sets[j]);
            if jaccard > JACCARD_THRESHOLD {
                similarities.push((j, jaccard));
            }
        }

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        similarities.truncate(MAX_NEIGHBORS);

        for (j, sim) in similarities {
            let weight = 1.0 - sim;
            if weight > 0.0 {
                edges.push((i, j, weight));
            }
        }
    }

    if edges.is_empty() {
        return files
            .iter()
            .enumerate()
            .map(|(i, entity)| {
                let file_path = vec![entity.path.clone()];
                let (name, description) = name_community(&file_path);
                FileCommunity {
                    id: i,
                    files: file_path,
                    size: 1,
                    name,
                    description,
                }
            })
            .collect();
    }

    let node_weights = vec![1.0; node_count];
    let network = CSRNetwork::from_edges(&edges, node_weights);

    let config = LeidenConfig {
        max_iterations: 100,
        tolerance: 1e-6,
        seed: Some(42),
        ..Default::default()
    };

    let mut optimizer = LeidenOptimizer::new(config);
    let partition: ModularityPartition<f64, _> = optimizer
        .find_partition::<f64, VectorGrouping, _>(network)
        .expect("Leiden clustering failed");

    let mut community_map: HashMap<usize, Vec<String>> = HashMap::new();
    for (i, entity) in files.iter().enumerate() {
        let community_id = partition.membership(i);
        community_map
            .entry(community_id)
            .or_default()
            .push(entity.path.clone());
    }

    let mut communities: Vec<FileCommunity> = community_map
        .into_iter()
        .enumerate()
        .map(|(idx, (_raw_id, files))| {
            let size = files.len();
            let (name, description) = name_community(&files);
            FileCommunity {
                id: idx,
                files,
                size,
                name,
                description,
            }
        })
        .collect();

    communities.sort_by_key(|c| c.size);
    communities.reverse();
    for (i, c) in communities.iter_mut().enumerate() {
        c.id = i;
    }

    communities
}
