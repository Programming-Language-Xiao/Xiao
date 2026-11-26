use super::*;
use xiao_intrinsics::CAPABILITY_FILESYSTEM;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
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
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn arg(path: &Path) -> RuntimeValue {
    RuntimeValue::new_string(path.to_str().unwrap()).unwrap()
}

fn call(operation: Operation, arguments: &[&RuntimeValue]) -> Result<RuntimeValue, ()> {
    execute(operation, arguments, CAPABILITY_FILESYSTEM, || Ok(()))
}

fn code(error: &Failure<()>) -> &str {
    match error {
        Failure::Runtime(error) => error.code(),
        Failure::Interrupted { .. } => panic!("预期文件系统错误"),
    }
}

#[test]
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
}

#[test]
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
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        File::create(directory.0.join(std::ffi::OsStr::from_bytes(&[0xff]))).unwrap();
        assert_eq!(
            code(&call(Operation::ListDir, &[&path]).unwrap_err()),
            "X06-RUNTIME-012"
        );
    }
}

#[test]
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
    call(Operation::Remove, &[&arg(&child)]).unwrap();
    call(Operation::Remove, &[&target_arg]).unwrap();
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 0);
}

#[test]
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
