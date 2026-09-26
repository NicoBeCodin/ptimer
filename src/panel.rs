use std::io::Write;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{Duration, Instant};

use crate::{Config, Phase, State, human_mmss};

pub struct PanelIndicator {
    child: Child,
    stdin: ChildStdin,
    previous_label: String,
}

impl PanelIndicator {
    fn start() -> Option<Self> {
        if !cfg!(target_os = "linux") {
            return None;
        }
        let mut child = Command::new("/usr/bin/python3")
            .args(["-u", "-c", include_str!("panel_status.py")])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let stdin = child.stdin.take()?;
        Some(Self {
            child,
            stdin,
            previous_label: String::new(),
        })
    }

    fn update(&mut self, label: &str) -> bool {
        if self.child.try_wait().ok().flatten().is_some() {
            return false;
        }
        if self.previous_label == label {
            return true;
        }
        if writeln!(self.stdin, "{label}").is_err() || self.stdin.flush().is_err() {
            return false;
        }
        self.previous_label = label.to_owned();
        true
    }
}

#[derive(Default)]
pub struct PanelStatus {
    indicator: Option<PanelIndicator>,
    retry_at: Option<Instant>,
}

impl Drop for PanelIndicator {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn panel_label(state: &State) -> String {
    match state.phase {
        Phase::Idle => "READY --:--".to_string(),
        phase => {
            let status = match phase {
                Phase::Work => "WORK",
                Phase::Short => "BREAK",
                Phase::Long => "LONG BREAK",
                Phase::Idle => unreachable!(),
            };
            if state.paused {
                format!("{} PAUSED {}", status, human_mmss(state.remaining))
            } else {
                format!("{} {}", status, human_mmss(state.remaining))
            }
        }
    }
}

impl PanelStatus {
    pub fn sync(&mut self, state: &State, cfg: &Config) {
        if !cfg.panel_status {
            self.indicator = None;
            self.retry_at = None;
            return;
        }
        if self.indicator.is_none() {
            if self
                .retry_at
                .is_some_and(|retry_at| Instant::now() < retry_at)
            {
                return;
            }
            self.indicator = PanelIndicator::start();
            self.retry_at = Some(Instant::now() + Duration::from_secs(30));
        }
        if let Some(active) = &mut self.indicator
            && !active.update(&panel_label(state))
        {
            self.indicator = None;
        }
    }
}
