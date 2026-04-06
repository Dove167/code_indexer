use crate::types::{Manifest, VersionInfo};
use std::fs;
use std::path::{Path, PathBuf};

const NATO_SEQUENCE: [&str; 24] = [
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "juliet",
    "kilo", "lima", "mike", "november", "oscar", "papa", "quebec", "romeo", "sierra", "tango",
    "uniform", "victor", "whiskey", "xray",
];

pub struct NatoSequence {
    index: usize,
    cycle: usize,
}

impl NatoSequence {
    pub fn new() -> Self {
        Self { index: 0, cycle: 0 }
    }

    pub fn next(&mut self) -> String {
        let name = NATO_SEQUENCE[self.index].to_string();
        self.index += 1;
        if self.index >= NATO_SEQUENCE.len() {
            self.index = 0;
            self.cycle += 1;
        }
        if self.cycle > 0 {
            format!("{}-{}", NATO_SEQUENCE[self.index].to_string(), self.cycle)
        } else {
            name
        }
    }

    pub fn from_string(s: &str) -> Option<(usize, usize)> {
        let parts: Vec<&str> = s.split('-').collect();
        let base = parts[0];
        let base_index = NATO_SEQUENCE.iter().position(|&x| x == base)?;
        if parts.len() == 1 {
            Some((base_index, 0))
        } else {
            let cycle: usize = parts[1].parse().ok()?;
            Some((base_index, cycle))
        }
    }
}

impl Default for NatoSequence {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ManifestManager {
    manifest_path: PathBuf,
    manifest: Manifest,
}

impl ManifestManager {
    pub fn new(manifest_path: PathBuf) -> Self {
        let manifest = Self::load_manifest(&manifest_path);
        Self {
            manifest_path,
            manifest,
        }
    }

    fn load_manifest(path: &Path) -> Manifest {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(m) = toml::from_str(&content) {
                    return m;
                }
            }
        }
        Manifest::default()
    }

    pub fn get_next_nato(&self, source: &str) -> String {
        if let Some(source_manifest) = self.manifest.sources.get(source) {
            let versions: Vec<_> = source_manifest.versions.keys().collect();
            let mut seq = NatoSequence::new();
            for v in versions {
                if let Some((idx, cyc)) = NatoSequence::from_string(v) {
                    while seq.index <= idx || seq.cycle < cyc {
                        seq.next();
                    }
                }
            }
            seq.next()
        } else {
            "alpha".to_string()
        }
    }

    pub fn add_version(&mut self, source: &str, nato: &str, file_count: usize, output_path: &str) {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let version_info = VersionInfo {
            timestamp,
            files: file_count,
            path: output_path.to_string(),
        };

        self.manifest
            .sources
            .entry(source.to_string())
            .or_default()
            .versions
            .insert(nato.to_string(), version_info);
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(&self.manifest)?;
        if let Some(parent) = self.manifest_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.manifest_path, content)?;
        Ok(())
    }
}

pub struct OutputInfo {
    pub filename: String,
    pub timestamp: String,
    pub source: String,
    pub nato: String,
    pub output_path: PathBuf,
}

impl OutputInfo {
    pub fn new(source: &str, nato: &str) -> Self {
        let timestamp = chrono::Utc::now();
        let timestamp_str = timestamp.format("%Y-%m-%d-%H%M%S").to_string();
        let filename = format!("{}-{}-{}.toml", source, timestamp_str, nato);
        Self {
            filename,
            timestamp: timestamp.to_rfc3339(),
            source: source.to_string(),
            nato: nato.to_string(),
            output_path: PathBuf::from("."),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nato_sequence_basic() {
        let mut seq = NatoSequence::new();
        assert_eq!(seq.next(), "alpha");
        assert_eq!(seq.next(), "bravo");
        assert_eq!(seq.next(), "charlie");
        assert_eq!(seq.next(), "delta");
    }

    #[test]
    fn test_nato_sequence_wrap() {
        let mut seq = NatoSequence::new();
        for _ in 0..24 {
            let _ = seq.next();
        }
        // After 24 calls, next() should wrap and return something ending with "-1"
        let next = seq.next();
        assert!(next.ends_with("-1"), "expected suffix '-1', got: {}", next);
    }

    #[test]
    fn test_nato_from_string() {
        assert_eq!(NatoSequence::from_string("alpha"), Some((0, 0)));
        assert_eq!(NatoSequence::from_string("bravo"), Some((1, 0)));
        assert_eq!(NatoSequence::from_string("charlie-2"), Some((2, 2)));
        assert_eq!(NatoSequence::from_string("invalid"), None);
    }

    #[test]
    fn test_output_info() {
        let info = OutputInfo::new("frontend", "alpha");
        assert!(info.filename.contains("frontend"));
        assert!(info.filename.contains("alpha"));
        assert!(info.filename.ends_with(".toml"));
    }
}
