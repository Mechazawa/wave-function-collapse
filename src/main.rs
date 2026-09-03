mod app;
mod cli;
mod render;

fn main() {
    use app::WfcApp;
    use clap::{CommandFactory, Parser};
    use cli::Opt;
    use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
    use std::io;

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

    let result = opt
        .into_app_config()
        .map_err(|error| error.into())
        .and_then(|config| WfcApp::new(config).run());

    if let Err(error) = result {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
