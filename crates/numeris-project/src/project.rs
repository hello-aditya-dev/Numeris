//! # Project format (`.numeris`)
//!
//! A Numeris project is an inspectable directory of open-standard files —
//! never an opaque single-file database:
//!
//! ```text
//! my-research.numeris/
//! ├── project.json        metadata, registries
//! ├── data/               imported datasets (CSV)
//! ├── scripts/            saved command scripts
//! ├── outputs/            exported tables and figures
//! ├── notes/              research notes
//! └── registry.jsonl      append-only analysis record
//! ```

use numeris_core::error::{NumerisError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Research disciplines supported by V1 (one engine, discipline-aware
/// terminology and templates).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Discipline {
    Economics,
    PoliticalScience,
    Sociology,
    PublicPolicy,
    Finance,
    Education,
    Psychology,
    Epidemiology,
    BusinessResearch,
    GeneralStatistics,
}

impl Discipline {
    pub fn label(&self) -> &'static str {
        match self {
            Discipline::Economics => "Economics",
            Discipline::PoliticalScience => "Political Science",
            Discipline::Sociology => "Sociology",
            Discipline::PublicPolicy => "Public Policy",
            Discipline::Finance => "Finance",
            Discipline::Education => "Education",
            Discipline::Psychology => "Psychology",
            Discipline::Epidemiology => "Epidemiology",
            Discipline::BusinessResearch => "Business Research",
            Discipline::GeneralStatistics => "General Statistics",
        }
    }
}

/// A registered dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRecord {
    pub id: String,
    pub name: String,
    pub source_path: String,
    pub rows: usize,
    pub columns: usize,
    pub imported_seq: u64,
}

/// project.json — the only metadata file required at the root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// UUID-shaped unique id.
    pub id: String,
    pub title: String,
    pub discipline: Discipline,
    pub creator: String,
    pub software_version: String,
    pub datasets: Vec<DatasetRecord>,
    pub scripts: Vec<String>,
    pub notes: Vec<String>,
}

impl ProjectMetadata {
    pub fn new(title: &str, discipline: Discipline, creator: &str) -> Self {
        Self {
            id: new_id(),
            title: title.to_string(),
            discipline,
            creator: creator.to_string(),
            software_version: numeris_core::VERSION.to_string(),
            datasets: Vec::new(),
            scripts: Vec::new(),
            notes: Vec::new(),
        }
    }
}

/// Simple deterministic-ish unique id (time + counter + process id).
fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let c = COUNTER.fetch_add(1, Ordering::SeqCst);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("p{now:x}{c:x}")
}

/// An open project: metadata + base directory.
#[derive(Debug)]
pub struct Project {
    pub base: PathBuf,
    pub metadata: ProjectMetadata,
}

impl Project {
    /// Create a new project directory structure.
    pub fn create(base: &Path, metadata: ProjectMetadata) -> Result<Project> {
        for dir in ["data", "scripts", "outputs", "notes"] {
            std::fs::create_dir_all(base.join(dir))?;
        }
        let project = Project {
            base: base.to_path_buf(),
            metadata,
        };
        project.save()?;
        Ok(project)
    }

    /// Open an existing project directory.
    pub fn open(base: &Path) -> Result<Project> {
        let meta_path = base.join("project.json");
        if !meta_path.exists() {
            return Err(NumerisError::io(
                format!("The folder '{}' does not contain a project.json file.", base.display()),
                "Numeris projects are directories containing project.json; this folder is not a Numeris project.",
                "Open a valid .numeris project directory, or create a new project first.",
            ));
        }
        let text = std::fs::read_to_string(&meta_path)?;
        let metadata: ProjectMetadata = serde_json::from_str(&text).map_err(|e| {
            NumerisError::io(
                format!("The project metadata could not be read: {e}"),
                "project.json exists but is not valid JSON for this version of Numeris.",
                "The project may have been created by a different version; check the file or restore a backup.",
            )
        })?;
        Ok(Project {
            base: base.to_path_buf(),
            metadata,
        })
    }

    /// Persist project.json.
    pub fn save(&self) -> Result<()> {
        let text = serde_json::to_string_pretty(&self.metadata)?;
        std::fs::write(self.base.join("project.json"), text)?;
        Ok(())
    }

    /// Save a dataset copy into data/ and register it.
    pub fn register_dataset(
        &mut self,
        name: &str,
        source_csv_text: &str,
        rows: usize,
        columns: usize,
    ) -> Result<String> {
        std::fs::create_dir_all(self.base.join("data"))?;
        let id = format!("ds{}", self.metadata.datasets.len() + 1);
        let path = self.base.join("data").join(format!("{id}.csv"));
        std::fs::write(&path, source_csv_text)?;
        self.metadata.datasets.push(DatasetRecord {
            id: id.clone(),
            name: name.to_string(),
            source_path: path.to_string_lossy().to_string(),
            rows,
            columns,
            imported_seq: self.metadata.datasets.len() as u64 + 1,
        });
        self.save()?;
        Ok(id)
    }

    /// Save a command script.
    pub fn save_script(&mut self, name: &str, source: &str) -> Result<()> {
        std::fs::create_dir_all(self.base.join("scripts"))?;
        let file = name.trim_end_matches(".nx").to_string() + ".nx";
        std::fs::write(self.base.join("scripts").join(&file), source)?;
        if !self.metadata.scripts.contains(&file) {
            self.metadata.scripts.push(file);
            self.save()?;
        }
        Ok(())
    }

    /// Load a script's source.
    pub fn load_script(&self, name: &str) -> Result<String> {
        let file = name.trim_end_matches(".nx").to_string() + ".nx";
        Ok(std::fs::read_to_string(
            self.base.join("scripts").join(file),
        )?)
    }

    /// Save a research note.
    pub fn add_note(&mut self, text: &str) -> Result<()> {
        std::fs::create_dir_all(self.base.join("notes"))?;
        let idx = self.metadata.notes.len() + 1;
        let file = format!("note{idx:03}.md");
        std::fs::write(self.base.join("notes").join(&file), text)?;
        self.metadata.notes.push(file);
        self.save()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("numeris-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        base
    }

    #[test]
    fn create_open_and_round_trip() {
        let base = temp_dir("project");
        let meta = ProjectMetadata::new("Wage Study", Discipline::Economics, "hello-aditya-dev");
        let mut p = Project::create(&base, meta).unwrap();
        p.register_dataset("wages", "a,b\n1,2\n", 1, 2).unwrap();
        p.save_script("baseline", "regress wage education\n")
            .unwrap();
        p.add_note("Baseline run complete.").unwrap();

        let opened = Project::open(&base).unwrap();
        assert_eq!(opened.metadata.title, "Wage Study");
        assert_eq!(opened.metadata.discipline, Discipline::Economics);
        assert_eq!(opened.metadata.datasets.len(), 1);
        assert_eq!(
            opened.load_script("baseline").unwrap(),
            "regress wage education\n"
        );
        assert_eq!(opened.metadata.notes.len(), 1);
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn open_rejects_non_project_folder() {
        let base = temp_dir("notproject");
        std::fs::create_dir_all(&base).unwrap();
        let err = Project::open(&base).unwrap_err();
        assert!(err.what.contains("project.json"));
        std::fs::remove_dir_all(&base).unwrap();
    }
}
