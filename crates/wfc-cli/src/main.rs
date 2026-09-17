use clap::{CommandFactory, Parser};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::io;
use wfc_cli::app::WfcApp;
use wfc_cli::cli::Opt;

fn main() {
    let opt = Opt::parse();

    if let Some(shell) = opt.completions {
        let mut command = Opt::command();
        let name = command.get_name().to_string();

        clap_complete::generate(shell, &mut command, name, &mut io::stdout());
        return;
    }

    TermLogger::init(
        opt.verbose.log_level_filter(),
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    if let Err(error) = opt
        .into_app_config()
        .and_then(|config| WfcApp::new(config).run())
    {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
