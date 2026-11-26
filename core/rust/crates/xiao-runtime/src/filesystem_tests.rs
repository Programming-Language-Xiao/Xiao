//! 文件系统共享内核的真实宿主与受控边界回归。

use super::*;
use xiao_intrinsics::CAPABILITY_FILESYSTEM;

/// 为每个测试创建并回收独占目录。
struct Scratch(PathBuf);

impl Scratch {
    /// 排他创建临时目录，重复运行与并发测试互相隔离。
    fn new() -> Self {
        /// 在同一测试进程内区分各个任务。
        static NEXT: AtomicU32 = AtomicU32::new(0);
        loop {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("xiao-fs-test-{}-{id}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("创建测试目录失败：{error}"),
            }
        }
    }
}

impl Drop for Scratch {
    /// 回收本夹具创建的目录和其中的文件。
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

/// 把独占目录中的路径封装为语言字符串。
fn arg(path: &Path) -> RuntimeValue {
    RuntimeValue::new_string(path.to_str().unwrap()).unwrap()
}

/// 使用 filesystem 授权执行共享入口。
fn call(operation: Operation, arguments: &[&RuntimeValue]) -> Result<RuntimeValue, ()> {
    execute(operation, arguments, CAPABILITY_FILESYSTEM, || Ok(()))
}

/// 读取可捕获错误的稳定身份。
fn code(error: &Failure<()>) -> &str {
    match error {
        Failure::Runtime(error) => error.code(),
        Failure::Interrupted { .. } => panic!("预期文件系统错误"),
    }
}

#[test]
/// 复合错误按类型、能力、参数与预算顺序交付。
fn fs_error_order_and_budgets_precede_host_calls() {
    let checkpoint = || panic!("拒绝无效实参前不应进入安全点");
    let empty = RuntimeValue::new_string("").unwrap();
    let invalid = RuntimeValue::new_string("a\0b").unwrap();
    let limit = RuntimeValue::new_string("a".repeat(PATH_BYTES + 1)).unwrap();
    let too_big = RuntimeValue::new_string("a".repeat(TEXT_BYTES + 1)).unwrap();
    let yes = RuntimeValue::Bool(true);
    for (operation, args, capabilities, expected) in [
        (
            Operation::Rename,
            &[&empty, &invalid, &RuntimeValue::Int(0)][..],
            0,
            "X06-RUNTIME-002",
        ),
        (Operation::Metadata, &[&empty][..], 0, "X06-RUNTIME-026"),
        (
            Operation::Rename,
            &[&limit, &invalid, &yes][..],
            CAPABILITY_FILESYSTEM,
            "X06-RUNTIME-012",
        ),
        (
            Operation::Remove,
            &[&limit][..],
            CAPABILITY_FILESYSTEM,
            "FS_LIMIT_EXCEEDED",
        ),
        (
            Operation::WriteText,
            &[&limit, &too_big, &yes][..],
            CAPABILITY_FILESYSTEM,
            "FS_LIMIT_EXCEEDED",
        ),
        (
            Operation::WriteText,
            &[&invalid, &too_big, &yes][..],
            CAPABILITY_FILESYSTEM,
            "X06-RUNTIME-012",
        ),
    ] {
        assert_eq!(
            code(&execute(operation, args, capabilities, checkpoint).unwrap_err()),
            expected
        );
    }
    let accepted = RuntimeValue::new_string("a".repeat(PATH_BYTES)).unwrap();
    assert!(matches!(
        execute(
            Operation::Metadata,
            &[&accepted],
            CAPABILITY_FILESYSTEM,
            || Err(())
        ),
        Err(Failure::Interrupted { .. })
    ));
}

#[test]
/// 并发写入同一名称时只保留一个完整结果。
fn fs_atomic_no_replace_has_one_winner_and_no_staging_files() {
    let directory = Scratch::new();
    let target = directory.0.join("target.txt");
    let barrier = std::sync::Barrier::new(8);
    let wins = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let target = &target;
                let barrier = &barrier;
                scope.spawn(move || {
                    let text = format!("内容{i}\n").repeat(4096);
                    barrier.wait();
                    let result = write_text(
                        target.to_str().unwrap(),
                        &text,
                        false,
                        &mut Control::new(|| Ok(())),
                    );
                    match result {
                        Ok(()) => Some(text),
                        Err(error) => {
                            assert_eq!(code(&error), "FS_EXISTS");
                            None
                        }
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .filter_map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(wins.len(), 1);
    assert_eq!(fs::read_to_string(&target).unwrap(), wins[0]);
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
    let existing = directory.0.join("existing");
    fs::create_dir(&existing).unwrap();
    let path = arg(&existing);
    let text = RuntimeValue::new_string("x").unwrap();
    let failure = call(
        Operation::WriteText,
        &[&path, &text, &RuntimeValue::Bool(false)],
    )
    .unwrap_err();
    assert_eq!(code(&failure), "FS_EXISTS");
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 2);
}

#[test]
/// 写入中断保留旧目标。超时与运行取消使用各自的通道。
fn fs_write_precommit_cancel_and_deadline_clean_up() {
    let directory = Scratch::new();
    let target = directory.0.join("old.txt");
    fs::write(&target, "old").unwrap();
    let text = "z".repeat(TEXT_BYTES);
    let mut calls = 0;
    let mut control = Control::new(|| {
        calls += 1;
        if calls == 7 { Err("cancel") } else { Ok(()) }
    });
    assert!(matches!(
        write_text(target.to_str().unwrap(), &text, true, &mut control),
        Err(Failure::Interrupted {
            control: "cancel",
            cleanup: None
        })
    ));
    assert_eq!(fs::read_to_string(&target).unwrap(), "old");
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
    let mut checks = 0;
    let mut control = Control::new(|| {
        checks += 1;
        if checks != 5 {
            return Ok(());
        }
        // 提交前用目录模拟清理失败，保留旧目标和根取消原因。
        let temporary = fs::read_dir(&directory.0)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| path != &target)
            .unwrap();
        fs::remove_file(&temporary).unwrap();
        fs::create_dir(&temporary).unwrap();
        Err(temporary)
    });
    let failure = write_text(target.to_str().unwrap(), "new", true, &mut control).unwrap_err();
    let Failure::Interrupted {
        control: temporary,
        cleanup: Some(cleanup),
    } = failure
    else {
        panic!("应保留取消及其清理原因");
    };
    assert_eq!(cleanup.code(), "FS_IO_FAILED");
    assert!(cleanup.params().contains_key("os_error"));
    fs::remove_dir(temporary).unwrap();
    assert_eq!(fs::read_to_string(&target).unwrap(), "old");
    write_text(
        target.to_str().unwrap(),
        &text,
        true,
        &mut Control::new(|| Ok::<_, ()>(())),
    )
    .unwrap();
    assert_eq!(fs::read_to_string(&target).unwrap(), text);

    let started = Instant::now() - OPERATION_LIMIT;
    let mut timeout = Control {
        started,
        checkpoint: || Ok(()),
    };
    assert_eq!(code(&timeout.check().unwrap_err()), "FS_TIMEOUT");
    let mut cancelled = Control {
        started,
        checkpoint: || Err(()),
    };
    assert!(matches!(
        cancelled.check(),
        Err(Failure::Interrupted { .. })
    ));
    let mut checks = 0;
    // 进入提交后直接交付实际结果。
    let mut control = Control::new(|| {
        checks += 1;
        if checks > 5 { Err(()) } else { Ok(()) }
    });
    write_text(target.to_str().unwrap(), "commit", true, &mut control).unwrap();
    assert_eq!(checks, 5);
    assert_eq!(fs::read_to_string(&target).unwrap(), "commit");
}

#[test]
/// 核对宿主名称顺序与三项枚举预算。
fn fs_enumeration_sorts_and_is_all_or_nothing() {
    let directory = Scratch::new();
    let path = arg(&directory.0);
    for name in ["z", "é", "a"] {
        File::create(directory.0.join(name)).unwrap();
    }
    let RuntimeValue::Array(names) = call(Operation::ListDir, &[&path]).unwrap() else {
        panic!("枚举返回数组");
    };
    assert_eq!(
        names
            .with_elements(|items| items
                .iter()
                .map(|item| item.display_text().unwrap())
                .collect::<Vec<_>>())
            .unwrap(),
        ["a", "z", "é"]
    );
    let budget = Scratch::new();
    let budget_path = arg(&budget.0);
    for i in 0..DIR_NAMES {
        File::create(budget.0.join(format!("{i:04}{}", "a".repeat(60)))).unwrap();
    }
    let RuntimeValue::Array(names) = call(Operation::ListDir, &[&budget_path]).unwrap() else {
        panic!("枚举返回数组");
    };
    assert_eq!(names.len(), DIR_NAMES);
    let first = budget.0.join(format!("0000{}", "a".repeat(60)));
    let extended = budget.0.join(format!("0000{}", "a".repeat(61)));
    fs::rename(&first, &extended).unwrap();
    assert_eq!(
        code(&call(Operation::ListDir, &[&budget_path]).unwrap_err()),
        "FS_LIMIT_EXCEEDED"
    );
    fs::rename(&extended, &first).unwrap();
    File::create(budget.0.join("over")).unwrap();
    assert_eq!(
        code(&call(Operation::ListDir, &[&budget_path]).unwrap_err()),
        "FS_LIMIT_EXCEEDED"
    );
    assert_eq!(
        directory_name(std::ffi::OsString::from("x".repeat(PATH_BYTES)))
            .unwrap()
            .len(),
        PATH_BYTES
    );
    assert_eq!(
        directory_name(std::ffi::OsString::from("x".repeat(PATH_BYTES + 1)))
            .unwrap_err()
            .code(),
        "FS_LIMIT_EXCEEDED"
    );
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        let invalid = std::ffi::OsString::from_wide(&[0xd800]);
        assert_eq!(
            directory_name(invalid).unwrap_err().code(),
            "X06-RUNTIME-012"
        );
        println!("B3-3-FS-UNICODE adapter=rejected encoding=utf16");
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let invalid = std::ffi::OsStr::from_bytes(&[0xff]);
        assert_eq!(
            directory_name(invalid.to_owned()).unwrap_err().code(),
            "X06-RUNTIME-012"
        );
        match File::create(directory.0.join(invalid)) {
            Ok(_) => {
                assert_eq!(
                    code(&call(Operation::ListDir, &[&path]).unwrap_err()),
                    "X06-RUNTIME-012"
                );
                println!("B3-3-FS-UNICODE adapter=rejected filesystem=accepted");
            }
            #[cfg(target_os = "macos")]
            Err(error) if error.raw_os_error() == Some(libc::EILSEQ) => {
                println!("B3-3-FS-UNICODE adapter=rejected filesystem=EILSEQ");
            }
            Err(error) => panic!("创建非法名称夹具失败：{error}"),
        }
    }
}

#[test]
/// 删除非空目录会失败，重命名保留目录中的文件。
fn fs_rename_and_remove_preserve_nonempty_directories() {
    let directory = Scratch::new();
    let source = directory.0.join("source");
    let target = directory.0.join("target");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("child"), b"abc").unwrap();
    fs::create_dir(&target).unwrap();
    let source_arg = arg(&source);
    let target_arg = arg(&target);
    assert_eq!(
        code(&call(Operation::Remove, &[&source_arg]).unwrap_err()),
        "FS_IO_FAILED"
    );
    assert_eq!(
        code(
            &call(
                Operation::Rename,
                &[&source_arg, &target_arg, &RuntimeValue::Bool(false)]
            )
            .unwrap_err()
        ),
        "FS_EXISTS"
    );
    call(Operation::Remove, &[&target_arg]).unwrap();
    call(
        Operation::Rename,
        &[&source_arg, &target_arg, &RuntimeValue::Bool(false)],
    )
    .unwrap();
    let child = target.join("child");
    let RuntimeValue::DictTable(info) = call(Operation::Metadata, &[&arg(&child)]).unwrap() else {
        panic!("元数据返回字典");
    };
    assert_eq!(info.len(), 3);
    assert_eq!(info.value("kind").unwrap(), Some(RuntimeValue::Int(1)));
    assert_eq!(info.value("size").unwrap(), Some(RuntimeValue::Int(3)));
    assert!(
        matches!(info.value("modified_ms").unwrap(), Some(RuntimeValue::Int(value)) if value > 0)
    );
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let fifo = target.join("fifo");
        let fifo_path = CString::new(fifo.as_os_str().as_bytes()).unwrap();
        // SAFETY: 操作只创建任务目录内的 FIFO。
        assert_eq!(unsafe { libc::mkfifo(fifo_path.as_ptr(), 0o600) }, 0);
        let RuntimeValue::DictTable(info) = call(Operation::Metadata, &[&arg(&fifo)]).unwrap()
        else {
            panic!("特殊目录项的元数据返回字典");
        };
        assert_eq!(info.value("kind").unwrap(), Some(RuntimeValue::Int(4)));
        assert_eq!(info.value("size").unwrap(), Some(RuntimeValue::Int(-1)));
        assert_eq!(
            code(&call(Operation::Remove, &[&arg(&fifo)]).unwrap_err()),
            "FS_UNSUPPORTED"
        );
        fs::remove_file(fifo).unwrap();
    }
    call(Operation::Remove, &[&arg(&child)]).unwrap();
    call(Operation::Remove, &[&target_arg]).unwrap();
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 0);
}

#[test]
/// 悬空链接占用目录项，覆盖及删除作用于链接本身。
fn fs_operations_handle_the_link_entry_and_preserve_its_target() {
    let directory = Scratch::new();
    let target = directory.0.join("target");
    let link = directory.0.join("link");
    let moved_link = directory.0.join("moved");
    let text = RuntimeValue::new_string("new").unwrap();
    let no = RuntimeValue::Bool(false);
    let yes = RuntimeValue::Bool(true);
    #[cfg(unix)]
    use std::os::unix::fs::symlink as symlink_file;
    #[cfg(windows)]
    use std::os::windows::fs::symlink_file;
    if let Err(error) = symlink_file(&target, &link) {
        // 某些 Windows 账户无法创建链接，专项采集会检查标记并单列。
        #[cfg(windows)]
        if error.raw_os_error() == Some(1314) {
            eprintln!("B3-3-FS-SYMLINK unverifiable reason=ERROR_PRIVILEGE_NOT_HELD");
            return;
        }
        panic!("创建符号链接失败：{error}");
    }
    let link_arg = arg(&link);
    assert_eq!(
        code(&call(Operation::WriteText, &[&link_arg, &text, &no]).unwrap_err()),
        "FS_EXISTS"
    );
    let RuntimeValue::DictTable(metadata) = call(Operation::Metadata, &[&link_arg]).unwrap() else {
        panic!("元数据返回字典");
    };
    assert_eq!(metadata.value("kind").unwrap(), Some(RuntimeValue::Int(3)));
    assert_eq!(metadata.value("size").unwrap(), Some(RuntimeValue::Int(-1)));
    assert!(!target.exists());
    fs::write(&target, "original").unwrap();
    call(Operation::Rename, &[&link_arg, &arg(&moved_link), &no]).unwrap();
    assert_eq!(fs::read_link(&moved_link).unwrap(), target);
    call(Operation::Remove, &[&arg(&moved_link)]).unwrap();
    symlink_file(&target, &link).unwrap();
    call(Operation::WriteText, &[&link_arg, &text, &yes]).unwrap();
    assert_eq!(fs::read_to_string(&link).unwrap(), "new");
    assert_eq!(fs::read_to_string(&target).unwrap(), "original");
    assert!(!fs::symlink_metadata(&link).unwrap().is_symlink());
    println!("B3-3-FS-SYMLINK passed");
}

#[cfg(windows)]
#[test]
/// Windows 超长路径仍可完成原子禁止覆盖的提交。
fn fs_windows_long_paths_allow_atomic_no_replace() {
    let directory = Scratch::new();
    let parent = directory
        .0
        .join("a".repeat(100))
        .join("b".repeat(100))
        .join("c".repeat(100));
    fs::create_dir_all(&parent).unwrap();
    let target = parent.join("target.txt");
    let text = RuntimeValue::new_string("long path").unwrap();
    let path = arg(&target);
    call(
        Operation::WriteText,
        &[&path, &text, &RuntimeValue::Bool(false)],
    )
    .unwrap();
    assert_eq!(fs::read_to_string(&target).unwrap(), "long path");
    assert_eq!(fs::read_dir(&parent).unwrap().count(), 1);
}

#[cfg(target_os = "linux")]
#[test]
/// 两种覆盖策略都拒绝跨卷回退，并保留源文件。
fn fs_rename_refuses_cross_volume_copy() {
    use std::os::unix::fs::MetadataExt;
    let dir = Scratch::new();
    let memory_fs = Path::new("/dev/shm");
    if fs::metadata(memory_fs).is_err()
        || fs::metadata(&dir.0).unwrap().dev() == fs::metadata(memory_fs).unwrap().dev()
    {
        eprintln!("B3-3-FS-CROSS-VOLUME unverifiable");
        return;
    }
    let destination_dir = memory_fs.join(dir.0.file_name().unwrap());
    fs::create_dir(&destination_dir).unwrap();
    let destination_dir = Scratch(destination_dir);
    let src = dir.0.join("source");
    let dst = destination_dir.0.join("destination");
    fs::write(&src, b"keep").unwrap();
    for overwrite in [false, true] {
        assert_eq!(
            code(
                &call(
                    Operation::Rename,
                    &[&arg(&src), &arg(&dst), &RuntimeValue::Bool(overwrite)]
                )
                .unwrap_err()
            ),
            "FS_UNSUPPORTED"
        );
        assert_eq!(fs::read(&src).unwrap(), b"keep");
        assert!(!dst.exists());
    }
    println!("B3-3-FS-CROSS-VOLUME passed");
}
