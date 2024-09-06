use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use crossbeam_channel::Sender;
use log::{debug, trace};
use query::QueryState;
use watch_rs::models::watcher::Watcher;

pub mod query;
pub mod watcher;

pub static TICK_RATE: Duration = Duration::from_millis(15);

pub struct WatcherIterationOutput {
    iteration: usize,
    output: String,
}

pub enum WatcherOutputEvent {
    SetupResult(WatcherIterationOutput),
    IterationResult(WatcherIterationOutput),
    End,
}

pub fn run_watcher_thread(
    mut watcher: Watcher,
    query_state: QueryState,
    interval: Duration,
    watch_duration: Option<Duration>,
    watcher_event_sender: Sender<WatcherOutputEvent>,
    should_close_watcher: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        // If set, add the setup commands in the shell
        debug!("Executing setup commands : {}", query_state.setup_commands);
        let captured_stdout = watcher
            .exec_cmd_and_fetch_output(&query_state.setup_commands)
            .unwrap();
        watcher_event_sender
            .send(WatcherOutputEvent::SetupResult(WatcherIterationOutput {
                iteration: 0,
                output: captured_stdout,
            }))
            .unwrap();

        let watcher_start_checkpoint = Instant::now();
        let mut iteration = 0;

        // Execute the watcher command in the shell in a loop
        loop {
            iteration += 1;
            let captured_stdout = watcher
                .exec_cmd_and_fetch_output(&query_state.main_commands)
                .unwrap();

            trace!("STDIN  > {}", query_state.main_commands);
            trace!("STDOUT = {}", captured_stdout);

            watcher_event_sender
                .try_send(WatcherOutputEvent::IterationResult(
                    WatcherIterationOutput {
                        iteration,
                        output: captured_stdout,
                    },
                ))
                .unwrap();

            if should_close_watcher.load(Ordering::Acquire) {
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

        watcher_event_sender
            .try_send(WatcherOutputEvent::End)
            .unwrap();
        watcher.kill().unwrap();
    });
}
