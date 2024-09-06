mod envs;
mod tui;

use clap::Parser;
use crossbeam_channel::unbounded;
use envs::WATCHER_LOGS_DIR;
use log::{trace, LevelFilter};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Root};
use log4rs::Config;
use ratatui::DefaultTerminal;
use std::time::Duration;
use std::{
    io::Read,
    sync::{atomic::AtomicBool, Arc},
};
use tui::query::QueryTui;
use watch_rs::{models::watcher::Watcher, utils::OpenResult};

const DEFAULT_COMMAND_TIMEOUT: u64 = 30 * 1000;

/// Short help message
#[derive(Parser)]
#[command(version, about, long_about=None)]
struct Args {
    /// Individual command run timeout.
    /// Unit in seconds.
    #[arg(short = 't', long)]
    timeout: Option<u64>,

    /// Call interval between two command invocations.
    /// Defaults to 1 second. Unit in seconds.
    #[arg(short = 'n', long, default_value_t = 1.0)]
    interval: f64,

    /// Main command to execute and watch on.
    /// Optional to pass as a command argument, as we would query user for command(s) if not provided.
    #[arg(short = 'c', long)]
    command: Option<String>,

    /// Total duration for the watcher process.
    /// If a provided duration is smaller than interval (+ timeout), then we would exit after the first run.
    /// Defaults to None for infinite runs. Unit in seconds.
    #[arg(short = 'w', long)]
    watch_duration: Option<u64>,

    /// Flag to specify the presence of setup commands.
    /// We can query user for the setup commands if there are setup commands.
    #[arg(short = 's', long, default_value_t = false)]
    has_setup: bool,
}

fn init() -> OpenResult<()> {
    let stdout = FileAppender::builder().build(WATCHER_LOGS_DIR.path().join("watcher.log"))?;

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(LevelFilter::Trace))?;

    let _handle = log4rs::init_config(config)?;

    Ok(())
}

fn query_and_fetch_file_input(file_title: &str) -> OpenResult<String> {
    let filepath = WATCHER_LOGS_DIR.path().join(file_title);
    trace!("Created a temporary file : {filepath:?}");

    std::process::Command::new("vim")
        .args([&filepath])
        .spawn()?
        .wait()?;

    let mut cmds = String::new();
    std::fs::File::open(&filepath)?.read_to_string(&mut cmds)?;
    Ok(cmds)
}

fn run_app_in_terminal_instance(
    app: impl FnOnce(DefaultTerminal) -> OpenResult<()>,
) -> OpenResult<()> {
    // Setup terminal for TUI start
    let terminal = ratatui::init();

    let result = app(terminal);

    // Restore terminal after finish
    ratatui::restore();

    result
}

pub fn run_tui_app() -> OpenResult<()> {
    let args = Args::parse();

    // // Fetch and initialize the setup commands if Watcher `has_setup`
    // let mut optional_setup_cmds: Option<String> = None;
    // if args.has_setup {
    //     let setup_cmds = query_and_fetch_file_input("setup_commands.bash")?;
    //     optional_setup_cmds = Some(setup_cmds);
    // }

    // Fetch or query the Watcher `command`
    let command: String = args
        .command
        .unwrap_or_else(|| query_and_fetch_file_input("run_commands.bash").unwrap());

    let command_timeout = args.timeout.map_or(DEFAULT_COMMAND_TIMEOUT, |t| t * 1000);

    let interval = Duration::from_millis((args.interval * 1000.0).floor() as u64);
    let watch_duration = args.watch_duration.map(|d| Duration::from_millis(d * 1000));

    let watcher = Watcher::new(command_timeout)?;

    run_app_in_terminal_instance(move |mut terminal| {
        if let Some(query_state) = QueryTui::new(Some(command.clone())).run_app(&mut terminal)? {
            let (event_sender, event_receiver) = unbounded();
            let should_close_watcher = Arc::new(AtomicBool::new(false));

            // Create and start the watcher thread, with the event sender channel
            tui::run_watcher_thread(
                watcher,
                query_state,
                interval,
                watch_duration,
                event_sender,
                Arc::clone(&should_close_watcher),
            );

            // Create the TUI app and run it, with the event receiver channel
            let mut watcher_tui =
                tui::watcher::WatcherTui::new(event_receiver, Arc::clone(&should_close_watcher));
            return watcher_tui.run_app(&mut terminal);
        }
        Ok(())
    })
}

fn main() -> OpenResult<()> {
    init()?;

    run_tui_app()
}

// for command in ["ls", "cd target", "export X=yes", "cd -", "tree -L 2", "echo $X"] {
//     writeln!(stdin, "{}", command)?;
//     writeln!(stdin, "printf '{}'", CMD_END_MARKER.get().unwrap())?;

//     let (captured_stdout, _) = stdout_reader.read_until(
//         &ReadUntil::String(CMD_END_MARKER.get().unwrap().clone())
//     )?;

//     trace!("> {command}");
//     trace!("{}", captured_stdout);
// }
