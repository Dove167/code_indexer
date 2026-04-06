use regex::Regex;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use tree_sitter_typescript::LANGUAGE_TYPESCRIPT;

const EXPORT_QUERY: &str = r#"
    (export_statement
      declaration: (function_declaration
        name: (identifier) @name
      ) @func
    )
    (export_statement
      declaration: (lexical_declaration
        (variable_declarator
          name: (identifier) @name
        ) @decl
      )
    )
"#;

const CONTEXT_QUERY: &str = r#"
    (variable_declarator
      name: (identifier) @name
      value: (call_expression
        function: (identifier) @fn
      )
    )
"#;

const USE_CONTEXT_QUERY: &str = r#"
    (call_expression
      function: (identifier) @name
      arguments: (arguments
        (identifier) @ctx
      )
    )
"#;

const CALL_QUERY: &str = r#"
    (call_expression
      function: (identifier) @name
    )
"#;

pub struct TsParser {
    parser: Parser,
}

impl TsParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        let language: tree_sitter::Language = LANGUAGE_TYPESCRIPT.into();
        parser
            .set_language(&language)
            .expect("Failed to set TSX language");
        Self { parser }
    }

    pub fn parse(&mut self, source: &str) -> tree_sitter::Tree {
        self.parser.parse(source, None).expect("Failed to parse")
    }
}

pub fn extract_exports(source: &str) -> Vec<crate::types::ExportSymbol> {
    let mut parser = TsParser::new();
    let tree = parser.parse(source);
    let mut exports = Vec::new();

    let mut cursor = QueryCursor::new();
    let language: tree_sitter::Language = LANGUAGE_TYPESCRIPT.into();
    let query = Query::new(&language, EXPORT_QUERY).unwrap();

    let mut matches_iter = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(m) = matches_iter.next() {
        for cap in m.captures {
            if cap.index == 0 {
                let name = get_node_text(cap.node, source);
                exports.push(crate::types::ExportSymbol {
                    name,
                    kind: crate::types::ExportKind::Function,
                    signature: extract_signature(&cap.node, source),
                    line_start: cap.node.start_position().row + 1,
                    line_end: cap.node.end_position().row + 1,
                });
            }
        }
    }

    let func_regex = Regex::new(r"(?m)^export\s+(?:function|const|let|var)\s+(\w+)").unwrap();
    for cap in func_regex.captures_iter(source) {
        let name = cap[1].to_string();
        if exports.iter().any(|e| e.name == name) {
            continue;
        }
        let match_start = cap.get(0).unwrap().start();
        let line_num = source[..match_start].chars().filter(|&c| c == '\n').count() + 1;
        let line_text = source.lines().nth(line_num - 1).unwrap_or("").to_string();

        exports.push(crate::types::ExportSymbol {
            name,
            kind: crate::types::ExportKind::Function,
            signature: line_text,
            line_start: line_num,
            line_end: line_num,
        });
    }

    exports
}

fn detect_export_kind(node: &tree_sitter::Node) -> crate::types::ExportKind {
    match node.kind() {
        "function_declaration" => crate::types::ExportKind::Function,
        "lexical_declaration" => crate::types::ExportKind::Const,
        "type_alias_declaration" => crate::types::ExportKind::Type,
        _ => crate::types::ExportKind::Function,
    }
}

fn get_node_text(node: tree_sitter::Node, source: &str) -> String {
    let start = node.start_byte();
    let end = node.end_byte();
    source[start..end].to_string()
}

pub fn extract_signature(node: &tree_sitter::Node, source: &str) -> String {
    let start = node.start_position();
    let end = node.end_position();
    let lines: Vec<&str> = source.lines().collect();
    if start.row <= end.row && start.row < lines.len() {
        lines[start.row..end.row + 1].join("\n")
    } else {
        String::new()
    }
}

pub fn extract_provides_context(source: &str) -> Option<String> {
    let mut parser = TsParser::new();
    let tree = parser.parse(source);
    let mut cursor = QueryCursor::new();
    let language: tree_sitter::Language = LANGUAGE_TYPESCRIPT.into();
    let query = match Query::new(&language, CONTEXT_QUERY) {
        Ok(q) => q,
        Err(_) => {
            let regex = Regex::new(r"(\w+)\s*=\s*createContext[<(]").ok()?;
            return regex.captures(source).map(|c| c[1].to_string());
        }
    };

    let mut matches_iter = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(m) = matches_iter.next() {
        let mut var_name: Option<String> = None;
        let mut fn_name: Option<String> = None;

        for cap in m.captures {
            match cap.index {
                0 => var_name = Some(get_node_text(cap.node, source)),
                1 => fn_name = Some(get_node_text(cap.node, source)),
                _ => {}
            }
        }
        if fn_name.as_deref() == Some("createContext") {
            if let Some(name) = var_name {
                return Some(name);
            }
        }
    }

    let regex = Regex::new(r"(\w+)\s*=\s*createContext[<(]").ok()?;
    let cap = regex.captures(source)?;
    Some(cap[1].to_string())
}

pub fn extract_consumed_contexts(source: &str) -> Vec<String> {
    let mut parser = TsParser::new();
    let tree = parser.parse(source);
    let mut cursor = QueryCursor::new();
    let language: tree_sitter::Language = LANGUAGE_TYPESCRIPT.into();
    let mut contexts = Vec::new();

    let query = match Query::new(&language, USE_CONTEXT_QUERY) {
        Ok(q) => q,
        Err(_) => {
            let regex = Regex::new(r"useContext\s*<\s*(\w+)\s*>").ok();
            if let Some(re) = regex {
                for cap in re.captures_iter(source) {
                    contexts.push(cap[1].to_string());
                }
            }
            contexts.dedup();
            return contexts;
        }
    };

    let mut matches_iter = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(m) = matches_iter.next() {
        let mut ctx_name: Option<String> = None;
        let mut fn_name: Option<String> = None;

        for cap in m.captures {
            match cap.index {
                0 => fn_name = Some(get_node_text(cap.node, source)),
                1 => ctx_name = Some(get_node_text(cap.node, source)),
                _ => {}
            }
        }
        if fn_name.as_deref() == Some("useContext") {
            if let Some(name) = ctx_name {
                contexts.push(name);
            }
        }
    }

    let regex = Regex::new(r"useContext\s*<\s*(\w+)\s*>").ok();
    if let Some(re) = regex {
        for cap in re.captures_iter(source) {
            contexts.push(cap[1].to_string());
        }
    }

    contexts.dedup();
    contexts
}

pub fn extract_calls(source: &str) -> Vec<crate::types::CallRef> {
    let mut calls = Vec::new();
    let mut parser = TsParser::new();
    let tree = parser.parse(source);
    let mut cursor = QueryCursor::new();
    let language: tree_sitter::Language = LANGUAGE_TYPESCRIPT.into();
    let query = Query::new(&language, CALL_QUERY).unwrap();

    let mut matches_iter = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(m) = matches_iter.next() {
        for cap in m.captures {
            if cap.index == 0 {
                let name = get_node_text(cap.node, source);
                if !name.starts_with("use") && !name.starts_with("set") {
                    calls.push(crate::types::CallRef {
                        target: name,
                        confidence: crate::types::Confidence::High,
                        call_type: crate::types::CallType::Direct,
                        heuristic_note: None,
                    });
                }
            }
        }
    }

    let dynamic_calls = crate::heuristics::detect_dynamic_call_pattern(source);
    calls.extend(dynamic_calls);

    calls
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let source = r#"
export function useAuth() {
    return context;
}
"#;
        let mut parser = TsParser::new();
        let tree = parser.parse(source);
        assert!(!tree.root_node().is_error());
    }

    #[test]
    fn test_extract_named_export() {
        let source = r#"
export function useAuth() {
    return authContext;
}
"#;
        let exports = extract_exports(source);
        assert!(exports.iter().any(|e| e.name == "useAuth"));
    }

    #[test]
    fn test_extract_context_provider() {
        let source = r#"
const AuthContext = createContext<AuthContextType | undefined>(undefined);
"#;
        let ctx = extract_provides_context(source);
        assert_eq!(ctx, Some("AuthContext".to_string()));
    }

    #[test]
    fn test_extract_consumed_context() {
        let source = r#"
const { user } = useAuth();
const theme = useContext(ThemeContext);
"#;
        let contexts = extract_consumed_contexts(source);
        assert!(contexts.contains(&"ThemeContext".to_string()));
    }

    #[test]
    fn test_dynamic_call_detection() {
        let source = r#"
const action = 'save';
api[action]();
"#;
        let calls = extract_calls(source);
        assert!(calls
            .iter()
            .any(|c| c.call_type == crate::types::CallType::Dynamic));
        assert_eq!(
            calls.first().unwrap().confidence,
            crate::types::Confidence::Low
        );
    }
}
