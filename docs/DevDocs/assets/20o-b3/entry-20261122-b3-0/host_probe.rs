//! 只观察宿主 API。所有样例输入由当前探测进程构造。

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn isolated_child(role: &str) -> io::Result<Command> {
    let mut command = Command::new(env::current_exe()?);
    command.env_clear().arg(role);
    Ok(command)
}

#[cfg(unix)]
fn malformed_text() -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(vec![0xff])
}

#[cfg(windows)]
fn malformed_text() -> OsString {
    use std::os::windows::ffi::OsStringExt;
    OsString::from_wide(&[0xd800])
}

fn print_child(command: &mut Command) -> io::Result<()> {
    let output = command.output()?;
    if !output.status.success() || !output.stderr.is_empty() {
        return Err(io::Error::other(format!("child: {:?}", output)));
    }
    io::stdout().write_all(&output.stdout)
}

fn time_probe() -> io::Result<()> {
    println!(
        "wall_unix_ns={}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let mut previous = Instant::now();
    let mut smallest = None;
    let mut regressions = 0;
    for _ in 0..10_000 {
        let next = Instant::now();
        match next.checked_duration_since(previous) {
            Some(delta) if !delta.is_zero() => {
                smallest = Some(smallest.map_or(delta, |prior: Duration| prior.min(delta)));
            }
            None => regressions += 1,
            _ => {}
        }
        previous = next;
    }
    println!(
        "monotonic_min_observed_ns={:?} regressions={regressions}",
        smallest.map(|d| d.as_nanos())
    );
    for ms in [0, 1, 10] {
        let samples = (0..5)
            .map(|_| {
                let start = Instant::now();
                thread::sleep(Duration::from_millis(ms));
                start.elapsed().as_nanos()
            })
            .collect::<Vec<_>>();
        println!("sleep_requested_ms={ms} observed_ns={samples:?}");
    }
    let (ready, waiting) = mpsc::sync_channel(0);
    let (cancel, signal) = mpsc::channel();
    let waiter = thread::spawn(move || {
        ready.send(()).unwrap();
        signal.recv_timeout(Duration::from_secs(5))
    });
    waiting
        .recv_timeout(Duration::from_secs(5))
        .map_err(io::Error::other)?;
    let start = Instant::now();
    cancel.send(()).map_err(io::Error::other)?;
    println!(
        "host_wakeable_wait_received={} observed_ns={}",
        waiter.join().unwrap().is_ok(),
        start.elapsed().as_nanos()
    );
    let (_sender, receiver) = mpsc::channel::<()>();
    println!(
        "host_wait_deadline={}",
        receiver.recv_timeout(Duration::from_millis(1)) == Err(RecvTimeoutError::Timeout)
    );
    Ok(())
}

const ARGUMENTS: [&str; 9] = [
    "",
    "two words",
    "\"quoted\"",
    "trail\\",
    "--",
    "-O3",
    "--help",
    "-x",
    "雪",
];

fn argv_probe() -> io::Result<()> {
    print_child(
        isolated_child("argv-child")?
            .args(ARGUMENTS)
            .arg(malformed_text()),
    )
}

fn argv_child() {
    // 子进程的 argv[0] 由宿主填充。探测模式参数由父进程加入。
    let mut arguments = env::args_os();
    let has_argv0 = arguments.next().is_some_and(|value| !value.is_empty());
    arguments.next();
    let arguments = arguments.collect::<Vec<_>>();
    let valid = arguments
        .get(..ARGUMENTS.len())
        .is_some_and(|values| values.iter().eq(ARGUMENTS.iter().map(OsStr::new)));
    let malformed = arguments
        .get(ARGUMENTS.len())
        .is_some_and(|value| value.to_str().is_none() && *value == malformed_text());
    println!(
        "argv0_present={has_argv0} argv_count={} valid_exact={valid} malformed_roundtrip={malformed}",
        arguments.len()
    );
}

fn env_probe() -> io::Result<()> {
    let mut child = isolated_child("env-child")?;
    child
        .env("XIAO_B3_PROBE_VALUE", "值")
        .env("XIAO_B3_PROBE_CASE", "fixed")
        .env("XIAO_B3_PROBE_EMPTY", "")
        .env("XIAO_B3_PROBE_INVALID", malformed_text());
    println!("child_with_values");
    print_child(&mut child)?;
    child.env_remove("XIAO_B3_PROBE_VALUE");
    println!("child_with_value_removed");
    print_child(&mut child)
}

fn env_child() {
    let present = env::var_os("XIAO_B3_PROBE_VALUE").is_some_and(|value| value == "值");
    let case_folded = env::var_os("xiao_b3_probe_case").is_some_and(|value| value == "fixed");
    let empty = env::var_os("XIAO_B3_PROBE_EMPTY").is_some_and(|value| value.is_empty());
    let missing = env::var_os("XIAO_B3_PROBE_ABSENT").is_none();
    let invalid = env::var_os("XIAO_B3_PROBE_INVALID")
        .is_some_and(|value| value.to_str().is_none() && value == malformed_text());
    println!(
        "value_present={present} case_folded={case_folded} empty_present={empty} absent={missing} malformed_roundtrip={invalid}"
    );
}

#[cfg(unix)]
fn directory_link(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn directory_link(target: &Path, link: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

fn describe<T>(result: io::Result<T>) -> String {
    match result {
        Ok(_) => "success".into(),
        Err(error) => format!("{:?}/{:?}", error.kind(), error.raw_os_error()),
    }
}

fn fs_probe(root: &Path) -> io::Result<()> {
    fs::create_dir(root)?;
    let working = root.join("working");
    let branch = root.join("branch");
    fs::create_dir(&working)?;
    fs::create_dir_all(branch.join("child"))?;
    let destination = working.join("result.txt");
    let temporary = working.join("temporary.txt");
    fs::write(&destination, b"old")?;
    fs::write(&temporary, b"new")?;
    let exclusive = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination);
    println!(
        "create_new_existing={} original_intact={}",
        describe(exclusive),
        fs::read(&destination)? == b"old"
    );
    let replaced = fs::rename(&temporary, &destination);
    println!(
        "same_directory_replace={} new_content={} temporary_removed={}",
        describe(replaced),
        fs::read(&destination)? == b"new",
        !temporary.exists()
    );
    fs::write(&temporary, b"temporary")?;
    let failed = fs::rename(&temporary, &branch);
    println!(
        "replace_directory={} source_retained={}",
        describe(failed),
        temporary.is_file()
    );
    fs::remove_file(&temporary)?;
    fs::write(branch.join("marker"), b"resolved")?;
    let link = working.join("link");
    let result = directory_link(&branch.join("child"), &link);
    let available = result.is_ok();
    println!("directory_link={}", describe(result));
    if available {
        println!(
            "link_parent_read={:?}",
            fs::read(link.join("..").join("marker")).map(|data| data == b"resolved")
        );
    }
    let saved = fs::metadata(&destination)?.permissions();
    let mut permissions = saved.clone();
    permissions.set_readonly(true);
    fs::set_permissions(&destination, permissions)?;
    let readonly = OpenOptions::new().write(true).open(&destination);
    fs::set_permissions(&destination, saved)?;
    println!("readonly_open_for_write={}", describe(readonly));
    Ok(())
}

fn log_probe() -> io::Result<()> {
    let output = isolated_child("log-child")?.output()?;
    let mut expected = (0..4)
        .flat_map(|task| (0..12).map(move |row| format!("task={task} row={row}")))
        .collect::<Vec<_>>();
    expected.sort_unstable();
    let stdout = String::from_utf8(output.stdout).map_err(io::Error::other)?;
    let mut lines = stdout.lines().collect::<Vec<_>>();
    lines.sort_unstable();
    println!(
        "concurrent_stdout_records={} exact={} stderr_separate={} exit={}",
        lines.len(),
        lines == expected,
        output.stderr == b"stderr-marker\n",
        output.status
    );
    let mut child = isolated_child("log-closed-child")?
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    drop(child.stdout.take());
    child.stdin.take().unwrap().write_all(b"x\n")?;
    let output = child.wait_with_output()?;
    io::stdout().write_all(&output.stderr)?;
    println!("closed_stream_child={}", output.status);
    Ok(())
}

fn log_child() -> io::Result<()> {
    let threads = (0..4)
        .map(|task| {
            thread::spawn(move || -> io::Result<()> {
                for row in 0..12 {
                    let record = format!("task={task} row={row}\n");
                    io::stdout().lock().write_all(record.as_bytes())?;
                }
                Ok(())
            })
        })
        .collect::<Vec<_>>();
    for worker in threads {
        worker.join().unwrap()?;
    }
    io::stdout().flush()?;
    io::stderr().write_all(b"stderr-marker\n")
}

fn log_closed_child() -> io::Result<()> {
    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    let mut stream = io::stdout().lock();
    let result = stream
        .write_all(b"closed reader\n")
        .and_then(|()| stream.flush());
    eprintln!("closed_stdout_write={}", describe(result));
    Ok(())
}

fn main() -> io::Result<()> {
    match env::args_os()
        .nth(1)
        .and_then(|value| value.into_string().ok())
        .as_deref()
    {
        Some("time") => time_probe(),
        Some("argv") => argv_probe(),
        Some("argv-child") => {
            argv_child();
            Ok(())
        }
        Some("env") => env_probe(),
        Some("env-child") => {
            env_child();
            Ok(())
        }
        Some("fs") => fs_probe(&PathBuf::from(
            env::args_os()
                .nth(2)
                .ok_or_else(|| io::Error::other("缺少探测目录"))?,
        )),
        Some("log") => log_probe(),
        Some("log-child") => log_child(),
        Some("log-closed-child") => log_closed_child(),
        _ => Err(io::Error::other("未知探测项")),
    }
}
