//! 校验释放轨迹的对象生命周期，再比较 VM 与原生的终态动作。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use xiao_runtime::ReleaseAction::{self, *};

/// 既有锁等待取消黄金夹具。
pub(super) const LOCK_WAIT: &str = "native-lock-wait-027";
/// 在同一夹具的 ABI 调用前固定取消时点的独立回归。
pub(super) const LOCK_WAIT_PRECHECK: &str = "native-lock-wait-027-precheck";

/// 单个对象的生命周期签名，保留全部弱释放次数。
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Lifecycle {
    Heap,
    Shared { weak_releases: usize },
    Guard,
}

/// 对象生命周期多重集合和带签名的文件顺序终态流。
#[derive(Debug)]
struct Projection {
    objects: Vec<Lifecycle>,
    terminal: Vec<(Lifecycle, ReleaseAction)>,
}

impl Projection {
    /// 逐行检查 TSV，只归并同一对象连续归还的强引用别名。
    fn parse(text: &str) -> Result<Self, String> {
        let mut by_object = BTreeMap::<u64, Vec<ReleaseAction>>::new();
        let mut terminal = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let mut fields = line.split('\t');
            let sequence = fields.next().and_then(|field| field.parse::<usize>().ok());
            let object_id = fields.next().and_then(|field| field.parse::<u64>().ok());
            let action = fields.next();
            if sequence != Some(index) || fields.next().is_some() {
                return Err(format!("释放追踪序号或字段数无效：{line:?}"));
            }
            let (Some(object_id), Some(action)) = (object_id, action) else {
                return Err(format!("释放追踪对象身份或动作无效：{line:?}"));
            };
            let action = match action {
                "concurrency_release" => continue,
                "strong_release" => StrongRelease,
                "destroy" => Destroy,
                "shared_release" => SharedRelease,
                "shared_destroy" => SharedDestroy,
                "weak_release" => WeakRelease,
                "lock_guard_release" => LockGuardRelease,
                _ => return Err(format!("未知释放动作：{line:?}")),
            };
            let actions = by_object.entry(object_id).or_default();
            // 临时强引用只在同一对象的子序列内压缩。终态动作逐条保留。
            if matches!(action, StrongRelease | SharedRelease) {
                if actions.last() != Some(&action) {
                    actions.push(action);
                }
            } else {
                actions.push(action);
                terminal.push((object_id, action));
            }
        }

        let lifecycles = by_object
            .into_iter()
            .map(|(id, actions)| {
                let lifecycle = match actions.as_slice() {
                    [StrongRelease, Destroy] => Lifecycle::Heap,
                    [SharedRelease, SharedDestroy, weak @ ..]
                        if !weak.is_empty() && weak.iter().all(|action| *action == WeakRelease) =>
                    {
                        Lifecycle::Shared {
                            weak_releases: weak.len(),
                        }
                    }
                    [LockGuardRelease] => Lifecycle::Guard,
                    _ => return Err(format!("对象 {id} 的释放生命周期无效：{actions:?}")),
                };
                Ok((id, lifecycle))
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        let mut objects = lifecycles.values().copied().collect::<Vec<_>>();
        objects.sort_unstable();
        let terminal = terminal
            .into_iter()
            .map(|(id, action)| (lifecycles[&id], action))
            .collect();
        Ok(Self { objects, terminal })
    }

    /// 每个合法 guard 签名代表一个不同且恰好释放一次的对象。
    fn guard_count(&self) -> usize {
        self.objects
            .iter()
            .filter(|&&lifecycle| lifecycle == Lifecycle::Guard)
            .count()
    }
}

/// 对两侧独立核对场景约束，再比较该场景要求稳定的终态。
fn compare(label: &str, native: &Projection, vm: &Projection) -> Result<(), String> {
    let shared_count = match label {
        "native-queued-cancellation"
        | "native-waiting-cancellation"
        | LOCK_WAIT
        | LOCK_WAIT_PRECHECK => Some(2),
        "native-wait-027" => Some(1),
        _ => None,
    };
    for (backend, trace) in [("native", native), ("VM", vm)] {
        match (shared_count, label) {
            (Some(expected), _) => {
                let shared = trace
                    .objects
                    .iter()
                    .filter(|lifecycle| matches!(lifecycle, Lifecycle::Shared { .. }))
                    .count();
                if shared != expected {
                    return Err(format!(
                        "{backend} 握手共享对象终态组数无效：实际 {shared}，预期 {expected}"
                    ));
                }
                if trace.guard_count() == 0 {
                    return Err(format!("{backend} 必须释放握手守卫"));
                }
            }
            (None, "native-running-cancellation-finally") if trace.objects.is_empty() => {
                return Err(format!("{backend} 应记录握手对象释放"));
            }
            (None, "lock-guard-scope-release") if trace.guard_count() != 2 => {
                return Err(format!("{backend} 两个函数各释放一个不同的 guard"));
            }
            (None, "detach-success") if trace.objects != [Lifecycle::Heap, Lifecycle::Heap] => {
                return Err(format!("{backend} 必须释放两个独立打印对象"));
            }
            _ => {}
        }
    }

    /// 从多重集合保留受取消场景约束的握手共享对象。
    fn shared_objects(trace: &Projection) -> Vec<Lifecycle> {
        trace
            .objects
            .iter()
            .copied()
            .filter(|lifecycle| matches!(lifecycle, Lifecycle::Shared { .. }))
            .collect()
    }
    /// 保留共享对象的销毁与全部弱释放在文件中的先后顺序。
    fn shared_terminal(trace: &Projection) -> Vec<(Lifecycle, ReleaseAction)> {
        trace
            .terminal
            .iter()
            .copied()
            .filter(|(lifecycle, _)| matches!(lifecycle, Lifecycle::Shared { .. }))
            .collect()
    }

    // 保留取消夹具的握手口径，轮询临时对象随调度变化。
    if shared_count.is_some() {
        if shared_objects(native) != shared_objects(vm)
            || shared_terminal(native) != shared_terminal(vm)
        {
            return Err(format!(
                "稳定共享对象的生命周期或终态动作不同：native={:?} VM={:?}",
                shared_terminal(native),
                shared_terminal(vm)
            ));
        }
    } else if label != "native-running-cancellation-finally" {
        if native.objects != vm.objects {
            return Err(format!(
                "释放对象生命周期多重集合不同：native={:?} VM={:?}",
                native.objects, vm.objects
            ));
        }
        if native.terminal != vm.terminal {
            return Err(format!(
                "终态动作流不同：native={:?} VM={:?}",
                native.terminal, vm.terminal
            ));
        }
    }
    Ok(())
}

/// 加载两侧原始轨迹并在比较前记录可对账的对象和终态数量。
pub(super) fn assert_case(label: &str, native: &Path, vm: &Path) {
    let read = |path: &Path| {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("{label}: 读取 {} 失败：{error}", path.display()));
        Projection::parse(&text)
            .unwrap_or_else(|error| panic!("{label}: {}：{error}", path.display()))
    };
    let native = read(native);
    let vm = read(vm);
    println!(
        "22D-TRACE {label} projection=2 vm_objects={} vm_terminal={} native_objects={} native_terminal={}",
        vm.objects.len(),
        vm.terminal.len(),
        native.objects.len(),
        native.terminal.len(),
    );
    compare(label, &native, &vm).unwrap_or_else(|error| panic!("{label}: {error}"));
}

#[test]
/// 用正反例检查别名归并、对象生命周期和终态顺序。
fn release_projection_v2_regressions() {
    let root = std::env::temp_dir().join(format!("xiao-22d-projection-{}", std::process::id()));
    fs::create_dir(&root).expect("创建投影反例临时目录");
    let project = |label: &str, events: &[(u64, ReleaseAction)]| {
        let tsv = events
            .iter()
            .enumerate()
            .map(|(index, (id, action))| format!("{index}\t{id}\t{}\n", action.as_str()))
            .collect::<String>();
        let path = root.join(format!("{label}.tsv"));
        fs::write(&path, tsv).expect("写入投影测试轨迹");
        Projection::parse(&fs::read_to_string(path).expect("读取投影测试轨迹"))
    };

    let heap = [
        (1, StrongRelease),
        (1, Destroy),
        (2, StrongRelease),
        (2, Destroy),
    ];
    let shared = [(3, SharedRelease), (3, SharedDestroy), (3, WeakRelease)];
    let guard = [(4, LockGuardRelease)];
    let baseline = [heap.as_slice(), &shared, &guard].concat();
    let mut alias = baseline.clone();
    alias.insert(1, (2, StrongRelease));
    alias.insert(2, (1, StrongRelease));

    let mut missing_destroy = baseline.clone();
    missing_destroy.remove(1);
    let mut duplicate = baseline.clone();
    duplicate.insert(2, (1, Destroy));
    let mut missing_shared_destroy = baseline.clone();
    missing_shared_destroy[5].1 = SharedRelease;
    let swap_heaps = [&heap[2..], &heap[..2], &shared, &guard].concat();
    let mut wrapper = baseline.clone();
    wrapper.insert(1, (9, ConcurrencyRelease));
    wrapper.insert(2, (1, ConcurrencyRelease));
    let swap_shared = [shared.as_slice(), &heap, &guard].concat();

    let baseline_projection = project("baseline", &baseline).unwrap();
    for (id, events, expected_error) in [
        ("P-1", alias.as_slice(), None),
        ("P-2", missing_destroy.as_slice(), Some("释放生命周期无效")),
        ("P-3", duplicate.as_slice(), Some("释放生命周期无效")),
        (
            "P-4",
            &[&baseline[..6], &baseline[7..]].concat(),
            Some("释放生命周期无效"),
        ),
        ("P-5", &baseline[..7], Some("生命周期多重集合不同")),
        (
            "P-6",
            missing_shared_destroy.as_slice(),
            Some("释放生命周期无效"),
        ),
        ("P-7", swap_heaps.as_slice(), None),
        ("P-8", wrapper.as_slice(), None),
        ("P-9", swap_shared.as_slice(), Some("终态动作流不同")),
    ] {
        let result =
            project(id, events).and_then(|actual| compare(id, &actual, &baseline_projection));
        match expected_error {
            None => assert_eq!(result, Ok(()), "{id}"),
            Some(reason) => {
                let error = result.expect_err(id);
                assert!(error.contains(reason), "{id}: {error}");
            }
        }
        println!("22D-PROJECTION {id}: ok");
    }

    // 两侧同时漏掉同一个对象时也要对照夹具的实际数量。
    let one_shared = project(
        "missing-shared-object",
        &[shared.as_slice(), &guard].concat(),
    )
    .unwrap();
    for label in [
        LOCK_WAIT,
        LOCK_WAIT_PRECHECK,
        "native-queued-cancellation",
        "native-waiting-cancellation",
    ] {
        assert!(
            compare(label, &one_shared, &one_shared)
                .unwrap_err()
                .contains("握手共享对象终态组数无效")
        );
    }
    let no_shared = project("missing-only-shared-object", &guard).unwrap();
    assert!(
        compare("native-wait-027", &no_shared, &no_shared)
            .unwrap_err()
            .contains("握手共享对象终态组数无效")
    );
    let one_print = project("missing-print-object", &heap[..2]).unwrap();
    assert!(
        compare("detach-success", &one_print, &one_print)
            .unwrap_err()
            .contains("必须释放两个独立打印对象")
    );
    assert!(Projection::parse("1\t1\tlock_guard_release\n").is_err());
    assert!(Projection::parse("0\t1\tlock_guard_release\textra\n").is_err());

    if let Some(root) = std::env::var_os("XIAO_22D_REPLAY_DIR") {
        let root = std::path::PathBuf::from(root);
        for label in [
            "xcon-bounded-string-success",
            "spawn-join-str-multiargument",
        ] {
            assert_case(
                label,
                &root.join(format!("{label}.native-release-trace.tsv")),
                &root.join(format!("{label}.vm-release-trace.tsv")),
            );
            println!("22D-REPLAY {label}: ok");
        }
    }
    fs::remove_dir_all(root).expect("清理投影反例临时目录");
}
