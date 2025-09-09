mod app;
mod cli;
mod grid;
mod render;
mod superstate;
mod tile;
mod wave;

#[cfg(feature = "image-input")]
fn main() {
    use cli::Opt;
    use app::WfcApp;
    use structopt::StructOpt;
    use structopt_flags::LogLevel;
    use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
    use std::io;

    let opt: Opt = Opt::from_args();

    if let Some(shell) = opt.completions {
        Opt::clap().gen_completions_to(env!("CARGO_PKG_NAME"), shell, &mut io::stdout());
        return;
    }

    TermLogger::init(
        opt.verbose.get_level_filter(),
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    match opt.into_app_config() {
        Ok(config) => {
            let app = WfcApp::new(config);
            if let Err(e) = app.run() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    }
}