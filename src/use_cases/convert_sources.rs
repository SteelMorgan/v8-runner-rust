use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;

use crate::config::model::AppConfig;
use crate::domain::convert::{ConvertDirection, ConvertResult};
use crate::platform::edt::EdtDsl;
use crate::platform::edt_session::{EdtSessionHostOptions, EdtSessionManager};
use crate::platform::locator::UtilityType;
use crate::platform::result::PlatformCommandResult;
use crate::platform::utilities::PlatformUtilities;
use crate::support::error::AppError;
use crate::support::fs::{
    ensure_dir, remove_path_if_exists, replace_dir_atomically, write_temp_dir_metadata, TempDirKind,
};
use crate::support::path::{nearest_existing_canonical_path, stable_path_identity};
use crate::use_cases::context::{CommandName, ExecutionContext, InterruptionSafetyClass};
use crate::use_cases::request::{ConvertDirectionRequest, ConvertRequest};
use crate::use_cases::result::{UseCaseFailure, UseCaseResult};

const CONVERT_BACKUP_PREFIX: &str = ".convert-backup";

type ConvertExecutionFailure = UseCaseFailure<ConvertResult>;

struct ResolvedConvertRequest {
    direction: ConvertDirection,
    source_path: PathBuf,
    target_path: PathBuf,
    workspace_path: PathBuf,
    target_identity: String,
    version: Option<String>,
    base_project_name: Option<String>,
    build: bool,
}

pub fn execute(
    context: &ExecutionContext,
    config: &AppConfig,
    request: &ConvertRequest,
) -> UseCaseResult<ConvertResult> {
    run_convert_with_context(context, config, request)
}

pub fn preflight_validate(config: &AppConfig, request: &ConvertRequest) -> Result<(), AppError> {
    let direction = map_direction(request.direction);
    resolve_request(config, request, direction).map(|_| ())
}

fn run_convert_with_context(
    context: &ExecutionContext,
    config: &AppConfig,
    request: &ConvertRequest,
) -> UseCaseResult<ConvertResult> {
    let started = Instant::now();
    let direction = map_direction(request.direction);
    let workspace_path = convert_workspace_path(config);

    if let Some(interruption) = context.interruption() {
        let error = AppError::Runtime(interruption.message(context.command()).to_owned());
        let message = error.to_string();
        return Err(ConvertExecutionFailure::with_payload(
            error,
            empty_result(direction, started, None, None, workspace_path, Some(message)),
        ));
    }

    let resolved = match resolve_request(config, request, direction) {
        Ok(resolved) => resolved,
        Err(error) => {
            let message = error.to_string();
            return Err(ConvertExecutionFailure::with_payload(
                error,
                empty_result(
                    direction,
                    started,
                    Some(PathBuf::from(request.source_path.trim())),
                    Some(PathBuf::from(request.target_path.trim())),
                    workspace_path,
                    Some(message),
                ),
            ));
        }
    };

    let target_parent = resolved.target_path.parent().ok_or_else(|| {
        ConvertExecutionFailure::with_payload(
            AppError::Validation(format!(
                "convert target path has no parent: {}",
                resolved.target_path.display()
            )),
            empty_result(
                resolved.direction,
                started,
                Some(resolved.source_path.clone()),
                Some(resolved.target_path.clone()),
                resolved.workspace_path.clone(),
                Some(format!(
                    "convert target path has no parent: {}",
                    resolved.target_path.display()
                )),
            ),
        )
    })?;
    ensure_dir(target_parent).map_err(|error| {
        ConvertExecutionFailure::with_payload(
            AppError::Runtime(format!(
                "failed to create convert target parent '{}': {error}",
                target_parent.display()
            )),
            empty_result(
                resolved.direction,
                started,
                Some(resolved.source_path.clone()),
                Some(resolved.target_path.clone()),
                resolved.workspace_path.clone(),
                Some(format!(
                    "failed to create convert target parent '{}': {error}",
                    target_parent.display()
                )),
            ),
        )
    })?;

    let mut utilities = PlatformUtilities::from_config(config);
    let location = match utilities.locate(UtilityType::EdtCli) {
        Ok(location) => location,
        Err(error) => {
            let app_error = AppError::Platform(error.to_string());
            let message = app_error.to_string();
            return Err(ConvertExecutionFailure::with_payload(
                app_error,
                empty_result(
                    resolved.direction,
                    started,
                    Some(resolved.source_path.clone()),
                    Some(resolved.target_path.clone()),
                    resolved.workspace_path.clone(),
                    Some(message),
                ),
            ));
        }
    };

    let run_id = make_run_id();
    let staging_dir = target_parent.join(format!(".convert-stage-{run_id}"));
    ensure_dir(&staging_dir).map_err(|error| {
        ConvertExecutionFailure::with_payload(
            AppError::Runtime(format!(
                "failed to create convert staging directory '{}': {error}",
                staging_dir.display()
            )),
            empty_result(
                resolved.direction,
                started,
                Some(resolved.source_path.clone()),
                Some(resolved.target_path.clone()),
                resolved.workspace_path.clone(),
                Some(format!(
                    "failed to create convert staging directory '{}': {error}",
                    staging_dir.display()
                )),
            ),
        )
    })?;
    write_temp_dir_metadata(
        &staging_dir,
        TempDirKind::Stage,
        &run_id,
        &resolved.target_path,
        &resolved.target_identity,
    )
    .map_err(|error| {
        let _ = remove_path_if_exists(&staging_dir);
        ConvertExecutionFailure::with_payload(
            AppError::Runtime(format!(
                "failed to write convert staging metadata '{}': {error}",
                staging_dir.display()
            )),
            empty_result(
                resolved.direction,
                started,
                Some(resolved.source_path.clone()),
                Some(resolved.target_path.clone()),
                resolved.workspace_path.clone(),
                Some(format!(
                    "failed to write convert staging metadata '{}': {error}",
                    staging_dir.display()
                )),
            ),
        )
    })?;

    let policy = context.process_policy(InterruptionSafetyClass::GracefulThenKill, None);
    let platform_result = if config.tools.edt_cli.interactive_mode {
        let manager =
            EdtSessionManager::for_config(config, EdtSessionHostOptions::for_cli_command(config))
                .map_err(|error| {
                    let _ = remove_path_if_exists(&staging_dir);
                    ConvertExecutionFailure::with_payload(
                        AppError::Platform(error.to_string()),
                        empty_result(
                            resolved.direction,
                            started,
                            Some(resolved.source_path.clone()),
                            Some(resolved.target_path.clone()),
                            resolved.workspace_path.clone(),
                            Some(error.to_string()),
                        ),
                    )
                })?;
        let dsl = EdtDsl::new_shared_session(
            location.path.clone(),
            resolved.workspace_path.clone(),
            Arc::new(manager),
            Duration::from_millis(config.tools.edt_cli.startup_timeout_ms),
            Duration::from_millis(config.tools.edt_cli.command_timeout_ms),
        )
        .map_err(|error| {
            let _ = remove_path_if_exists(&staging_dir);
            ConvertExecutionFailure::with_payload(
                AppError::Platform(error.to_string()),
                empty_result(
                    resolved.direction,
                    started,
                    Some(resolved.source_path.clone()),
                    Some(resolved.target_path.clone()),
                    resolved.workspace_path.clone(),
                    Some(error.to_string()),
                ),
            )
        })?
        .with_timeout(context.edt_timeout())
        .with_execution_policy(policy);
        run_platform_conversion(&dsl, &resolved, &staging_dir).map_err(|error| {
            let _ = remove_path_if_exists(&staging_dir);
            let message = error.to_string();
            ConvertExecutionFailure::with_payload(
                error,
                empty_result(
                    resolved.direction,
                    started,
                    Some(resolved.source_path.clone()),
                    Some(resolved.target_path.clone()),
                    resolved.workspace_path.clone(),
                    Some(message),
                ),
            )
        })?
    } else {
        let dsl = EdtDsl::new(
            location.path.clone(),
            resolved.workspace_path.clone(),
            utilities.runner_for(UtilityType::EdtCli),
        )
        .with_timeout(context.edt_timeout())
        .with_execution_policy(policy);
        run_platform_conversion(&dsl, &resolved, &staging_dir).map_err(|error| {
            let _ = remove_path_if_exists(&staging_dir);
            let message = error.to_string();
            ConvertExecutionFailure::with_payload(
                error,
                empty_result(
                    resolved.direction,
                    started,
                    Some(resolved.source_path.clone()),
                    Some(resolved.target_path.clone()),
                    resolved.workspace_path.clone(),
                    Some(message),
                ),
            )
        })?
    };

    ensure_platform_success(&resolved, &platform_result).map_err(|error| {
        let _ = remove_path_if_exists(&staging_dir);
        let message = error.to_string();
        ConvertExecutionFailure::with_payload(
            error,
            empty_result(
                resolved.direction,
                started,
                Some(resolved.source_path.clone()),
                Some(resolved.target_path.clone()),
                resolved.workspace_path.clone(),
                Some(message),
            ),
        )
    })?;

    if let Some(interruption) = context.interruption() {
        let _ = remove_path_if_exists(&staging_dir);
        let error = AppError::Runtime(format!(
            "{} for command '{}' before entering convert publication safe point",
            interruption.message(context.command()),
            CommandName::Convert.as_str()
        ));
        let message = error.to_string();
        return Err(ConvertExecutionFailure::with_payload(
            error,
            empty_result(
                resolved.direction,
                started,
                Some(resolved.source_path.clone()),
                Some(resolved.target_path.clone()),
                resolved.workspace_path.clone(),
                Some(message),
            ),
        ));
    }

    let publish_phase = context
        .run_no_process_critical_phase(|| {
            replace_dir_atomically(
                &staging_dir,
                &resolved.target_path,
                &run_id,
                &resolved.target_identity,
                CONVERT_BACKUP_PREFIX,
            )
            .map_err(|error| AppError::Runtime(format!("failed to publish converted target: {error}")))
        })
        .map_err(|error| {
            let message = error.to_string();
            ConvertExecutionFailure::with_payload(
                error,
                empty_result(
                    resolved.direction,
                    started,
                    Some(resolved.source_path.clone()),
                    Some(resolved.target_path.clone()),
                    resolved.workspace_path.clone(),
                    Some(message),
                ),
            )
        })?;

    Ok(ConvertResult {
        ok: true,
        direction: resolved.direction,
        source_path: resolved.source_path,
        target_path: resolved.target_path,
        workspace_path: resolved.workspace_path,
        duration_ms: started.elapsed().as_millis() as u64,
        message: merge_optional_messages(
            publish_phase.value.cleanup_warning,
            deferred_interruption_warning(publish_phase.deferred_interruption),
        ),
    })
}

fn resolve_request(
    config: &AppConfig,
    request: &ConvertRequest,
    direction: ConvertDirection,
) -> Result<ResolvedConvertRequest, AppError> {
    let source_path = normalize_input_path(&request.source_path, "source")?;
    let target_path = normalize_input_path(&request.target_path, "target")?;

    validate_distinct_paths(&source_path, &target_path)?;
    validate_directory_source(&source_path, "source")?;
    validate_target_path(&target_path)?;

    match direction {
        ConvertDirection::EdtToDesigner => {
            validate_required_marker(&source_path, ".project", "EDT source")?;
        }
        ConvertDirection::DesignerToEdt => {
            validate_designer_source(&source_path)?;
        }
    }

    let version = normalize_optional_value(request.version.as_deref(), "version")?;
    let base_project_name =
        normalize_optional_value(request.base_project_name.as_deref(), "base project name")?;
    let target_identity = stable_path_identity(
        &nearest_existing_canonical_path(&target_path).map_err(|error| {
            AppError::Runtime(format!(
                "failed to canonicalize convert target '{}': {error}",
                target_path.display()
            ))
        })?,
    );

    Ok(ResolvedConvertRequest {
        direction,
        source_path,
        target_path,
        workspace_path: convert_workspace_path(config),
        target_identity,
        version,
        base_project_name,
        build: request.build,
    })
}

fn normalize_input_path(value: &str, label: &str) -> Result<PathBuf, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(format!(
            "convert {label} path must not be blank"
        )));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(AppError::Validation(format!(
            "convert {label} path must not contain control characters"
        )));
    }
    Ok(PathBuf::from(trimmed))
}

fn normalize_optional_value(value: Option<&str>, label: &str) -> Result<Option<String>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(format!(
            "convert {label} must not be blank"
        )));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(AppError::Validation(format!(
            "convert {label} must not contain control characters"
        )));
    }
    Ok(Some(trimmed.to_owned()))
}

fn validate_distinct_paths(source_path: &Path, target_path: &Path) -> Result<(), AppError> {
    let source = nearest_existing_canonical_path(source_path).map_err(|error| {
        AppError::Runtime(format!(
            "failed to canonicalize convert source '{}': {error}",
            source_path.display()
        ))
    })?;
    let target = nearest_existing_canonical_path(target_path).map_err(|error| {
        AppError::Runtime(format!(
            "failed to canonicalize convert target '{}': {error}",
            target_path.display()
        ))
    })?;
    if source == target {
        return Err(AppError::Validation(
            "convert source and target paths must be different".to_owned(),
        ));
    }
    Ok(())
}

fn validate_directory_source(path: &Path, label: &str) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::Validation(format!(
            "convert {label} path does not exist: {}",
            path.display()
        )));
    }
    if !path.is_dir() {
        return Err(AppError::Validation(format!(
            "convert {label} path is not a directory: {}",
            path.display()
        )));
    }
    Ok(())
}

fn validate_target_path(path: &Path) -> Result<(), AppError> {
    if path.exists() && !path.is_dir() {
        return Err(AppError::Validation(format!(
            "convert target path is not a directory: {}",
            path.display()
        )));
    }
    Ok(())
}

fn validate_required_marker(path: &Path, marker: &str, label: &str) -> Result<(), AppError> {
    let marker_path = path.join(marker);
    if !marker_path.exists() {
        return Err(AppError::Validation(format!(
            "{label} path must contain '{}': {}",
            marker,
            path.display()
        )));
    }
    Ok(())
}

fn validate_designer_source(path: &Path) -> Result<(), AppError> {
    if path.join("Configuration.xml").exists() {
        return Ok(());
    }

    let has_top_level_xml = std::fs::read_dir(path)
        .map_err(|error| {
            AppError::Runtime(format!(
                "failed to inspect Designer source '{}': {error}",
                path.display()
            ))
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .any(|entry_path| {
            entry_path.is_file()
                && entry_path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case("xml"))
        });

    if has_top_level_xml {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "Designer source path must contain 'Configuration.xml' or a top-level XML descriptor: {}",
            path.display()
        )))
    }
}

fn run_platform_conversion(
    dsl: &EdtDsl<'_>,
    resolved: &ResolvedConvertRequest,
    staging_dir: &Path,
) -> Result<PlatformCommandResult, AppError> {
    match resolved.direction {
        ConvertDirection::EdtToDesigner => dsl
            .export_project_path(&resolved.source_path, staging_dir)
            .map_err(|error| AppError::Platform(error.to_string())),
        ConvertDirection::DesignerToEdt => dsl
            .import_configuration_files(
                staging_dir,
                &resolved.source_path,
                resolved.version.as_deref(),
                resolved.base_project_name.as_deref(),
                resolved.build,
            )
            .map_err(|error| AppError::Platform(error.to_string())),
    }
}

fn ensure_platform_success(
    resolved: &ResolvedConvertRequest,
    result: &PlatformCommandResult,
) -> Result<(), AppError> {
    if result.process.exit_code == 0 {
        return Ok(());
    }

    let direction = match resolved.direction {
        ConvertDirection::EdtToDesigner => "edt-to-designer",
        ConvertDirection::DesignerToEdt => "designer-to-edt",
    };
    let mut details = vec![format!(
        "convert {direction} failed with exit code {}",
        result.process.exit_code
    )];
    if !result.process.stdout.trim().is_empty() {
        details.push(format!("stdout: {}", result.process.stdout.trim()));
    }
    if !result.process.stderr.trim().is_empty() {
        details.push(format!("stderr: {}", result.process.stderr.trim()));
    }
    if let Some(log) = result
        .platform_log
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        details.push(format!("platform log: {}", log.trim()));
    }
    if let Some(path) = result.platform_log_path.as_ref() {
        details.push(format!("platform log path: {}", path.display()));
    }

    Err(AppError::Platform(details.join("; ")))
}

fn empty_result(
    direction: ConvertDirection,
    started: Instant,
    source_path: Option<PathBuf>,
    target_path: Option<PathBuf>,
    workspace_path: PathBuf,
    message: Option<String>,
) -> ConvertResult {
    ConvertResult {
        ok: false,
        direction,
        source_path: source_path.unwrap_or_default(),
        target_path: target_path.unwrap_or_default(),
        workspace_path,
        duration_ms: started.elapsed().as_millis() as u64,
        message,
    }
}

fn convert_workspace_path(config: &AppConfig) -> PathBuf {
    config.work_path.join("convert").join("edt-workspace")
}

fn map_direction(direction: ConvertDirectionRequest) -> ConvertDirection {
    match direction {
        ConvertDirectionRequest::EdtToDesigner => ConvertDirection::EdtToDesigner,
        ConvertDirectionRequest::DesignerToEdt => ConvertDirection::DesignerToEdt,
    }
}

fn deferred_interruption_warning(
    interruption: Option<crate::use_cases::context::ExecutionInterruption>,
) -> Option<String> {
    interruption.map(|interruption| {
        let reason = match interruption {
            crate::use_cases::context::ExecutionInterruption::Cancelled => "cancellation request",
            crate::use_cases::context::ExecutionInterruption::TimedOut => "timeout",
        };
        format!(
            "convert publication completed successfully after {reason} during critical phase; unsafe interruption was not performed"
        )
    })
}

fn merge_optional_messages(
    first: Option<String>,
    second: Option<String>,
) -> Option<String> {
    match (first, second) {
        (Some(first), Some(second)) => Some(format!("{first}; {second}")),
        (Some(message), None) | (None, Some(message)) => Some(message),
        (None, None) => None,
    }
}

fn make_run_id() -> String {
    let timestamp = Utc::now().timestamp_nanos_opt().unwrap_or_default();
    format!("{}-{timestamp:x}", std::process::id())
}
