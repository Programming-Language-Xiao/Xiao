//! 新建测量进程的双向协议、有界校准与逐批原始证据。

use super::memory::Memory;
use serde::Serialize;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub use super::calibration::MAX_REPEAT;

/// 一次实际批次的原始总耗时与OS读数。
#[derive(Clone, Debug, Serialize)]
pub struct Iteration {
    pub repeats: usize,
    pub seconds: f64,
    pub memory: Option<Memory>,
}

/// 一个独立进程的一次预热、全部校准与正式样本。
#[derive(Clone, Debug, Serialize)]
pub struct Batch {
    pub pid: u32,
    pub started_unix_nanoseconds: u128,
    pub stdout: String,
    pub stderr: String,
    pub baseline: Option<Memory>,
    pub calibration: Vec<Iteration>,
    pub sample: Iteration,
    pub resident_increment_upper_bytes: Option<u64>,
    pub calibrated: bool,
}

/// 子进程拥有全部管道；任一失败都终止并回收进程，同时保留原始日志。
struct Process {
    child: Child,
    input: Option<ChildStdin>,
    events: Receiver<Result<serde_json::Value, String>>,
    readers: Vec<JoinHandle<()>>,
}

impl Drop for Process {
    fn drop(&mut self) {
        self.input.take();
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
    }
}

impl Process {
    fn event(&self, expected: &str) -> Result<serde_json::Value, String> {
        let value = self
            .events
            .recv_timeout(Duration::from_secs(60))
            .map_err(|error| format!("A6进程看门狗/管道失败：{error}"))??;
        if value["event"] != expected {
            return Err(format!("期望{expected}，实际{value}"));
        }
        Ok(value)
    }
    fn write(&mut self, text: &str) -> Result<(), String> {
        self.input
            .as_mut()
            .unwrap()
            .write_all(text.as_bytes())
            .and_then(|()| self.input.as_mut().unwrap().flush())
            .map_err(|error| error.to_string())
    }
}

/// 同一PID内先取初始化后基线，再构造输入并预热一次，之后驱动全部真实批次。
pub fn batch(
    executable: &Path,
    args: &[String],
    directory: &Path,
    case: &str,
    profile: &str,
    index: usize,
) -> Result<Batch, String> {
    let stdout = directory.join(format!("{case}-{profile}-batch{index}.stdout.jsonl"));
    let stderr = directory.join(format!("{case}-{profile}-batch{index}.stderr.log"));
    let out_file = File::create_new(&stdout).map_err(|error| error.to_string())?;
    let err_file = File::create_new(&stderr).map_err(|error| error.to_string())?;
    let start_path = directory.join(format!("{case}-{profile}-batch{index}.process.json"));
    let mut process_file = File::create_new(start_path).map_err(|error| error.to_string())?;
    let mut child = Command::new(executable)
        .args(args)
        .current_dir(directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let pid = child.id();
    let started_unix_nanoseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    if let Err(error) = process_file.write_all(&serde_json::to_vec_pretty(&serde_json::json!({"pid":pid,"started_unix_nanoseconds":started_unix_nanoseconds.to_string(),"executable":executable,"argv":args,"case":case,"profile":profile,"batch":index,"stdout":stdout,"stderr":stderr})).unwrap()) {
        let _=child.kill(); let _=child.wait(); return Err(error.to_string());
    }
    let input = child.stdin.take();
    let output = child.stdout.take().unwrap();
    let mut errors = child.stderr.take().unwrap();
    let (send, events) = channel();
    let readers = vec![
        std::thread::spawn(move || {
            let mut file = out_file;
            for line in BufReader::new(output).lines() {
                let parsed = match line {
                    Ok(line) => {
                        writeln!(file, "{line}").unwrap();
                        serde_json::from_str(&line).map_err(|error| error.to_string())
                    }
                    Err(error) => Err(error.to_string()),
                };
                if send.send(parsed).is_err() {
                    break;
                }
            }
        }),
        std::thread::spawn(move || {
            let mut file = err_file;
            let _ = std::io::copy(&mut errors, &mut file);
        }),
    ];
    let mut process = Process {
        child,
        input,
        events,
        readers,
    };
    process.write(&format!("{case}\n{profile}\n"))?;
    let base = process.event("baseline")?;
    if base["pid"].as_u64() != Some(u64::from(pid)) {
        return Err("基线PID不属于当前新进程".into());
    }
    let baseline: Option<Memory> =
        serde_json::from_value(base["memory"].clone()).map_err(|error| error.to_string())?;
    process.event("validated")?;
    let ready = process.event("ready")?;
    if ready["pid"].as_u64() != Some(u64::from(pid)) {
        return Err("预热PID不属于当前新进程".into());
    }
    let mut repeats = 1;
    let mut calibration = Vec::new();
    loop {
        process.write(&format!("{repeats}\n"))?;
        let event = process.event("sample")?;
        process.event("validated")?;
        let seconds = event["seconds"].as_f64().ok_or("缺少有效时长")?;
        if !seconds.is_finite() || seconds < 0.0 {
            return Err("时长必须非负有限".into());
        }
        let memory =
            serde_json::from_value(event["memory"].clone()).map_err(|error| error.to_string())?;
        calibration.push(Iteration {
            repeats,
            seconds,
            memory,
        });
        if super::calibration::finished(repeats, seconds) {
            break;
        }
        repeats = super::calibration::next_repeat(repeats, seconds);
    }
    let sample = calibration.last().unwrap().clone();
    let calibrated = sample.seconds >= 0.05;
    let memory = baseline.as_ref().and_then(|base| {
        let peaks: Option<Vec<_>> = calibration
            .iter()
            .map(|iteration| iteration.memory.as_ref().map(|memory| memory.peak_bytes))
            .collect();
        peaks.map(|peaks| {
            peaks
                .into_iter()
                .max()
                .unwrap_or(base.peak_bytes)
                .max(base.peak_bytes)
                .saturating_sub(base.current_bytes)
        })
    });
    process.write("0\n")?;
    process.event("done")?;
    process.input.take();
    let deadline = Instant::now();
    let status = loop {
        if let Some(status) = process
            .child
            .try_wait()
            .map_err(|error| error.to_string())?
        {
            break status;
        }
        if deadline.elapsed() > Duration::from_secs(60) {
            return Err("测量进程退出超时".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    if !status.success() {
        return Err(format!("测量进程退出码{status}"));
    }
    drop(process);
    let mut errors = String::new();
    File::open(&stderr)
        .unwrap()
        .read_to_string(&mut errors)
        .unwrap();
    if !errors.is_empty() {
        return Err(format!("测量进程stderr：{errors}"));
    }
    Ok(Batch {
        pid,
        started_unix_nanoseconds,
        stdout: stdout.display().to_string(),
        stderr: stderr.display().to_string(),
        baseline,
        calibration,
        sample,
        resident_increment_upper_bytes: memory,
        calibrated,
    })
}
