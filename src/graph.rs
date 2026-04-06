use std::collections::HashMap;

pub struct ContextGraph {
    providers: HashMap<String, Vec<String>>,
    consumers: HashMap<String, Vec<String>>,
}

impl ContextGraph {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            consumers: HashMap::new(),
        }
    }

    pub fn add_provider(&mut self, context: &str, file: &str) {
        self.providers
            .entry(context.to_string())
            .or_default()
            .push(file.to_string());
    }

    pub fn add_consumer(&mut self, context: &str, file: &str) {
        self.consumers
            .entry(context.to_string())
            .or_default()
            .push(file.to_string());
    }

    pub fn get_consumed_by(&self, context: &str) -> Vec<String> {
        self.consumers.get(context).cloned().unwrap_or_default()
    }
}

use crate::types::FileEntity;

pub fn build_context_graph(entities: &[FileEntity]) -> ContextGraph {
    let mut graph = ContextGraph::new();

    for entity in entities {
        if let Some(ctx) = &entity.provides_context {
            graph.add_provider(ctx, &entity.path);
        }
        for ctx in &entity.consumes_context {
            graph.add_consumer(ctx, &entity.path);
        }
    }

    graph
}

#[cfg(test)]
mod tests {
    use crate::types::{ExportKind, ExportSymbol, FileEntity};

    #[test]
    fn test_context_graph_build() {
        let entities = vec![
            FileEntity {
                path: "AuthContext.tsx".to_string(),
                provides_context: Some("AuthContext".to_string()),
                exports: vec![ExportSymbol {
                    name: "AuthContext".to_string(),
                    kind: ExportKind::Context,
                    signature: "createContext".to_string(),
                    line_start: 1,
                    line_end: 1,
                }],
                file_type: "tsx".to_string(),
                lines: 10,
                complexity: "low".to_string(),
                functions: vec![],
                components: vec![],
                imports: vec![],
                related_hooks: vec![],
                related_components: vec![],
                calls: vec![],
                consumes_context: vec![],
            },
            FileEntity {
                path: "Navbar.tsx".to_string(),
                consumes_context: vec!["AuthContext".to_string()],
                exports: vec![],
                file_type: "tsx".to_string(),
                lines: 20,
                complexity: "low".to_string(),
                functions: vec![],
                components: vec![],
                imports: vec![],
                related_hooks: vec![],
                related_components: vec![],
                calls: vec![],
                provides_context: None,
            },
        ];

        let graph = build_context_graph(&entities);
        let consumers = graph.get_consumed_by("AuthContext");
        assert!(consumers.contains(&"Navbar.tsx".to_string()));
    }
}
