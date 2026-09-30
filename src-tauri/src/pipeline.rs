
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

struct Sidecar {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
}

pub struct Pool {
    exe: PathBuf,
    tessdata: PathBuf,
    idle: Mutex<Vec<Sidecar>>,
}

impl Pool {
    pub fn new(exe: PathBuf, tessdata: PathBuf) -> Self {
        Self {
            exe,
            tessdata,
            idle: Mutex::new(Vec::new()),
        }
    }

    fn spawn(&self) -> io::Result<Sidecar> {
        let mut child = Command::new(&self.exe)
            .env("TESSDATA_PREFIX", &self.tessdata)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdin = child.stdin.take().ok_or(io::ErrorKind::BrokenPipe)?;
        let stdout = child.stdout.take().ok_or(io::ErrorKind::BrokenPipe)?;
        Ok(Sidecar {
            child,
            stdin,
            stdout,
        })
    }

    pub fn call(&self, frames: &[u8]) -> io::Result<Vec<u8>> {
        let idle = self.idle.lock().unwrap_or_else(|e| e.into_inner()).pop();
        let mut sidecar = match idle {
            Some(sidecar) => sidecar,
            None => self.spawn()?,
        };
        let reply = exchange(&mut sidecar, frames);
        if reply.is_ok() {
            self.idle
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(sidecar);
        } else {
            let _ = sidecar.child.kill();
            let _ = sidecar.child.wait();
        }
        reply
    }
}

fn exchange(sidecar: &mut Sidecar, frames: &[u8]) -> io::Result<Vec<u8>> {
    sidecar.stdin.write_all(frames)?;
    sidecar.stdin.flush()?;
    let mut len = [0u8; 4];
    sidecar.stdout.read_exact(&mut len)?;
    let mut reply = vec![0; u32::from_le_bytes(len) as usize];
    sidecar.stdout.read_exact(&mut reply)?;
    Ok(reply)
}

pub fn is_two_frames(body: &[u8]) -> bool {
    let len = |at: usize| {
        body.get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    match len(0).and_then(|header| len(4 + header).map(|rgba| 8 + header + rgba)) {
        Some(total) => total == body.len(),
        None => false,
    }
}

pub fn memory_gb() -> Option<f64> {
    #[cfg(target_os = "linux")]
    {
        let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
        let kb: f64 = meminfo
            .lines()
            .find_map(|line| line.strip_prefix("MemTotal:"))?
            .trim()
            .trim_end_matches("kB")
            .trim()
            .parse()
            .ok()?;
        Some(kb / (1024.0 * 1024.0))
    }
    #[cfg(target_os = "macos")]
    {
        let out = Command::new("/usr/sbin/sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok()?;
        let bytes: f64 = String::from_utf8(out.stdout).ok()?.trim().parse().ok()?;
        Some(bytes / (1024.0 * 1024.0 * 1024.0))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::is_two_frames;

    #[test]
    fn frames_must_fill_the_body_exactly() {
        let body = [2, 0, 0, 0, b'{', b'}', 1, 0, 0, 0, 9];
        assert!(is_two_frames(&body));
        assert!(!is_two_frames(&body[..10]));
        assert!(!is_two_frames(&[body.as_slice(), &[0]].concat()));
        assert!(!is_two_frames(&[]));
    }
}
