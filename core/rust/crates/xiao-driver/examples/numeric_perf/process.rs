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

/// 复用冻结校准上限，控制器和独立协议测试使用同一常量。
pub use super::calibration::MAX_REPEAT;

/// 一次实际批次的原始总耗时与OS读数。
#[derive(Clone, Debug, Serialize)]
pub struct Iteration {
    /// 这一批实际执行的完整语言调用次数。
    pub repeats: usize,
    /// 整个批次的原始单调钟总耗时。
    pub seconds: f64,
    /// 批次结束时OS原始读数，缺失不能声称通过。
    pub memory: Option<Memory>,
}

/// 一个独立进程的一次预热、全部校准与正式样本。
#[derive(Clone, Debug, Serialize)]
pub struct Batch {
    /// 实际新建的测量子进程ID。
    pub pid: u32,
    /// 新建进程后的Unix纳秒时间，用于区分PID复用。
    pub started_unix_nanoseconds: u128,
    /// 逐行原始协议输出文件，独占创建。
    pub stdout: String,
    /// 实际子进程错误输出文件，独占创建。
    pub stderr: String,
    /// Runtime初始化后、输入构造前的同PID读数。
    pub baseline: Option<Memory>,
    /// 从repeat=1开始的全部实际校准样本。
    pub calibration: Vec<Iteration>,
    /// 最后保留的校准批次，作为正式样本。
    pub sample: Iteration,
    /// 进程OS高水位减当前驻留基线的保守字节增量。
    pub resident_increment_upper_bytes: Option<u64>,
    /// 正式样本是否达到至少50ms。
    pub calibrated: bool,
}

/// 子进程拥有全部管道；任一失败都终止并回收进程，同时保留原始日志。
struct Process {
    /// 当前批次独占的实际OS子进程。
    child: Child,
    /// 只发送规模和校准次数的协议输入。
    input: Option<ChildStdin>,
    /// 原始stdout解析后的有界等待事件入口。
    events: Receiver<Result<serde_json::Value, String>>,
    /// 回收前必须结束的stdout/stderr保存线程。
    readers: Vec<JoinHandle<()>>,
}

impl Drop for Process {
    /// 任何错误路径都终止并回收子进程，再收齐原始管道日志。
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
    /// 在60秒看门狗内读取下一条声明的协议事件。
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
    /// 发送完整配置行并立即flush，避免管道缓冲影响等待。
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
