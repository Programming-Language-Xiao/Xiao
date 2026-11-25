//! 构建期读取并校验 intrinsic 声明，生成供消费者使用的静态契约表。

#![allow(missing_docs)]

use std::{collections::BTreeSet, env, fs, path::PathBuf};

use serde::Deserialize;

#[derive(Deserialize)]
struct FileModel {
    schema_version: u32,
    intrinsics: Vec<Row>,
}
#[derive(Deserialize)]
struct Row {
    id: u32,
    name: String,
    kind: String,
    arity: String,
    parameter_type: String,
    return_type: String,
    vm_binding: String,
    abi_binding: String,
    effect: String,
    capability: String,
    error: String,
    status: String,
}

fn main() {
    println!("cargo:rerun-if-changed=intrinsics.json");
    let source = fs::read_to_string("intrinsics.json").expect("读取 intrinsic 声明失败");
    let model: FileModel = serde_json::from_str(&source).expect("intrinsic 声明不是合法 JSON");
    assert_eq!(model.schema_version, 1, "不支持的 intrinsic 声明版本");
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for row in &model.intrinsics {
        assert_ne!(row.id, 0, "IntrinsicId 0 保留为无效哨兵");
        assert!(ids.insert(row.id), "重复的 IntrinsicId {}", row.id);
        assert!(
            names.insert(&row.name),
            "重复的 intrinsic 名称 {}",
            row.name
        );
        if row.status == "active" {
            assert_ne!(
                row.vm_binding, "none",
                "活动 intrinsic 缺少 VM 绑定: {}",
                row.name
            );
            if row.kind == "print" || row.kind == "input" {
                assert_ne!(
                    row.abi_binding, "none",
                    "IO intrinsic 缺少 ABI 绑定: {}",
                    row.name
                );
            }
            match row.kind.as_str() {
                "scalar_conversion" => {
                    assert_eq!(row.arity, "one", "标量转换必须是一元: {}", row.name);
                    assert_eq!(
                        row.parameter_type, "dynamic",
                        "标量转换参数必须是 dynamic: {}",
                        row.name
                    );
                    assert_eq!(
                        row.vm_binding, "scalar_cast",
                        "标量转换缺少 scalar_cast 绑定: {}",
                        row.name
                    );
                    assert_eq!(
                        row.abi_binding, "none",
                        "标量转换不应声明 ABI 绑定: {}",
                        row.name
                    );
                    assert_eq!(row.effect, "pure", "标量转换必须是 pure: {}", row.name);
                    assert_eq!(row.capability, "none", "标量转换不应需要能力: {}", row.name);
                }
                "char_constructor" => {
                    assert_eq!(row.arity, "one", "字符构造必须是一元: {}", row.name);
                    assert_eq!(
                        row.parameter_type, "dynamic",
                        "字符构造参数必须是 dynamic: {}",
                        row.name
                    );
                    assert_eq!(row.return_type, "str", "字符构造必须返回 str: {}", row.name);
                    assert_eq!(
                        row.vm_binding, "char_constructor",
                        "字符构造缺少绑定: {}",
                        row.name
                    );
                    assert_eq!(
                        row.abi_binding, "none",
                        "字符构造不应声明 ABI 绑定: {}",
                        row.name
                    );
                    assert_eq!(row.effect, "pure", "字符构造必须是 pure: {}", row.name);
                    assert_eq!(row.capability, "none", "字符构造不应需要能力: {}", row.name);
                }
                "time" => {
                    let time = [
                        ("utc_now", "one", "str", "int", "reads_clock", "clock"),
                        ("monotonic", "one", "str", "int", "reads_clock", "clock"),
                        (
                            "format_utc",
                            "two",
                            "time_value_and_unit",
                            "str",
                            "allocates",
                            "none",
                        ),
                        (
                            "sleep",
                            "two",
                            "time_value_and_unit",
                            "none",
                            "sleeps",
                            "clock",
                        ),
                    ];
                    let index = row.id.checked_sub(49).expect("time ID 从 49 开始") as usize;
                    let &(name, arity, parameter, result, effect, capability) =
                        time.get(index).expect("time ID 必须为 49～52");
                    assert_eq!(row.name, format!("os_time_{name}"));
                    assert_eq!(row.vm_binding, format!("time_{name}"));
                    assert_eq!(row.abi_binding, row.vm_binding);
                    assert_eq!(row.arity, arity);
                    assert_eq!(row.parameter_type, parameter);
                    assert_eq!(row.return_type, result);
                    assert_eq!(row.effect, effect);
                    assert_eq!(row.capability, capability);
                }
                "args" => {
                    let (binding, result) = match row.id {
                        53 => ("args_program_name", "str"),
                        54 => ("args_argv", "str_array"),
                        _ => panic!("args ID 必须为 53 或 54"),
                    };
                    assert_eq!(row.name, format!("os_{binding}"));
                    assert_eq!(row.vm_binding, binding);
                    assert_eq!(row.abi_binding, binding);
                    assert_eq!(row.arity, "zero");
                    assert_eq!(row.parameter_type, "none");
                    assert_eq!(row.return_type, result);
                    assert_eq!(row.effect, "reads_arguments");
                    assert_eq!(row.capability, "process");
                    assert_eq!(row.error, "runtime.type_mismatch");
                }
                "lfloat_divide" => {
                    assert_eq!(row.id, 33);
                    assert_eq!(row.arity, "four");
                    assert_eq!(row.parameter_type, "lfloat_divide_parameters");
                    assert_eq!(row.return_type, "lfloat");
                    assert_eq!(row.vm_binding, "lfloat_divide");
                    assert_eq!(row.abi_binding, "lfloat_divide");
                    assert_eq!(row.effect, "allocates");
                    assert_eq!(row.capability, "none");
                }
                "cmath" => {
                    // F-5编号顺序与冻结函数集合，生成期拒绝错号或绑定漂移。
                    const NAMES: [&str; 15] = [
                        "sqrt", "floor", "ceil", "round", "trunc", "abs", "min", "max", "pow",
                        "sin", "cos", "tan", "exp", "ln", "log10",
                    ];
                    let index = row.id.checked_sub(34).expect("cmath编号必须为34～48") as usize;
                    let name = NAMES.get(index).expect("cmath编号必须为34～48");
                    assert_eq!(row.name, format!("cmath_{name}"));
                    assert_eq!(
                        row.arity,
                        if matches!(*name, "min" | "max" | "pow") {
                            "two"
                        } else {
                            "one"
                        }
                    );
                    assert_eq!(row.parameter_type, "float");
                    assert_eq!(row.return_type, "float");
                    assert_eq!(row.vm_binding, row.name);
                    assert_eq!(row.abi_binding, row.name);
                    assert_eq!(row.effect, "pure");
                    assert_eq!(row.capability, "none");
                    assert_eq!(row.error, "runtime.type_mismatch");
                }
                "error_code" => {
                    assert_eq!(row.arity, "one");
                    assert_eq!(row.parameter_type, "error");
                    assert_eq!(row.return_type, "str");
                    assert_eq!(row.vm_binding, "error_code");
                    assert_eq!(row.abi_binding, "error_code");
                    assert_eq!(row.effect, "allocates");
                    assert_eq!(row.capability, "none");
                }
                "set_constructor" => {
                    assert_eq!(row.arity, "zero", "set 构造必须是零元: {}", row.name);
                    assert_eq!(row.return_type, "set", "set 构造必须返回 set: {}", row.name);
                    assert_eq!(
                        row.vm_binding, "set_new",
                        "set 构造缺少 set_new 绑定: {}",
                        row.name
                    );
                    assert_eq!(
                        row.abi_binding, "none",
                        "set 构造不应声明 ABI 绑定: {}",
                        row.name
                    );
                    assert_eq!(
                        row.effect, "allocates",
                        "set 构造必须声明分配效果: {}",
                        row.name
                    );
                }
                "set_add" | "set_remove" => {
                    assert_eq!(row.arity, "two", "集合变更必须是二元: {}", row.name);
                    assert_eq!(
                        row.parameter_type, "set_element",
                        "集合变更参数必须是 set_element: {}",
                        row.name
                    );
                    assert_eq!(row.vm_binding, row.kind, "集合变更 VM 绑定必须对应类别");
                    assert_eq!(row.abi_binding, row.kind, "集合变更 ABI 绑定必须对应类别");
                    assert_eq!(row.effect, "mutates_container", "集合变更必须声明可变效果");
                    assert_eq!(row.capability, "none", "集合变更不应申请能力");
                    if row.kind == "set_add" {
                        assert_eq!(row.return_type, "none");
                    } else {
                        assert_eq!(row.return_type, "bool");
                    }
                }
                "array_push" => {
                    assert_eq!(row.arity, "two", "数组追加必须是二元: {}", row.name);
                    assert_eq!(row.parameter_type, "array_element");
                    assert_eq!(row.return_type, "none");
                    assert_eq!(row.vm_binding, "array_push");
                    assert_eq!(row.abi_binding, "array_push");
                    assert_eq!(row.effect, "mutates_container");
                    assert_eq!(row.capability, "none");
                }
                "print" => {
                    assert_eq!(row.arity, "variadic", "print 必须是可变参数");
                    assert_eq!(row.parameter_type, "dynamic", "print 参数必须是 dynamic");
                    assert_eq!(row.return_type, "none", "print 必须返回 none");
                    assert_eq!(row.vm_binding, "print", "print 缺少 VM 绑定");
                    assert_eq!(row.abi_binding, "print", "print 缺少 ABI 绑定");
                    assert_eq!(row.effect, "writes_stdout", "print 必须声明 stdout 效果");
                    assert_eq!(row.capability, "stdout", "print 必须声明 stdout 能力");
                }
                "input" => {
                    assert_eq!(row.arity, "optional_one", "input 只接受可选提示参数");
                    assert_eq!(row.parameter_type, "str", "input 提示参数必须是 str");
                    assert_eq!(row.return_type, "str", "input 必须返回 str");
                    assert_eq!(row.vm_binding, "input", "input 缺少 VM 绑定");
                    assert_eq!(row.abi_binding, "input", "input 缺少 ABI 绑定");
                    assert_eq!(row.effect, "reads_stdin", "input 必须声明 stdin 效果");
                    assert_eq!(row.capability, "stdin", "input 必须声明 stdin 能力");
                }
                "resource_open_file"
                | "resource_open_directory"
                | "resource_close"
                | "resource_read_text" => {
                    assert_eq!(row.arity, "one", "资源 intrinsic 必须是一元: {}", row.name);
                    assert_eq!(
                        row.vm_binding, row.kind,
                        "资源 intrinsic VM 绑定必须与类别对应: {}",
                        row.name
                    );
                    assert_eq!(
                        row.abi_binding, row.kind,
                        "资源 intrinsic ABI 绑定必须与类别对应: {}",
                        row.name
                    );
                    assert_eq!(
                        row.capability, "filesystem",
                        "资源 intrinsic 必须需要 filesystem 能力: {}",
                        row.name
                    );
                    assert!(
                        row.effect == "reads_filesystem" || row.effect == "writes_filesystem",
                        "资源 intrinsic 必须声明文件系统效果: {}",
                        row.name
                    );
                    match row.kind.as_str() {
                        "resource_open_file" => {
                            assert_eq!(row.parameter_type, "str");
                            assert_eq!(row.return_type, "file");
                            assert_eq!(row.effect, "reads_filesystem");
                        }
                        "resource_open_directory" => {
                            assert_eq!(row.parameter_type, "str");
                            assert_eq!(row.return_type, "directory");
                            assert_eq!(row.effect, "reads_filesystem");
                        }
                        "resource_close" => {
                            assert_eq!(row.parameter_type, "resource");
                            assert_eq!(row.return_type, "none");
                            assert_eq!(row.effect, "writes_filesystem");
                        }
                        "resource_read_text" => {
                            assert_eq!(row.parameter_type, "file");
                            assert_eq!(row.return_type, "str");
                            assert_eq!(row.effect, "reads_filesystem");
                        }
                        _ => unreachable!(),
                    }
                }
                "process_spawn" | "process_wait" | "process_close" => {
                    assert_eq!(row.arity, "one", "进程入口必须是一元");
                    assert_eq!(row.capability, "process", "进程入口必须需要 process 能力");
                    assert_eq!(row.vm_binding, row.kind, "进程入口 VM 绑定必须对应类别");
                    assert_eq!(row.abi_binding, row.kind, "进程入口 ABI 绑定必须对应类别");
                    let (parameter, result, effect) = match row.kind.as_str() {
                        "process_spawn" => ("str", "process", "spawns_process"),
                        "process_wait" => ("process", "int", "waits_process"),
                        "process_close" => ("process", "none", "releases_process"),
                        _ => unreachable!(),
                    };
                    assert_eq!(row.parameter_type, parameter);
                    assert_eq!(row.return_type, result);
                    assert_eq!(row.effect, effect);
                }
                "error_constructor" => {
                    assert_eq!(row.arity, "error", "错误构造签名不完整: {}", row.name);
                    assert_eq!(
                        row.return_type, "dynamic",
                        "错误构造必须返回 dynamic: {}",
                        row.name
                    );
                    assert_eq!(
                        row.vm_binding, "make_error",
                        "错误构造缺少 make_error 绑定: {}",
                        row.name
                    );
                    assert_eq!(
                        row.abi_binding, "error_new",
                        "错误构造缺少 error_new 绑定: {}",
                        row.name
                    );
                    assert_eq!(
                        row.effect, "allocates",
                        "错误构造必须声明分配效果: {}",
                        row.name
                    );
                }
                other => panic!("未知活动 intrinsic 类别 {other}"),
            }
        } else {
            match row.status.as_str() {
                "reserved" => {
                    assert!((55..=63).contains(&row.id), "预登记的 ID 必须为 55～63");
                    assert!(row.name.starts_with("os_"));
                    assert_eq!(row.kind, "reserved");
                    assert!(
                        [
                            &row.arity,
                            &row.parameter_type,
                            &row.return_type,
                            &row.vm_binding,
                            &row.abi_binding,
                            &row.effect,
                            &row.capability,
                            &row.error
                        ]
                        .into_iter()
                        .all(|field| field == "none"),
                        "预登记 intrinsic 不得带活动签名或后端绑定: {}",
                        row.name
                    );
                }
                "tombstone" => assert!(
                    row.name.starts_with("__reserved_"),
                    "墓碑名称必须明确保留: {}",
                    row.name
                ),
                _ => panic!("未知 intrinsic 状态 {}", row.status),
            }
        }
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR 缺失"));
    let mut generated = String::new();
    generated.push_str("pub static DECLARATIONS: &[IntrinsicDecl] = &[\n");
    for row in &model.intrinsics {
        generated.push_str(&format!("    IntrinsicDecl {{ id: IntrinsicId::new_unchecked({}), public_name: {:?}, kind: IntrinsicKind::{}, signature: Signature {{ arity: Arity::{}, parameter: ValueType::{}, return_type: ValueType::{} }}, effects: Effects::{}, capability: Capability::{}, error_code: {:?}, vm_binding: VmBinding::{}, abi_binding: AbiBinding::{}, status: DeclarationStatus::{}, }},\n", row.id, row.name, pascal(&row.kind), pascal(&row.arity), pascal(&row.parameter_type), pascal(&row.return_type), pascal(&row.effect), pascal(&row.capability), row.error, pascal(&row.vm_binding), pascal(&row.abi_binding), pascal(&row.status)));
    }
    generated.push_str("] ;\n");
    fs::write(out.join("intrinsics_generated.rs"), generated).expect("写入 intrinsic 生成产物失败");
}

fn pascal(value: &str) -> String {
    value
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}
