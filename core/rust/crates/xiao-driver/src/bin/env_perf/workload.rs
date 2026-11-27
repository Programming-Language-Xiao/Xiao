//! 固定合成输入和双后端共用的 Xiao 负载。

use std::fmt::Write;

/// 每个操作负载固定执行的调用次数。
pub const FIXED_CALLS: usize = 64;
/// 普通负载覆盖的名值对数量。
pub const SIZES: [usize; 3] = [256, 512, 1_024];
/// 快照、读、写、删、满额替换和超限失败六类负载。
pub const OPERATIONS: [&str; 6] = [
    "snapshot",
    "get",
    "set",
    "unset",
    "full-replace",
    "overflow",
];

/// 生成固定序号的环境名称。
/// 生成固定序号的环境名称。
fn key(index: usize) -> String {
    format!("XIAO_B3_PERF_{index:04}")
}

/// 返回普通或满额负载中每条值使用的字节数。
pub fn value_bytes(full: bool) -> usize {
    if full { 1_024 - key(0).len() } else { 32 }
}

/// 生成固定顺序和固定内容的合成名值对。
pub fn input(pairs: usize, full: bool) -> Vec<(String, String)> {
    let value = "o".repeat(value_bytes(full));
    (0..pairs)
        .map(|index| (key(index), value.clone()))
        .collect()
}

/// 为指定负载生成同时适用于 VM 与原生后端的 Xiao 源码。
pub fn source(case: &str) -> String {
    let full = matches!(case, "full-replace" | "overflow");
    let old = "o".repeat(value_bytes(full));
    let new = "r".repeat(value_bytes(full));
    let mut source = String::from(concat!(
        "from std.env import get, set, unset\nfrom std.time import monotonic\n",
        "def env_matches(str name, str expected) -> bool\n",
        "    count = 0\n    valid = true\n    for value in get(name)\n",
        "        count = count + 1\n        if value != expected\n            valid = false\n",
        "    return count == 1 and valid\n",
        "def env_missing(str name) -> bool\n",
        "    missing = true\n    for value in get(name)\n        missing = false\n    return missing\n",
    ));
    writeln!(source, "old = {old:?}\nreplacement = {new:?}").unwrap();
    if !full {
        let names: Vec<_> = (0..FIXED_CALLS).map(|i| key(4 * i)).collect();
        writeln!(source, "names = {names:?}").unwrap();
    } else {
        let cycles = vec![0; FIXED_CALLS / 2];
        writeln!(
            source,
            "name = {:?}\nsentinel = {:?}\nextra = {:?}\ncycles = {cycles:?}",
            key(0),
            key(1_023),
            key(1_024),
        )
        .unwrap();
        if case == "overflow" {
            writeln!(source, "too_large = {:?}", old.clone() + "x").unwrap();
        }
    }
    source.push_str("valid = true\nstarted = monotonic(\"ns\")\n");
    source.push_str(match case {
        // 快照负载在宿主计时，复用读取程序作为计时后的校验。
        "get" => "for name in names\n    if not env_matches(name, old)\n        valid = false\n",
        "set" => "for name in names\n    set(name, replacement)\n",
        "unset" => "for name in names\n    if not unset(name)\n        valid = false\n",
        "full-replace" => "for cycle in cycles\n    set(name, replacement)\n    set(name, old)\n",
        "overflow" => concat!(
            "for cycle in cycles\n",
            "    try\n        set(extra, old)\n        valid = false\n",
            "    catch failure as ResourceError\n        if error_code(failure) != \"ENV_LIMIT_EXCEEDED\"\n            valid = false\n",
            "    try\n        set(name, too_large)\n        valid = false\n",
            "    catch failure as ResourceError\n        if error_code(failure) != \"ENV_LIMIT_EXCEEDED\"\n            valid = false\n",
        ),
        _ => unreachable!("{case}"),
    });
    source.push_str("elapsed = monotonic(\"ns\") - started\n");
    source.push_str(match case {
        "get" => "",
        "set" => "for name in names\n    if not env_matches(name, replacement)\n        valid = false\n",
        "unset" => "for name in names\n    if not env_missing(name)\n        valid = false\n",
        "full-replace" => concat!(
            "if not env_matches(name, old) or not env_matches(sentinel, old) or not env_missing(extra)\n    valid = false\n",
            "set(name, replacement)\n",
            "if not env_matches(name, replacement)\n    valid = false\n",
        ),
        "overflow" => concat!(
            "if not env_matches(name, old) or not env_matches(sentinel, old) or not env_missing(extra)\n    valid = false\n",
            "set(name, replacement)\n",
            "if not unset(sentinel)\n    valid = false\n",
            "set(extra, old)\n",
            "if not env_matches(name, replacement) or not env_missing(sentinel) or not env_matches(extra, old)\n    valid = false\n",
        ),
        _ => unreachable!("{case}"),
    });
    source.push_str("if valid\n    print(elapsed)\nelse\n    print(-1)\n");
    source
}
