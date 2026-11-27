//! 环境视图的接纳、错误顺序和局部提交。

use std::cell::Cell;
use std::convert::Infallible;

use xiao_intrinsics::DEFAULT_CAPABILITY_MASK;

use super::*;

/// 以拥有的 Runtime 字符串模拟已求值的实参。
fn text(value: &str) -> RuntimeValue {
    RuntimeValue::Str(StringHandle::new(value).unwrap())
}

/// 在已授权且无运行控制中断时调用原语。
fn run(
    snapshot: &EnvironmentSnapshot,
    operation: Operation,
    arguments: &[&str],
) -> Result<RuntimeValue, Failure<Infallible>> {
    let values: Vec<_> = arguments.iter().map(|arg| text(arg)).collect();
    let values: Vec<_> = values.iter().collect();
    snapshot.execute(operation, &values, CAPABILITY_ENVIRONMENT, || Ok(()))
}

/// 提取环境读取结果的独立数组。
fn array(value: RuntimeValue) -> ArrayHandle {
    let RuntimeValue::Array(array) = value else {
        panic!("环境读取必须返回数组");
    };
    array
}

/// 逐项核对数组持有的字符串。
fn strings(array: &ArrayHandle) -> Vec<String> {
    array
        .with_elements(|items| {
            items
                .iter()
                .map(|item| item.display_text().unwrap())
                .collect()
        })
        .unwrap()
}

/// 返回语言错误，遇到其他通道时使测试失败。
fn runtime_error<C: fmt::Debug>(result: Result<RuntimeValue, Failure<C>>) -> RuntimeError {
    match result.unwrap_err() {
        Failure::Runtime(error) => error,
        other => panic!("预期运行错误，收到 {other:?}"),
    }
}

#[test]
/// 区分缺失与空值，并验证输出和局部更新都独立拥有数据。
fn get_set_unset_return_owned_values() {
    let snapshot = EnvironmentSnapshot::default();
    assert!(array(run(&snapshot, Operation::Get, &["Name"]).unwrap()).is_empty());
    assert_eq!(
        run(&snapshot, Operation::Set, &["Name", ""]).unwrap(),
        RuntimeValue::None
    );
    let empty_value = array(run(&snapshot, Operation::Get, &["Name"]).unwrap());
    assert_eq!(strings(&empty_value), [""]);
    run(&snapshot, Operation::Set, &["Name", "new=value"]).unwrap();
    let owned = array(run(&snapshot, Operation::Get, &["Name"]).unwrap());
    let independent = array(run(&snapshot, Operation::Get, &["Name"]).unwrap());
    assert_eq!(strings(&owned), ["new=value"]);
    owned.push(text("local")).unwrap();
    assert_eq!(strings(&independent), ["new=value"]);
    assert_eq!(
        run(&snapshot, Operation::Unset, &["Name"]).unwrap(),
        RuntimeValue::Bool(true)
    );
    assert_eq!(
        run(&snapshot, Operation::Unset, &["Name"]).unwrap(),
        RuntimeValue::Bool(false)
    );
    assert!(array(run(&snapshot, Operation::Get, &["Name"]).unwrap()).is_empty());
    assert_eq!(strings(&owned), ["new=value", "local"]);
    assert_eq!(strings(&empty_value), [""]);
}

#[test]
/// 同时提供多种错误时按类型、权限、语法、预算和运行控制的顺序处理。
fn validates_types_authorization_arguments_and_budget_before_checkpoint() {
    let snapshot = EnvironmentSnapshot::from_pairs(&[("Name".into(), "old".into())]).unwrap();
    let name = text("Name");
    let invalid = text("bad=name");
    let oversized = text(&"x".repeat(MAX_VALUE_BYTES + 1));
    let int = RuntimeValue::Int(7);
    let denied = [
        (Operation::Get, vec![&invalid]),
        (Operation::Set, vec![&name, &oversized]),
        (Operation::Unset, vec![&name]),
    ];
    let never = || -> Result<(), Infallible> { panic!("拒绝应发生在安全点之前") };

    for (operation, arguments) in [
        (Operation::Get, vec![&int]),
        (Operation::Set, vec![&invalid, &int]),
        (Operation::Unset, Vec::new()),
    ] {
        assert_eq!(
            runtime_error(snapshot.execute(operation, &arguments, 0, never)).code(),
            "X06-RUNTIME-002"
        );
    }
    for (operation, arguments) in denied {
        for mask in [0, DEFAULT_CAPABILITY_MASK] {
            assert_eq!(
                runtime_error(snapshot.execute(operation, &arguments, mask, never)).code(),
                "X06-RUNTIME-026"
            );
        }
    }
    assert_eq!(
        runtime_error(snapshot.execute(
            Operation::Get,
            &[&name],
            CAPABILITY_ENVIRONMENT | (1 << 8),
            never
        ))
        .code(),
        INVALID_VALUE_CODE
    );
    assert_eq!(
        runtime_error(snapshot.execute(
            Operation::Set,
            &[&invalid, &oversized],
            CAPABILITY_ENVIRONMENT,
            never
        ))
        .code(),
        INVALID_VALUE_CODE
    );
    let quota = runtime_error(snapshot.execute(
        Operation::Set,
        &[&name, &oversized],
        CAPABILITY_ENVIRONMENT,
        never,
    ));
    assert_eq!(quota.code(), "ENV_LIMIT_EXCEEDED");
    assert_eq!(quota.kind(), RuntimeErrorKind::Resource);
    assert_eq!(
        strings(&array(run(&snapshot, Operation::Get, &["Name"]).unwrap())),
        ["old"]
    );
}

#[test]
/// 空名称、等号和 NUL 对快照与运行调用使用各自的错误身份。
fn rejects_invalid_names_and_values() {
    let snapshot = EnvironmentSnapshot::default();
    for name in ["", "na=me", "=C:", "na\0me"] {
        let error = EnvironmentSnapshot::from_pairs(&[(name.into(), "".into())]).unwrap_err();
        assert_eq!(error, SnapshotError::InvalidName);
        assert_eq!(error.code(), "X09-DRIVER-006");
        for operation in [Operation::Get, Operation::Unset] {
            assert_eq!(
                runtime_error(run(&snapshot, operation, &[name])).code(),
                INVALID_VALUE_CODE
            );
        }
        assert_eq!(
            runtime_error(run(&snapshot, Operation::Set, &[name, "value"])).code(),
            INVALID_VALUE_CODE
        );
    }
    let error = EnvironmentSnapshot::from_pairs(&[("Name".into(), "va\0lue".into())]).unwrap_err();
    assert_eq!(error, SnapshotError::InvalidValue);
    assert_eq!(error.code(), "X09-DRIVER-006");
    assert_eq!(
        runtime_error(run(&snapshot, Operation::Set, &["Name", "va\0lue"])).code(),
        INVALID_VALUE_CODE
    );
}

#[test]
/// 缺位与未知位都跳过来源，授权后的每个视图各调用一次来源。
fn gates_host_source_and_keeps_distinct_snapshots() {
    let calls = Cell::new(0);
    let read = || {
        let next = calls.get() + 1;
        calls.set(next);
        [("Name".into(), next.to_string().into())]
    };
    let empty = EnvironmentSnapshot::from_host(DEFAULT_CAPABILITY_MASK, read).unwrap();
    assert_eq!(calls.get(), 0);
    assert!(array(run(&empty, Operation::Get, &["Name"]).unwrap()).is_empty());
    assert!(matches!(
        EnvironmentSnapshot::from_host(CAPABILITY_ENVIRONMENT | (1 << 8), read),
        Err(SnapshotError::InvalidCapabilities(_))
    ));
    assert_eq!(calls.get(), 0);
    let first = EnvironmentSnapshot::from_host(CAPABILITY_ENVIRONMENT, read).unwrap();
    assert_eq!(calls.get(), 1);
    let second = EnvironmentSnapshot::from_host(CAPABILITY_ENVIRONMENT, read).unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(
        strings(&array(run(&first, Operation::Get, &["Name"]).unwrap())),
        ["1"]
    );
    assert_eq!(
        strings(&array(run(&second, Operation::Get, &["Name"]).unwrap())),
        ["2"]
    );
}

#[cfg(any(windows, unix))]
#[test]
/// 原生名称和值均严格解码，任一错误拒绝整份快照。
fn rejects_invalid_native_unicode() {
    #[cfg(windows)]
    let invalid = {
        use std::os::windows::ffi::OsStringExt;
        OsString::from_wide(&[0xd800])
    };
    #[cfg(unix)]
    let invalid = {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(vec![0xff])
    };
    for invalid_pair in [(invalid.clone(), "".into()), ("Name".into(), invalid)] {
        let error = EnvironmentSnapshot::from_host(CAPABILITY_ENVIRONMENT, || {
            [("Valid".into(), "prefix".into()), invalid_pair]
        })
        .unwrap_err();
        assert_eq!(error, SnapshotError::InvalidEncoding);
        assert_eq!(error.code(), "X09-DRIVER-006");
    }
    assert!(
        EnvironmentSnapshot::from_host(CAPABILITY_ENVIRONMENT, || {
            [("\u{fffd}".into(), "\u{fffd}".into())]
        })
        .is_ok()
    );
}

#[test]
/// 各平台都拒绝精确重复，大小写变体遵循当前平台规则。
fn platform_name_equivalence_governs_duplicate_lookup_and_removal() {
    let pairs = [
        ("ÉName".into(), "original".into()),
        ("éNAME".into(), "second".into()),
    ];
    assert_eq!(
        EnvironmentSnapshot::from_pairs(&[pairs[0].clone(), pairs[0].clone()]).unwrap_err(),
        SnapshotError::DuplicateName
    );
    let snapshot = EnvironmentSnapshot::from_pairs(&pairs[..1]).unwrap();
    #[cfg(windows)]
    {
        let error = EnvironmentSnapshot::from_pairs(&pairs).unwrap_err();
        assert_eq!(error, SnapshotError::DuplicateName);
        assert_eq!(error.code(), "X09-DRIVER-006");
        assert_eq!(
            strings(&array(run(&snapshot, Operation::Get, &["éNAME"]).unwrap())),
            ["original"]
        );
        run(&snapshot, Operation::Set, &["éNAME", "updated"]).unwrap();
        assert_eq!(snapshot.view.lock().unwrap().entries.len(), 1);
        assert_eq!(
            strings(&array(run(&snapshot, Operation::Get, &["ÉName"]).unwrap())),
            ["updated"]
        );
        assert_eq!(
            run(&snapshot, Operation::Unset, &["ÉNAME"]).unwrap(),
            RuntimeValue::Bool(true)
        );
        assert!(array(run(&snapshot, Operation::Get, &["éNAME"]).unwrap()).is_empty());
    }
    #[cfg(not(windows))]
    {
        assert!(EnvironmentSnapshot::from_pairs(&pairs).is_ok());
        assert!(array(run(&snapshot, Operation::Get, &["éNAME"]).unwrap()).is_empty());
        run(&snapshot, Operation::Set, &["éNAME", "updated"]).unwrap();
        assert_eq!(snapshot.view.lock().unwrap().entries.len(), 2);
        assert_eq!(
            run(&snapshot, Operation::Unset, &["éNAME"]).unwrap(),
            RuntimeValue::Bool(true)
        );
        assert_eq!(
            strings(&array(run(&snapshot, Operation::Get, &["ÉName"]).unwrap())),
            ["original"]
        );
    }
}

#[test]
/// 多字节字段按 UTF-8 字节数计量，读删和写入使用各自的超限错误。
fn byte_limits_include_exact_name_and_value_boundaries() {
    let snapshot = EnvironmentSnapshot::default();
    for bytes in [MAX_NAME_BYTES - 1, MAX_NAME_BYTES, MAX_NAME_BYTES + 1] {
        let mut name = "é".repeat(bytes / 2);
        if bytes % 2 != 0 {
            name.push('a');
        }
        assert_eq!(name.len(), bytes);
        let admitted = EnvironmentSnapshot::from_pairs(&[(name.clone(), "".into())]);
        if bytes > MAX_NAME_BYTES {
            assert_eq!(admitted.unwrap_err().code(), "X09-DRIVER-007");
            assert_eq!(
                runtime_error(run(&snapshot, Operation::Set, &[&name, ""])).code(),
                "ENV_LIMIT_EXCEEDED"
            );
            for operation in [Operation::Get, Operation::Unset] {
                assert_eq!(
                    runtime_error(run(&snapshot, operation, &[&name])).code(),
                    INVALID_VALUE_CODE
                );
            }
        } else {
            assert!(admitted.is_ok());
            run(&snapshot, Operation::Set, &[&name, ""]).unwrap();
            assert_eq!(
                strings(&array(run(&snapshot, Operation::Get, &[&name]).unwrap())),
                [""]
            );
            assert_eq!(
                run(&snapshot, Operation::Unset, &[&name]).unwrap(),
                RuntimeValue::Bool(true)
            );
        }
    }
    let value = "é".repeat(MAX_VALUE_BYTES / 2);
    let below = format!("{}x", &value[2..]);
    let above = format!("{value}x");
    assert_eq!(below.len(), MAX_VALUE_BYTES - 1);
    assert!(EnvironmentSnapshot::from_pairs(&[("Name".into(), below.clone())]).is_ok());
    assert!(EnvironmentSnapshot::from_pairs(&[("Name".into(), value.clone())]).is_ok());
    run(&snapshot, Operation::Set, &["Name", &below]).unwrap();
    run(&snapshot, Operation::Set, &["Name", &value]).unwrap();
    assert_eq!(
        EnvironmentSnapshot::from_pairs(&[("Name".into(), above.clone())])
            .unwrap_err()
            .code(),
        "X09-DRIVER-007"
    );
    assert_eq!(
        runtime_error(run(&snapshot, Operation::Set, &["Name", &above])).code(),
        "ENV_LIMIT_EXCEEDED"
    );
    assert_eq!(
        strings(&array(run(&snapshot, Operation::Get, &["Name"]).unwrap())),
        [value]
    );
}

#[test]
/// 条目达到上限时仍可替换，删除后可插入新名称。
fn pair_budget_counts_only_committed_names() {
    let pairs: Vec<_> = (0..=MAX_PAIRS)
        .map(|index| (format!("Name{index}"), String::new()))
        .collect();
    assert!(EnvironmentSnapshot::from_pairs(&pairs[..MAX_PAIRS - 1]).is_ok());
    let snapshot = EnvironmentSnapshot::from_pairs(&pairs[..MAX_PAIRS]).unwrap();
    assert_eq!(
        EnvironmentSnapshot::from_pairs(&pairs).unwrap_err().code(),
        "X09-DRIVER-007"
    );
    let name = &pairs[0].0;
    let new_name = &pairs[MAX_PAIRS].0;
    run(&snapshot, Operation::Set, &[name, "replacement"]).unwrap();
    assert_eq!(
        runtime_error(run(&snapshot, Operation::Set, &[new_name, ""])).code(),
        "ENV_LIMIT_EXCEEDED"
    );
    assert_eq!(snapshot.view.lock().unwrap().entries.len(), MAX_PAIRS);
    assert_eq!(
        strings(&array(run(&snapshot, Operation::Get, &[name]).unwrap())),
        ["replacement"]
    );
    run(&snapshot, Operation::Unset, &[name]).unwrap();
    run(&snapshot, Operation::Set, &[new_name, ""]).unwrap();
    assert_eq!(snapshot.view.lock().unwrap().entries.len(), MAX_PAIRS);
}

#[test]
/// 总字节预算先扣旧项，超限失败后保留旧值和配额。
fn total_budget_accounts_for_replacement_and_failure() {
    let mut pairs: Vec<_> = (0..15)
        .map(|index| (format!("V{index:02}"), "x".repeat(MAX_VALUE_BYTES)))
        .collect();
    let previous_bytes: usize = pairs
        .iter()
        .map(|(name, value)| name.len() + value.len())
        .sum();
    let last_name = "Last";
    let exact_len = MAX_TOTAL_BYTES - previous_bytes - last_name.len();
    let mut last_value = "x".repeat(exact_len - 1);
    pairs.push((last_name.into(), last_value.clone()));
    assert!(EnvironmentSnapshot::from_pairs(&pairs).is_ok());
    last_value.push('x');
    pairs.last_mut().unwrap().1 = last_value.clone();
    let snapshot = EnvironmentSnapshot::from_pairs(&pairs).unwrap();
    let replacement = "y".repeat(exact_len);
    run(&snapshot, Operation::Set, &[last_name, &replacement]).unwrap();
    last_value.push('x');
    pairs.last_mut().unwrap().1 = last_value.clone();
    assert_eq!(
        EnvironmentSnapshot::from_pairs(&pairs).unwrap_err().code(),
        "X09-DRIVER-007"
    );
    assert_eq!(
        runtime_error(run(&snapshot, Operation::Set, &[last_name, &last_value])).code(),
        "ENV_LIMIT_EXCEEDED"
    );
    assert_eq!(snapshot.view.lock().unwrap().total_bytes, MAX_TOTAL_BYTES);
    assert_eq!(
        strings(&array(
            run(&snapshot, Operation::Get, &[last_name]).unwrap()
        )),
        [replacement]
    );
    run(&snapshot, Operation::Unset, &[last_name]).unwrap();
    assert_eq!(snapshot.view.lock().unwrap().total_bytes, previous_bytes);
}

#[test]
/// 提交前中断保留旧视图，并原样交回调用方的控制身份。
fn interruptions_preserve_entries_and_budget() {
    let snapshot = EnvironmentSnapshot::from_pairs(&[("Name".into(), "old".into())]).unwrap();
    let name = text("Name");
    let new_name = text("New");
    let value = text("longer");
    for code in [
        "X09-DRIVER-001",
        "X09-DRIVER-002",
        crate::CONCURRENCY_CANCELLED_CODE,
    ] {
        for (operation, arguments) in [
            (Operation::Set, vec![&name, &value]),
            (Operation::Set, vec![&new_name, &value]),
            (Operation::Unset, vec![&name]),
            (Operation::Get, vec![&name]),
        ] {
            let failure =
                snapshot.execute(operation, &arguments, CAPABILITY_ENVIRONMENT, || Err(code));
            assert!(matches!(failure, Err(Failure::Interrupted(actual)) if actual == code));
        }
    }
    let view = snapshot.view.lock().unwrap();
    assert_eq!(view.total_bytes, 7);
    assert_eq!(view.entries, [("Name".into(), "old".into())]);
}

#[test]
/// 同次运行的线程共享成功提交，不同运行使用独立视图。
fn shares_committed_mutations_only_within_one_run() {
    let pairs = [("Name".into(), "initial".into())];
    let local = EnvironmentSnapshot::from_pairs(&pairs).unwrap();
    let other = EnvironmentSnapshot::from_pairs(&pairs).unwrap();
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                assert_eq!(
                    run(&local, Operation::Set, &["Name", "updated"]).unwrap(),
                    RuntimeValue::None
                );
            })
            .join()
            .unwrap();
        scope
            .spawn(|| {
                assert_eq!(
                    strings(&array(run(&local, Operation::Get, &["Name"]).unwrap())),
                    ["updated"]
                );
                assert_eq!(
                    run(&local, Operation::Unset, &["Name"]).unwrap(),
                    RuntimeValue::Bool(true)
                );
            })
            .join()
            .unwrap();
    });
    assert!(array(run(&local, Operation::Get, &["Name"]).unwrap()).is_empty());
    assert_eq!(
        strings(&array(run(&other, Operation::Get, &["Name"]).unwrap())),
        ["initial"]
    );
}

#[test]
/// 输出分配失败不会交付部分数组或改变视图。
fn output_allocation_failure_preserves_the_snapshot() {
    let snapshot = EnvironmentSnapshot::from_pairs(&[("Name".into(), "initial".into())]).unwrap();
    let name = text("Name");
    let failure = crate::allocation_testing::StringAllocationFailure::after(0);
    let result = snapshot.execute(Operation::Get, &[&name], CAPABILITY_ENVIRONMENT, || {
        Ok::<_, Infallible>(())
    });
    assert_eq!(runtime_error(result).code(), ALLOCATION_CODE);
    assert!(failure.was_triggered());
    drop(failure);
    assert_eq!(
        strings(&array(run(&snapshot, Operation::Get, &["Name"]).unwrap())),
        ["initial"]
    );
}
