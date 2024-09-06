
use once_cell::sync::Lazy;
use rand::{Rng, SeedableRng};
use std::io::Write;
use subprocess::{Popen, PopenConfig, Redirection};
use rexpect::reader::{NBReader, ReadUntil};
use rand::{prelude::StdRng, distributions::Alphanumeric};
use crate::utils::OpenResult;


static CMD_END_MARKER: Lazy<String> = Lazy::new(|| {
    let rng = StdRng::seed_from_u64(5);
    rng.sample_iter(Alphanumeric).map(|u| u as char).take(100).collect()
});


pub struct Watcher {
    shell: Popen,
    stdout_reader: NBReader,
}

impl Watcher {
    pub fn new(command_timeout: u64) -> OpenResult<Self> {
        let mut shell_envs = PopenConfig::current_env();
        shell_envs.push(("LC_ALL".into(), "C".into()));

        // Setup Bash Shell subprocess
        let mut shell = Popen::create(
            &["/bin/bash"],
            PopenConfig {
                stdout: Redirection::Pipe,
                stderr: Redirection::Merge,
                stdin: Redirection::Pipe,
                env: Some(shell_envs.clone()),
                detached: true,
                ..Default::default()
            },
        )?;
        let stdout_reader = NBReader::new(shell.stdout.take().unwrap(),Some(command_timeout));

        // Init and execute shell setup commands
        let mut watcher = Self { shell, stdout_reader };
        watcher.exec_cmd_and_fetch_output("
            shopt -s expand_aliases;
            source ~/.bashrc;
        ")?;

        Ok(watcher)
    }

    pub fn exec_cmd_and_fetch_output(&mut self, command: &str) -> OpenResult<String> {
        let stdin = self.shell.stdin.as_mut().unwrap();

        writeln!(stdin, "{}", command)?;
        writeln!(stdin, "printf '{}'", CMD_END_MARKER.clone())?;

        let (captured_stdout, _) = self.stdout_reader
            .read_until(&ReadUntil::String(CMD_END_MARKER.clone()))?;
        Ok(captured_stdout)
    }

    pub fn kill(&mut self) -> OpenResult {
        Ok(self.shell.kill()?)
    }
}