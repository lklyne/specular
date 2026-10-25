//! A child process whose stdout and stderr arrive as lines, read off the
//! loop's thread, with its exit last.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use super::{AgentProcess, Output, Waker};

/// Starts `program` in `cwd` with `env` added to its environment, and
/// writes `input` to its stdin.
pub(super) fn spawn(
    program: &Path,
    args: &[String],
    input: String,
    cwd: &Path,
    env: &[(&str, &str)],
    waker: Option<Waker>,
) -> std::io::Result<Box<dyn AgentProcess>> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .envs(env.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let (tx, rx) = mpsc::channel();
    let mut threads = Vec::new();
    if let Some(mut stdin) = child.stdin.take() {
        // Off the loop's thread: a big image would block on a full pipe.
        threads.push(std::thread::spawn(move || {
            // A child that exited early closes the pipe; its exit says why.
            let _ = stdin.write_all(input.as_bytes());
        }));
    }
    if let Some(stdout) = child.stdout.take() {
        threads.push(reader(stdout, tx.clone(), waker.clone(), Output::Stdout));
    }
    if let Some(stderr) = child.stderr.take() {
        threads.push(reader(stderr, tx.clone(), waker.clone(), Output::Stderr));
    }
    let child = Arc::new(Mutex::new(child));
    let waiter = wait_for_exit(Arc::clone(&child), threads, tx, waker);
    Ok(Box::new(Process {
        child,
        output: rx,
        waiter: Some(waiter),
    }))
}

fn reader<R: std::io::Read + Send + 'static>(
    source: R,
    tx: Sender<Output>,
    waker: Option<Waker>,
    wrap: fn(String) -> Output,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        for line in BufReader::new(source).lines() {
            let Ok(line) = line else { break };
            if tx.send(wrap(line)).is_err() {
                break;
            }
            if let Some(wake) = &waker {
                wake();
            }
        }
    })
}

fn wait_for_exit(
    child: Arc<Mutex<Child>>,
    readers: Vec<JoinHandle<()>>,
    tx: Sender<Output>,
    waker: Option<Waker>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let status = loop {
            let polled = match child.lock() {
                Ok(mut child) => child.try_wait(),
                Err(_) => return,
            };
            match polled {
                Ok(Some(status)) => break Some(status),
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(_) => break None,
            }
        };
        // Every line is in the channel before the exit.
        for reader in readers {
            let _ = reader.join();
        }
        let exit = match status {
            Some(status) => Output::Exit {
                success: status.success(),
                detail: status.to_string(),
            },
            None => Output::Exit {
                success: false,
                detail: "the process could not be waited on".into(),
            },
        };
        let _ = tx.send(exit);
        if let Some(wake) = &waker {
            wake();
        }
    })
}

struct Process {
    child: Arc<Mutex<Child>>,
    output: Receiver<Output>,
    waiter: Option<JoinHandle<()>>,
}

impl AgentProcess for Process {
    fn poll(&mut self) -> Vec<Output> {
        self.output.try_iter().collect()
    }

    fn kill(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            // Already gone is the goal; `wait` reaps it either way.
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(waiter) = self.waiter.take() {
            // The pipes close with the child, so the readers and the waiter end.
            let _ = waiter.join();
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        self.kill();
    }
}
