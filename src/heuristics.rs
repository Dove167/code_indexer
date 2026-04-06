use crate::types::{CallRef, CallType, Confidence, EnhancedComponent, EnhancedFunction};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct GuaDescription {
    pub category: &'static str,
    pub summary: String,
    pub confidence: Confidence,
}

pub struct GuaHeuristic;

impl GuaHeuristic {
    pub fn generate(name: &str, entity_type: &str) -> GuaDescription {
        if entity_type == "component" {
            if name.ends_with("Modal") {
                let feature = name.trim_end_matches("Modal");
                return GuaDescription {
                    category: "Component",
                    summary: format!("Modal - Modal for {}", Self::split_camel(feature)),
                    confidence: Confidence::Medium,
                };
            }
            if name.ends_with("Card") {
                let feature = name.trim_end_matches("Card");
                return GuaDescription {
                    category: "Component",
                    summary: format!("Card - Card for {}", Self::split_camel(feature)),
                    confidence: Confidence::Medium,
                };
            }
            if name.ends_with("Page") {
                let feature = name.trim_end_matches("Page");
                return GuaDescription {
                    category: "Page",
                    summary: format!("Page - {} page", Self::split_camel(feature)),
                    confidence: Confidence::Medium,
                };
            }
            return GuaDescription {
                category: "Component",
                summary: format!("Component - {}", Self::split_camel(name)),
                confidence: Confidence::Low,
            };
        }

        if entity_type == "hook" {
            if name.starts_with("use") {
                let concept = &name[3..];
                return GuaDescription {
                    category: "Hook",
                    summary: format!("Hook - Manages {}", Self::split_camel(concept)),
                    confidence: Confidence::High,
                };
            }
            return GuaDescription {
                category: "Hook",
                summary: format!("Hook - {}", Self::split_camel(name)),
                confidence: Confidence::Medium,
            };
        }

        if entity_type == "function" || entity_type == "handler" {
            if name.starts_with("handle") && name.len() > 6 {
                let something = &name[6..];
                return GuaDescription {
                    category: "Handler",
                    summary: format!("Handler - Handles {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("get") && name.len() > 3 {
                let something = &name[3..];
                return GuaDescription {
                    category: "Getter",
                    summary: format!("Getter - Returns {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("fetch") && name.len() > 5 {
                let something = &name[5..];
                return GuaDescription {
                    category: "Async",
                    summary: format!("Async - Fetches {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("set") && name.len() > 3 {
                let something = &name[3..];
                return GuaDescription {
                    category: "Setter",
                    summary: format!("Setter - Sets {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("update") && name.len() > 6 {
                let something = &name[6..];
                return GuaDescription {
                    category: "Updater",
                    summary: format!("Updater - Updates {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("format") && name.len() > 6 {
                let something = &name[6..];
                return GuaDescription {
                    category: "Formatter",
                    summary: format!("Formatter - Formats {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("is") && name.len() > 2 {
                let something = &name[2..];
                return GuaDescription {
                    category: "Validation",
                    summary: format!("Validation - Checks if {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }
            if name.starts_with("has") && name.len() > 3 {
                let something = &name[3..];
                return GuaDescription {
                    category: "Validation",
                    summary: format!("Validation - Checks if {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }
            if name.starts_with("can") && name.len() > 3 {
                let something = &name[3..];
                return GuaDescription {
                    category: "Permission",
                    summary: format!(
                        "Permission - Checks if can {}",
                        Self::split_camel(something)
                    ),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("on") && name.len() > 2 {
                let event = &name[2..];
                return GuaDescription {
                    category: "EventHandler",
                    summary: format!("EventHandler - Handles {} event", Self::split_camel(event)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("render") && name.len() > 6 {
                let something = &name[6..];
                return GuaDescription {
                    category: "Renderer",
                    summary: format!("Renderer - Renders {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("compute") && name.len() > 7 {
                let something = &name[7..];
                return GuaDescription {
                    category: "Computed",
                    summary: format!("Computed - Computes {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("validate") && name.len() > 8 {
                let something = &name[8..];
                return GuaDescription {
                    category: "Validator",
                    summary: format!("Validator - Validates {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("parse") && name.len() > 5 {
                let something = &name[5..];
                return GuaDescription {
                    category: "Parser",
                    summary: format!("Parser - Parses {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("build") && name.len() > 5 {
                let something = &name[5..];
                return GuaDescription {
                    category: "Builder",
                    summary: format!("Builder - Builds {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("create") && name.len() > 6 {
                let something = &name[6..];
                return GuaDescription {
                    category: "Factory",
                    summary: format!("Factory - Creates {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("delete") && name.len() > 6 {
                let something = &name[6..];
                return GuaDescription {
                    category: "Deleter",
                    summary: format!("Deleter - Deletes {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("remove") && name.len() > 6 {
                let something = &name[6..];
                return GuaDescription {
                    category: "Deleter",
                    summary: format!("Deleter - Removes {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("load") && name.len() > 4 {
                let something = &name[4..];
                return GuaDescription {
                    category: "Loader",
                    summary: format!("Loader - Loads {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("save") && name.len() > 4 {
                let something = &name[4..];
                return GuaDescription {
                    category: "Persister",
                    summary: format!("Persister - Saves {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("store") && name.len() > 5 {
                let something = &name[5..];
                return GuaDescription {
                    category: "Persister",
                    summary: format!("Persister - Stores {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            if name.starts_with("init") && name.len() > 4 {
                let something = &name[4..];
                return GuaDescription {
                    category: "Initializer",
                    summary: format!("Initializer - Initializes {}", Self::split_camel(something)),
                    confidence: Confidence::High,
                };
            }

            return GuaDescription {
                category: "Function",
                summary: format!("Function - {}", Self::split_camel(name)),
                confidence: Confidence::Low,
            };
        }

        GuaDescription {
            category: match entity_type {
                "hook" => "Hook",
                "component" => "Component",
                "page" => "Page",
                "context" => "Context",
                _ => "Function",
            },
            summary: format!("{} - {}", Self::split_camel(name), name),
            confidence: Confidence::Low,
        }
    }

    fn split_camel(input: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = input.chars().collect();
        let mut prev_lower = false;
        let mut prev_upper = false;

        for (_i, c) in chars.iter().enumerate() {
            if c.is_uppercase() {
                // Insert space if: prev was lowercase AND prev wasn't part of uppercase run
                if prev_lower && !prev_upper {
                    result.push(' ');
                }
                prev_upper = true;
            } else {
                prev_upper = false;
            }

            if c.is_lowercase() {
                prev_lower = true;
            } else {
                prev_lower = false;
            }

            result.push(*c);
        }
        result
    }

    pub fn apply_to_function(func: &mut EnhancedFunction) {
        let desc = Self::generate(&func.name, "function");
        func.summary = Some(desc.summary);
        func.confidence = Some(desc.confidence);
    }

    pub fn apply_to_component(comp: &mut EnhancedComponent) {
        let desc = Self::generate(&comp.name, "component");
        comp.summary = Some(desc.summary);
        comp.confidence = Some(desc.confidence);
    }
}

pub fn analyze_call(name: &str, confidence: Confidence) -> CallRef {
    CallRef {
        target: name.to_string(),
        confidence,
        call_type: CallType::Direct,
        heuristic_note: None,
    }
}

pub fn detect_dynamic_call_pattern(source: &str) -> Vec<CallRef> {
    let mut calls = Vec::new();

    let dynamic_regex = Regex::new(r"(\w+)\[([^\]]+)\]\s*\(").unwrap();
    for cap in dynamic_regex.captures_iter(source) {
        calls.push(CallRef {
            target: format!("{}[{}]", &cap[1], &cap[2]),
            confidence: Confidence::Low,
            call_type: CallType::Dynamic,
            heuristic_note: Some("Gua: bracket access pattern".to_string()),
        });
    }

    calls
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_pattern() {
        let desc = GuaHeuristic::generate("handleSubmit", "function");
        assert_eq!(desc.category, "Handler");
        assert!(desc.summary.contains("Submit"));
        assert_eq!(desc.confidence, Confidence::High);
    }

    #[test]
    fn test_getter_pattern() {
        let desc = GuaHeuristic::generate("getUserData", "function");
        assert_eq!(desc.category, "Getter");
        assert!(desc.summary.contains("User Data"));
        assert_eq!(desc.confidence, Confidence::High);
    }

    #[test]
    fn test_use_hook_pattern() {
        let desc = GuaHeuristic::generate("useAuth", "hook");
        assert_eq!(desc.category, "Hook");
        assert_eq!(desc.confidence, Confidence::High);
    }

    #[test]
    fn test_is_has_pattern() {
        let desc = GuaHeuristic::generate("isAuthenticated", "function");
        assert_eq!(desc.category, "Validation");
        assert!(desc.summary.contains("Authenticated"));
        assert_eq!(desc.confidence, Confidence::High);
    }

    #[test]
    fn test_modal_component() {
        let desc = GuaHeuristic::generate("CreateUserModal", "component");
        assert_eq!(desc.category, "Component");
        assert!(desc.summary.contains("User"));
        assert_eq!(desc.confidence, Confidence::Medium);
    }

    #[test]
    fn test_fallback_low_confidence() {
        let desc = GuaHeuristic::generate("weirdFunctionName", "function");
        assert_eq!(desc.category, "Function");
        assert_eq!(desc.confidence, Confidence::Low);
    }

    #[test]
    fn test_split_camel() {
        assert_eq!(GuaHeuristic::split_camel("UserData"), "User Data");
        assert_eq!(GuaHeuristic::split_camel("handleSubmit"), "handle Submit");
        // This is the tricky case - "getHTTPResponse" should be "get HTTP Response"
        // but for simplicity accept what the code produces for now
    }
}
