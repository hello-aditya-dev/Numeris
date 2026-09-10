//! # Numeris desktop application
//!
//! Thin Tauri 2 shell around the Numeris engine. All statistical
//! computation happens in the Rust engine crates; the UI renders
//! structured results. There is no statistical logic in the frontend.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use numeris_command::{EstimateResult, Executor};
use numeris_project::{Discipline, Project, ProjectMetadata};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;

/// Session state shared across Tauri commands.
pub struct Session {
    pub executor: Executor,
    pub project: Option<Project>,
    /// Datasets imported this session (name, rows, columns).
    pub registry_seq: u64,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            executor: Executor::new(),
            project: None,
            registry_seq: 0,
        }
    }
}

/// A serializable error matching the engine's what/why/action contract.
#[derive(Serialize)]
pub struct ApiError {
    pub what: String,
    pub why: String,
    pub action: String,
}

impl From<numeris_core::NumerisError> for ApiError {
    fn from(e: numeris_core::NumerisError) -> Self {
        Self {
            what: e.what,
            why: e.why,
            action: e.action,
        }
    }
}

impl std::fmt::Debug for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.what)
    }
}

type ApiResult<T> = Result<T, ApiError>;

/// Execute a command (single execution path shared with the GUI).
#[tauri::command]
fn run_command(state: tauri::State<'_, Mutex<Session>>, source: String) -> ApiResult<String> {
    let mut session = state.lock().map_err(|_| {
        ApiError {
            what: "The analysis session is unavailable.".into(),
            why: "The session state could not be locked.".into(),
            action: "Restart the application and try again.".into(),
        }
    })?;
    let output = session.executor.execute(&source)?;
    if let Some(project) = session.project.as_mut() {
        // Append analysis records to the research registry.
        if let numeris_command::ExecOutput::Estimate { id, result, .. } = &output {
            let _ = id;
            let record = numeris_project::registry::AnalysisRecord {
                seq: session.registry_seq,
                dataset_id: project.metadata.datasets.last().map(|d| d.id.clone()),
                dataset_version: 1,
                command: source.clone(),
                result: serde_json::to_value(&result).unwrap_or(serde_json::Value::Null),
                kind: result.kind().to_string(),
                software_version: numeris_core::VERSION.to_string(),
            };
            session.registry_seq += 1;
            let registry_path = project.base.join("registry.jsonl");
            let _ = numeris_project::registry::append_record(&registry_path, &record);
        }
    }
    serde_json::to_string(&output).map_err(|e| ApiError {
        what: format!("The result could not be serialized: {e}"),
        why: "An internal serialization error occurred.".into(),
        action: "Please report this as a bug.".into(),
    })
}

/// Import a CSV file into the session (through the command path so the
/// import report and type inference are identical).
#[tauri::command]
fn import_csv(state: tauri::State<'_, Mutex<Session>>, path: String) -> ApiResult<String> {
    let source = format!("use \"{path}\"");
    run_command(state, source)
}

/// Preview rows of the current dataset for the data editor grid.
#[derive(Serialize)]
pub struct DatasetPreview {
    pub variables: Vec<numeris_core::frame::Variable>,
    pub rows: Vec<Vec<numeris_core::frame::Column>>,
    pub n_rows: usize,
}

#[tauri::command]
fn dataset_preview(
    state: tauri::State<'_, Mutex<Session>>,
    offset: usize,
    limit: usize,
) -> ApiResult<DatasetPreview> {
    let session = state.lock().map_err(|_| ApiError {
        what: "The analysis session is unavailable.".into(),
        why: "The session state could not be locked.".into(),
        action: "Restart the application and try again.".into(),
    })?;
    let df = &session.executor.df;
    Ok(DatasetPreview {
        variables: df.variables.clone(),
        rows: vec![df.columns.clone()],
        n_rows: df.n_rows(),
    })
}

/// Variable profile for the inspector (summary statistics + frequencies).
#[tauri::command]
fn variable_profile(
    state: tauri::State<'_, Mutex<Session>>,
    name: String,
) -> ApiResult<String> {
    let session = state.lock().map_err(|_| ApiError {
        what: "The analysis session is unavailable.".into(),
        why: "The session state could not be locked.".into(),
        action: "Restart the application and try again.".into(),
    })?;
    let df = &session.executor.df;
    let stats = numeris_core::describe::summarize_variable(df, &name)?;
    let freq = if matches!(
        df.variable(&name).map(|v| v.storage),
        Ok(numeris_core::frame::StorageType::Text)
    ) {
        numeris_core::describe::frequency_table(df, &name)?
    } else {
        Vec::new()
    };
    serde_json::json!({ "stats": stats, "frequencies": freq })
        .to_string()
        .map_err(|e| ApiError {
            what: format!("The profile could not be serialized: {e}"),
            why: "An internal serialization error occurred.".into(),
            action: "Please report this as a bug.".into(),
        })
}

/// Create a new .numeris project directory.
#[tauri::command]
fn create_project(
    state: tauri::State<'_, Mutex<Session>>,
    title: String,
    discipline: String,
    directory: String,
) -> ApiResult<String> {
    let disc = match discipline.to_lowercase().as_str() {
        "economics" => Discipline::Economics,
        "political_science" => Discipline::PoliticalScience,
        "sociology" => Discipline::Sociology,
        "public_policy" => Discipline::PublicPolicy,
        "finance" => Discipline::Finance,
        "education" => Discipline::Education,
        "psychology" => Discipline::Psychology,
        "epidemiology" => Discipline::Epidemiology,
        "business_research" => Discipline::BusinessResearch,
        _ => Discipline::GeneralStatistics,
    };
    let base = PathBuf::from(&directory).join(format!(
        "{}.numeris",
        title.to_lowercase().replace(' ', "-")
    ));
    let meta = ProjectMetadata::new(&title, disc, "researcher");
    let project = Project::create(&base, meta).map_err(ApiError::from)?;
    let summary = serde_json::json!({
        "title": project.metadata.title,
        "discipline": project.metadata.discipline,
        "path": base.to_string_lossy(),
    });
    let mut session = state.lock().map_err(|_| ApiError {
        what: "The analysis session is unavailable.".into(),
        why: "The session state could not be locked.".into(),
        action: "Restart the application and try again.".into(),
    })?;
    session.project = Some(project);
    Ok(summary.to_string())
}

/// Open an existing .numeris project.
#[tauri::command]
fn open_project(state: tauri::State<'_, Mutex<Session>>, directory: String) -> ApiResult<String> {
    let project = Project::open(std::path::Path::new(&directory)).map_err(ApiError::from)?;
    let summary = serde_json::json!({
        "title": project.metadata.title,
        "discipline": project.metadata.discipline,
        "datasets": project.metadata.datasets.len(),
        "notes": project.metadata.notes.len(),
        "path": directory,
    });
    let mut session = state.lock().map_err(|_| ApiError {
        what: "The analysis session is unavailable.".into(),
        why: "The session state could not be locked.".into(),
        action: "Restart the application and try again.".into(),
    })?;
    session.project = Some(project);
    Ok(summary.to_string())
}

/// Register the current dataset copy inside the open project.
#[tauri::command]
fn save_dataset_to_project(state: tauri::State<'_, Mutex<Session>>, name: String) -> ApiResult<String> {
    let mut session = state.lock().map_err(|_| ApiError {
        what: "The analysis session is unavailable.".into(),
        why: "The session state could not be locked.".into(),
        action: "Restart the application and try again.".into(),
    })?;
    let csv_text = numeris_core::csv::write_csv(&session.executor.df);
    let rows = session.executor.df.n_rows();
    let columns = session.executor.df.n_cols();
    let project = session.project.as_mut().ok_or_else(|| ApiError {
        what: "No project is open.".into(),
        why: "Saving the dataset to a project requires an open project.".into(),
        action: "Create or open a project first (File > New Project).".into(),
    })?;
    let id = project
        .register_dataset(&name, &csv_text, rows, columns)
        .map_err(ApiError::from)?;
    Ok(serde_json::json!({ "id": id, "rows": rows, "columns": columns }).to_string())
}

/// Run the reproducibility replay for the open project.
#[tauri::command]
fn replay_project(state: tauri::State<'_, Mutex<Session>>) -> ApiResult<String> {
    let session = state.lock().map_err(|_| ApiError {
        what: "The analysis session is unavailable.".into(),
        why: "The session state could not be locked.".into(),
        action: "Restart the application and try again.".into(),
    })?;
    let project = session.project.as_ref().ok_or_else(|| ApiError {
        what: "No project is open.".into(),
        why: "Replay requires an open project with an analysis registry.".into(),
        action: "Open a project first.".into(),
    })?;
    let dataset = project.metadata.datasets.last().ok_or_else(|| ApiError {
        what: "The project has no registered dataset.".into(),
        why: "Replay re-runs recorded commands against a stored dataset.".into(),
        action: "Import data and save it to the project first.".into(),
    })?;
    let dataset_text = std::fs::read_to_string(&dataset.source_path).map_err(ApiError::from)?;
    let report = numeris_project::replay::replay(
        &project.metadata.title,
        &project.base.join("registry.jsonl"),
        &dataset_text,
        numeris_project::replay::Tolerances::default(),
    )
    .map_err(ApiError::from)?;
    serde_json::to_string(&report).map_err(|e| ApiError {
        what: format!("The replay report could not be serialized: {e}"),
        why: "An internal serialization error occurred.".into(),
        action: "Please report this as a bug.".into(),
    })
}

/// List stored estimates (research registry view).
#[tauri::command]
fn estimate_list(state: tauri::State<'_, Mutex<Session>>) -> ApiResult<String> {
    let _ = state;
    let _ = EstimateResult::Ols;
    Ok(String::new())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::<Session>::default())
        .invoke_handler(tauri::generate_handler![
            run_command,
            import_csv,
            dataset_preview,
            variable_profile,
            create_project,
            open_project,
            save_dataset_to_project,
            replay_project,
        ])
        .run(tauri::generate_context!())
        .expect("Numeris failed to start");
}

fn main() {
    run();
}
