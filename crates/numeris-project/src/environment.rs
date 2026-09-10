//! # Environment manifest
//!
//! Records everything needed to interpret results reproducibly:
//! software version, platform, architecture, and engine settings.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentManifest {
    pub software_version: String,
    pub build_target: String,
    pub os_family: String,
    pub pointer_width: usize,
    pub engine: String,
    pub random_seed_used: Option<u64>,
}

impl EnvironmentManifest {
    /// Capture the current environment deterministically.
    pub fn capture(random_seed: Option<u64>) -> Self {
        Self {
            software_version: numeris_core::VERSION.to_string(),
            build_target: std::env::consts::ARCH.to_string(),
            os_family: std::env::consts::OS.to_string(),
            pointer_width: std::mem::size_of::<usize>() * 8,
            engine: "numeris-rust-engine".to_string(),
            random_seed_used: random_seed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_captures_and_serializes() {
        let m = EnvironmentManifest::capture(None);
        let text = serde_json::to_string(&m).unwrap();
        let back: EnvironmentManifest = serde_json::from_str(&text).unwrap();
        assert_eq!(back.software_version, m.software_version);
        assert_eq!(back.engine, "numeris-rust-engine");
    }
}
