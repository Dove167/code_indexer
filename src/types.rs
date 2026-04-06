#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub file_type: String,
    pub lines: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    pub export_name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComponentInfo {
    pub name: String,
    pub file: String,
    pub export_name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportInfo {
    pub file: String,
    pub import_name: String,
    pub import_path: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Confidence {
    High,
    Medium,
    Low,
    Unknown,
}

impl Confidence {
    pub fn to_gua_string(&self) -> String {
        match self {
            Confidence::High => "HIGH".to_string(),
            Confidence::Medium => "MEDIUM".to_string(),
            Confidence::Low => "LOW?".to_string(),
            Confidence::Unknown => "UNKNOWN?".to_string(),
        }
    }
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::High => write!(f, "HIGH"),
            Confidence::Medium => write!(f, "MEDIUM"),
            Confidence::Low => write!(f, "LOW"),
            Confidence::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnhancedFunction {
    pub name: String,
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    pub export_name: Option<String>,
    pub summary: Option<String>,
    pub confidence: Option<Confidence>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnhancedComponent {
    pub name: String,
    pub file: String,
    pub export_name: Option<String>,
    pub summary: Option<String>,
    pub confidence: Option<Confidence>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExportKind {
    Function,
    Const,
    Type,
    Context,
    Component,
    Hook,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CallType {
    Direct,
    Indirect,
    Dynamic,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportSymbol {
    pub name: String,
    pub kind: ExportKind,
    pub signature: String,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CallRef {
    pub target: String,
    pub confidence: Confidence,
    pub call_type: CallType,
    pub heuristic_note: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FileEntity {
    pub path: String,
    pub file_type: String,
    pub lines: usize,
    pub complexity: String,
    pub functions: Vec<EnhancedFunction>,
    pub components: Vec<EnhancedComponent>,
    pub imports: Vec<String>,
    pub related_hooks: Vec<String>,
    pub related_components: Vec<String>,
    pub exports: Vec<ExportSymbol>,
    pub calls: Vec<CallRef>,
    pub provides_context: Option<String>,
    pub consumes_context: Vec<String>,
    pub imported_by: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileCommunity {
    pub id: usize,
    pub files: Vec<String>,
    pub size: usize,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub sources: std::collections::HashMap<String, SourceManifest>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SourceManifest {
    pub versions: std::collections::HashMap<String, VersionInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionInfo {
    pub timestamp: String,
    pub files: usize,
    pub path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_ordering() {
        assert!(matches!(
            Confidence::High.cmp(&Confidence::Medium),
            std::cmp::Ordering::Greater
        ));
        assert!(matches!(
            Confidence::Medium.cmp(&Confidence::Low),
            std::cmp::Ordering::Greater
        ));
    }

    #[test]
    fn test_confidence_display() {
        assert_eq!(Confidence::High.to_string(), "HIGH");
        assert_eq!(Confidence::Medium.to_string(), "MEDIUM");
        assert_eq!(Confidence::Low.to_string(), "LOW");
    }

    #[test]
    fn test_confidence_gua_markers() {
        assert_eq!(Confidence::High.to_gua_string(), "HIGH");
        assert_eq!(Confidence::Low.to_gua_string(), "LOW?");
        assert_eq!(Confidence::Unknown.to_gua_string(), "UNKNOWN?");
    }

    #[test]
    fn test_export_kind_variants() {
        let kinds = vec![ExportKind::Function, ExportKind::Context, ExportKind::Hook];
        assert_eq!(kinds.len(), 3);
    }
}
