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
        if row.status == "active" {
            assert!(
                names.insert(&row.name),
                "重复的 intrinsic 名称 {}",
                row.name
            );
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
            assert_eq!(
                row.status, "tombstone",
                "未知 intrinsic 状态 {}",
                row.status
            );
            assert!(
                row.name.starts_with("__reserved_"),
                "墓碑名称必须明确保留: {}",
                row.name
            );
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
