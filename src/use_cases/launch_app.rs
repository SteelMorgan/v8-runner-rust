use std::time::{Duration, Instant};

use crate::config::model::AppConfig;
use crate::domain::launch::{LaunchMode, LaunchResult};
use crate::platform::locator::UtilityType;
use crate::platform::process::ProcessRequest;
use crate::platform::utilities::PlatformUtilities;
use crate::support::error::AppError;
use crate::use_cases::context::ExecutionContext;
use crate::use_cases::request::LaunchRequest as LaunchArgs;
use crate::use_cases::result::{UseCaseFailure, UseCaseResult};
use tracing::info;

const LAUNCH_STARTUP_PROBE: Duration = Duration::from_millis(250);

pub fn execute(
    context: &ExecutionContext,
    config: &AppConfig,
    args: &LaunchArgs,
) -> UseCaseResult<LaunchResult> {
    info!(
        command = context.command().as_str(),
        transport = ?context.transport(),
        mode = args.mode.as_str(),
        "executing launch use case"
    );
    let started = Instant::now();
    let (mode, utility, command_mode) = match args.mode.as_str() {
        "designer" => (LaunchMode::Designer, UtilityType::V8, "DESIGNER"),
        "thin" => (LaunchMode::Thin, UtilityType::V8C, "ENTERPRISE"),
        "thick" => (LaunchMode::Thick, UtilityType::V8, "ENTERPRISE"),
        other => {
            return Err(UseCaseFailure::without_payload(
                AppError::Validation(format!("unsupported launch mode: {other}")),
                failure_result(other, started, None, None),
            ));
        }
    };

    let mut utilities = PlatformUtilities::from_config(config);
    let location = utilities
        .locate(utility)
        .map_err(|e| {
            UseCaseFailure::without_payload(
                AppError::Platform(e.to_string()),
                failure_result(args.mode.as_str(), started, None, None),
            )
        })?;

    let mut process_args = vec![command_mode.to_owned()];
    process_args.extend(config.v8_connection().args());

    let spawned = utilities
        .runner_for(utility)
        .spawn(&ProcessRequest {
            program: location.path.clone(),
            args: process_args,
            workdir: None,
            stdout_log_path: None,
            stderr_log_path: None,
            startup_probe: Some(LAUNCH_STARTUP_PROBE),
        })
        .map_err(|e| {
            UseCaseFailure::without_payload(
                AppError::Platform(e.to_string()),
                failure_result(args.mode.as_str(), started, Some(location.path.clone()), None),
            )
        })?;

    let result = LaunchResult {
        ok: true,
        mode,
        pid: Some(spawned.pid),
        binary: spawned.binary.clone(),
        message: Some(format!(
            "Launched {} via {} (pid {})",
            args.mode,
            spawned.binary.display(),
            spawned.pid
        )),
        duration_ms: started.elapsed().as_millis() as u64,
    };
    Ok(result)
}

fn failure_result(
    mode: &str,
    started: Instant,
    binary: Option<std::path::PathBuf>,
    message: Option<String>,
) -> LaunchResult {
    let mode = match mode {
        "thin" => LaunchMode::Thin,
        "thick" => LaunchMode::Thick,
        _ => LaunchMode::Designer,
    };
    LaunchResult {
        ok: false,
        mode,
        pid: None,
        binary: binary.unwrap_or_default(),
        message,
        duration_ms: started.elapsed().as_millis() as u64,
    }
}
