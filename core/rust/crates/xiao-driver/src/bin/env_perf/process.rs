//! 子进程校准、看门狗与逐批原始记录。

use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{calibration, memory::Memory};

/// 单个采样进程的总体看门狗期限。
const TIMEOUT: Duration = Duration::from_secs(120);

/// 一次校准轮次的重复次数、总时长和驻留内存读数。
#[derive(Serialize, Deserialize)]
pub struct Sample {
    /// 本轮实际重复次数。
    pub repeats: usize,
    /// 本轮所有重复的总秒数。
    pub seconds: f64,
    /// 本轮结束时的 OS 驻留内存。
    pub memory: Option<Memory>,
}

/// 一个子进程从预热到退出保留的全部原始测量状态。
#[derive(Default, Serialize)]
pub struct Trial {
    /// 采样子进程 PID。
    pub pid: Option<u32>,
    /// 子进程启动时的 Unix 纳秒时间戳。
    pub started_unix_ns: Option<String>,
    /// 子进程进入用户代码前的内存基线。
    pub baseline: Option<Memory>,
    /// 预热轮次。
    pub warmup: Option<Sample>,
    /// 按顺序执行的所有校准轮次。
    pub calibration: Vec<Sample>,
    /// 子进程退出码。
    pub exit: Option<i32>,
    /// 启动、校准或退出阶段的失败原因。
    pub error: Option<String>,
}

impl Trial {
    /// 返回最后一个校准轮次作为本批正式样本。
    pub fn sample(&self) -> Option<&Sample> {
        self.calibration.last()
    }

    /// 返回相对基线的驻留内存增量上界。
    pub fn peak_increment(&self) -> Option<u64> {
        let baseline = self.baseline.as_ref()?;
        let warmup = self.warmup.as_ref()?.memory.as_ref()?;
        let mut peak = baseline.peak_bytes.max(warmup.peak_bytes);
        for sample in &self.calibration {
            peak = peak.max(sample.memory.as_ref()?.peak_bytes);
        }
        Some(peak.saturating_sub(baseline.current_bytes))
    }
}

/// 启动独立采样子进程，完成预热、校准并收集原始日志。
pub fn measure(
    executable: &Path,
    args: &[OsString],
    task: &Path,
    stdout: &Path,
    stderr: &Path,
    probe: bool,
) -> Trial {
    let mut trial = Trial::default();
    let launched = Command::new(executable)
        .args(args)
        .current_dir(task)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(File::create_new(stderr).unwrap())
        .spawn();
    let mut log = File::create_new(stdout).unwrap();
    let mut child = match launched {
        Ok(child) => child,
        Err(error) => {
            trial.error = Some(format!("启动失败 {error}"));
            return trial;
        }
    };
    trial.pid = Some(child.id());
    trial.started_unix_ns = Some(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string(),
    );
    let (send, receive) = channel();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let reader = std::thread::spawn(move || -> io::Result<()> {
        loop {
            let mut line = Vec::new();
            if output.read_until(b'\n', &mut line)? == 0 {
                return Ok(());
            }
            log.write_all(&line)?;
            let _ = send.send(line);
        }
    });
    if let Err(error) = calibrate(&mut child, &receive, &mut trial, probe) {
        trial.error = Some(error);
    }
    child.stdin.take();
    match finish(&mut child) {
        Ok(status) => trial.exit = status.code(),
        Err(error) => {
            trial.error.get_or_insert(error);
            trial.exit = child.wait().ok().and_then(|status| status.code());
        }
    }
    if let Err(error) = reader.join().expect("收集测量输出的线程") {
        trial.error.get_or_insert_with(|| error.to_string());
    }
    if trial.exit != Some(0) || std::fs::metadata(stderr).unwrap().len() != 0 {
        trial
            .error
            .get_or_insert_with(|| "子进程异常退出或产生诊断，见原始日志".into());
    }
    trial
}

/// 等待并校验子进程发出的指定 JSON 事件。
fn event(receive: &Receiver<Vec<u8>>, expected: &str) -> Result<Value, String> {
    let bytes = receive
        .recv_timeout(TIMEOUT)
        .map_err(|error| format!("读取 {expected} 失败 {error}"))?;
    let event: Value = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if event["event"] != expected {
        return Err(format!("预期 {expected}，收到 {event}"));
    }
    Ok(event)
}

/// 执行预热和冻结的重复次数校准，并保存每轮原始样本。
fn calibrate(
    child: &mut Child,
    receive: &Receiver<Vec<u8>>,
    trial: &mut Trial,
    probe: bool,
) -> Result<(), String> {
    let baseline = event(receive, "baseline")?;
    if baseline["pid"].as_u64() != Some(u64::from(child.id())) {
        return Err("基线 PID 与采样进程不一致".into());
    }
    trial.baseline =
        serde_json::from_value(baseline["memory"].clone()).map_err(|error| error.to_string())?;
    let input = child.stdin.as_mut().unwrap();
    let mut collect = |repeats| -> Result<Sample, String> {
        writeln!(input, "{repeats}").map_err(|error| error.to_string())?;
        let sample: Sample =
            serde_json::from_value(event(receive, "sample")?).map_err(|error| error.to_string())?;
        if sample.repeats != repeats || !sample.seconds.is_finite() || sample.seconds < 0.0 {
            return Err("采样次数或时钟读数无效".into());
        }
        Ok(sample)
    };
    trial.warmup = Some(collect(1)?);
    let mut repeats = 1;
    if !probe {
        loop {
            let sample = collect(repeats)?;
            let seconds = sample.seconds;
            trial.calibration.push(sample);
            if calibration::finished(repeats, seconds) {
                break;
            }
            repeats = calibration::next_repeat(repeats, seconds);
        }
    }
    writeln!(input, "0").map_err(|error| error.to_string())
}

/// 在总体期限内等待子进程退出，超时则终止它。
fn finish(child: &mut Child) -> Result<std::process::ExitStatus, String> {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return Ok(status);
        }
        if start.elapsed() >= TIMEOUT {
            child.kill().map_err(|error| error.to_string())?;
            return Err("测量进程退出超时".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
