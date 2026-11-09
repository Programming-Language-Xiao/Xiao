//! 当前快照的显式精度除法全链路回归，包含 B3-3 内部 intrinsic 声明。
//! B2“仅激活 33”的历史证据固定在代码 37b92c53 及其原始归档。
//! 此处 34～48 的声明断言不证明 VM/LLVM 绑定或 std.cmath 可用。
#[path = "20o_b2_a_support/mod.rs"]
#[allow(dead_code)]
mod b2_a;
#[path = "support/b2_divide.rs"]
mod support;
use xiao_driver::{FrontendCompiler, FrontendRequest};

#[test]
/// 完整示例必须从源级进入VM、字节码及真实归档。
fn lfloat_divide_has_vm_golden_outputs() {
    support::vm_suite();
}

#[test]
#[ignore = "需要本任务新Runtime和显式LLVM工具链；维护原生精确入口调用相同套件"]
/// 同一份源码和黄金进入O0/O3；失败不可删除命名调用。
fn lfloat_divide_matches_native_golden_outputs() {
    support::native_suite();
}

#[test]
/// 源级33号契约保持精确；B3内部声明34～48不改变其参数或错误边界。
fn lfloat_divide_requires_the_frozen_public_contract() {
    let valid = "lfloat_divide(lfloat(1),lfloat(2),3,\"half_even\")";
    for expression in [
        "lfloat_divide()",
        "lfloat_divide(lfloat(1),lfloat(2),3)",
        "lfloat_divide(1,lfloat(2),3,\"half_even\")",
        "lfloat_divide(lfloat(1),1,3,\"half_even\")",
        "lfloat_divide(lfloat(1),lfloat(2),sint(3),\"half_even\")",
        "lfloat_divide(lfloat(1),lfloat(2),3,true)",
        "lfloat_divide(lfloat(1),lfloat(2),3,\"half_even\",0)",
        "lfloat_divide(numerator=lfloat(1),denominator=lfloat(2),precision=3,typo=\"half_even\")",
        "lfloat_divide(lfloat(1),numerator=lfloat(2),precision=3,rounding=\"half_even\")",
    ] {
        assert!(
            FrontendCompiler::new()
                .compile(&FrontendRequest::from_text(format!(
                    "print({expression})\n"
                )))
                .is_err(),
            "{expression}"
        );
    }
    assert!(
        FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(format!("print({valid})\n")))
            .is_ok()
    );
    let decl = xiao_intrinsics::by_name("lfloat_divide").unwrap();
    assert_eq!(decl.id.get(), 33);
    assert_eq!(decl.effects, xiao_intrinsics::Effects::Allocates);
    assert_eq!(decl.capability, xiao_intrinsics::Capability::None);
    for id in 34..=48 {
        let internal =
            xiao_intrinsics::active_by_id(xiao_intrinsics::IntrinsicId::new(id).unwrap())
                .expect("B3-3已登记34～48内部声明");
        assert_eq!(internal.kind, xiao_intrinsics::IntrinsicKind::Cmath);
        assert_eq!(
            internal.signature.parameter,
            xiao_intrinsics::ValueType::Float
        );
        assert_eq!(
            internal.signature.return_type,
            xiao_intrinsics::ValueType::Float
        );
        assert_eq!(internal.effects, xiao_intrinsics::Effects::Pure);
        assert_eq!(internal.capability, xiao_intrinsics::Capability::None);
    }
    assert!(
        xiao_intrinsics::active_by_id(xiao_intrinsics::IntrinsicId::new(49).unwrap()).is_none()
    );
    assert!(
        xiao_intrinsics::active_by_id(xiao_intrinsics::IntrinsicId::new(100).unwrap()).is_none()
    );
    assert_eq!(xiao_runtime_abi::ABI_MINOR_VERSION, 18);
}
