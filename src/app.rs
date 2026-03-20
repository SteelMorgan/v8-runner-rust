use clap::Parser;
use tracing::{error, info};

use crate::cli::args::{Cli, Command};
use crate::cli::execute;
use crate::config::loader::load_config;
use crate::output::presenter::Presenter;

pub fn run() -> i32 {
    let cli = Cli::parse();

    let color_mode = if cli.no_color {
        crate::output::presenter::ColorMode::Disabled
    } else {
        crate::output::presenter::ColorMode::Enabled
    };
    let presenter = Presenter::new(cli.output.clone(), color_mode);

    let config = match load_config(cli.config.as_deref(), cli.workdir.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            presenter.print_error(&format!("{e}"));
            return crate::output::exit_codes::VALIDATION_ERROR;
        }
    };

    let level = cli.log_level.as_deref().unwrap_or("info");
    let action_log_path =
        match crate::support::logging::init_action_logging(level, &cli.output, &config.work_path) {
            Ok(path) => path,
            Err(e) => {
                presenter.print_error(&format!("{e}"));
                return crate::output::exit_codes::RUNTIME_ERROR;
            }
        };

    info!(
        command = command_name(&cli.command),
        output = cli.output.as_str(),
        work_path = %config.work_path.display(),
        "starting command"
    );
    if let Some(path) = &action_log_path {
        info!(path = %path.display(), "action log file enabled");
    }

    if cli.clean_before_execution {
        info!("cleaning platform logs directory before execution");
        match crate::support::temp::platform_logs_dir(&config.work_path)
            .and_then(|dir| crate::support::fs::clean_dir(&dir))
        {
            Ok(()) => info!("platform logs directory cleaned"),
            Err(e) => {
                presenter.print_error(&format!("failed to clean platform logs: {e}"));
                return crate::output::exit_codes::RUNTIME_ERROR;
            }
        }
    }

    let result = match &cli.command {
        Command::Build(_) | Command::Test(_) | Command::Dump(_) | Command::Syntax(_)
        | Command::Launch(_) => execute::execute_command(&config, &cli.command, &presenter),
    };

    match result {
        Ok(()) => {
            info!(
                command = command_name(&cli.command),
                "command finished successfully"
            );
            0
        }
        Err(e) => {
            error!("{e}");
            e.exit_code()
        }
    }
}

fn command_name(command: &Command) -> &'static str {
    execute::command_name(command).as_str()
}
