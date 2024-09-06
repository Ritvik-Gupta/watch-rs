use std::{
    time::{Duration},
};
use clap::Parser;
use watch_rs::{utils::OpenResult, models::watcher::Watcher};
use std::{
    io::Read, thread, time::{Instant}
};
use signal_hook::{consts::SIGINT, iterator::Signals};
use log::{debug, trace, LevelFilter};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Root};
use log4rs::Config;
use crossbeam_channel::{bounded, Receiver};


const DEFAULT_COMMAND_TIMEOUT: u64 = 30 * 1000;


/// Short help message
#[derive(Parser)]
#[command(version, about, long_about=None)]
struct Args {
    /// Individual command run timeout.
    /// Unit in seconds.
    #[arg(short='t', long)]
    timeout: Option<u64>,

    /// Call interval between two command invocations
    /// Defaults to 1 second. Unit in seconds.
    #[arg(short='n', long, default_value_t=1.0)]
    interval: f64,

    /// Main command to execute and watch on.
    /// Optional to pass as a command argument, as we would query user for command(s) if not provided.
    #[arg(short='c', long)]
    command: Option<String>,

    /// Total duration for the watcher process.
    /// If a provided duration is smaller than interval (+ timeout), then we would exit after the first run.
    /// Defaults to None for infinite runs. Unit in seconds.
    #[arg(short='w', long)]
    watch_duration: Option<u64>,

    /// Flag to specify the presence of setup commands.
    /// We can query user for the setup commands if there are setup commands.
    #[arg(short='s', long)]
    has_setup: bool
}

fn init() -> OpenResult<()> {
    let stdout = FileAppender::builder().build("logs/watcher.log")?;
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(LevelFilter::Trace))?;

    let _handle = log4rs::init_config(config)?;

    Ok(())
}

fn query_and_fetch_file_input() -> OpenResult<String> {
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_filepath = temp_file.into_temp_path();
    trace!("Created temporary file : {temp_filepath:?}");

    std::process::Command::new("vim")
        .args([&temp_filepath])
        .spawn()?
        .wait()?;

    let mut cmds = String::new();
    std::fs::File::open(&temp_filepath)?.read_to_string(&mut cmds)?;
    Ok(cmds)
}

fn setup_interrupt_signal_handler() -> OpenResult<Receiver<()>> {
    let (sender, receiver) = bounded(10);

    // Use global exit-signals to exit out of Watcher on termination.
    let mut signals = Signals::new([SIGINT])?;
    thread::spawn(move || {
        debug!("Registered a global signal handler for watcher process.");

        for sig in signals.forever() {
            debug!("Received signal {sig}, sending teminate event to watcher.");
            let _ = sender.send_timeout((), Duration::from_millis(500));
        }
    });

    Ok(receiver)
}

fn main() -> OpenResult<()> {
    init()?;

    let args = Args::parse();

    // Setup the signal handler thread and fetch the signal channel
    let interrupt_event_receiver = setup_interrupt_signal_handler()?;

    // Fetch and initialize the setup commands if Watcher `has_setup`
    let mut optional_setup_cmds: Option<String> = None;
    if args.has_setup {
        let setup_cmds = query_and_fetch_file_input()?;
        optional_setup_cmds = Some(setup_cmds);
    }

    // Fetch or query the Watcher `command`
    let command: String = args.command
        .unwrap_or_else(|| { query_and_fetch_file_input().unwrap() });

    let command_timeout = args.timeout
        .map_or(DEFAULT_COMMAND_TIMEOUT, |t| t * 1000);

    let interval = Duration::from_millis((args.interval * 1000.0).floor() as u64);
    let watch_duration = args.watch_duration.map(|d| Duration::from_millis(d * 1000));

    let mut watcher = Watcher::new(command_timeout)?;

    // If set, add the setup commands in the shell
    if let Some(setup_cmds) = optional_setup_cmds {
        debug!("Executing setup commands : {setup_cmds}");
        let _setup_captured_stdout = watcher.exec_cmd_and_fetch_output(&setup_cmds)?;
    }

    let watcher_start_checkpoint = Instant::now();

    // Execute the watcher command in the shell in a loop
    loop {
        let captured_stdout = watcher.exec_cmd_and_fetch_output(&command)?;

        trace!("STDIN  > {}", command);
        trace!("STDOUT = {}", captured_stdout);

        // Break if an interrupt signal was received
        if interrupt_event_receiver.try_recv().is_ok() {
            debug!("Received interrupt event, teminating the watcher.");
            break;
        }

        // Break if a we have exceeded a 'watch duration' specified
        if let Some(duration) = &watch_duration {
            if duration < &watcher_start_checkpoint.elapsed() {
                break;
            }
        }
        thread::sleep(interval);
    }

    watcher.kill()?;

    Ok(())
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