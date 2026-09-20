//! 09R2G 迭代基础指令的真实执行夹具。
//!
//! 共享向量覆盖完整 `for` 降低；本文件再手工构造最小 TAC，直接命中
//! `Len`/`IndexGetDynamic` 的语义分支，避免降低器与指令执行同时出错时
//! 测试仍然“自洽通过”。

use std::collections::BTreeMap;

use xiao_bytecode::research::{
    BlockId, CallSigTable, CategoryMap, ConstPool, RegisterClass, TacAbi, TacBlock, TacConstant,
    TacFunction, TacInstr, TacOp, TacProgram, VReg,
};
use xiao_ir::IrSpan;
use xiao_runtime::RuntimeValue;
use xiao_vm::research::{
    Carrier, HybridCarrier, RegisterCarrier, RunResult, StackCarrier, VmOptions, run_with,
};

/// 手工 TAC 夹具共用的源码区间。
const SPAN: IrSpan = IrSpan::new(0, 1);

/// 建立只有一个入口块的最小 TAC 程序。
fn program(
    categories: CategoryMap,
    constants: ConstPool,
    instructions: Vec<TacInstr>,
) -> TacProgram {
    TacProgram {
        version: 1,
        abi: TacAbi {
            bytecode_abi_version: 1,
            runtime_abi_version: 1,
            ir_version: 1,
            language_version: "0.1.0".to_owned(),
            target: "r2g-iteration-test".to_owned(),
        },
        constants,
        signatures: CallSigTable::new(),
        functions: vec![TacFunction {
            name: String::new(),
            signature: None,
            entry: BlockId::new(0),
            blocks: vec![TacBlock {
                id: BlockId::new(0),
                scope: 0,
                instructions,
            }],
            parameters: Vec::new(),
            locals: Vec::new(),
            categories: categories.clone(),
            scopes: vec![0],
            handlers: Vec::new(),
            value_registers: BTreeMap::new(),
            span: SPAN,
        }],
        categories,
        plans: Vec::new(),
        selection_plans: Vec::new(),
        broadcast_assignment_plans: Vec::new(),
        random_seed_plans: Vec::new(),
        unsupported: Vec::new(),
    }
}

/// 在指定载体上执行并返回结果。
fn run<C: Carrier>(program: &TacProgram) -> RuntimeValue {
    let outcome = run_with::<C>(program, VmOptions::new());
    assert!(
        matches!(outcome.result, RunResult::Success),
        "执行失败: {:?}",
        outcome.result
    );
    outcome.value.expect("夹具应返回值")
}

/// 构造一个返回 `Len(source)` 的程序。
fn len_program(kind: &str) -> TacProgram {
    let mut constants = ConstPool::new();
    let mut categories = CategoryMap::new();
    let one = VReg::new(0);
    let two = VReg::new(1);
    let source = VReg::new(2);
    let result = VReg::new(3);
    for register in [one, two] {
        categories.insert(register, RegisterClass::Int);
    }
    categories.insert(source, RegisterClass::ObjHandle);
    categories.insert(result, RegisterClass::Int);
    let one_id = constants.intern(TacConstant::Int(1));
    let two_id = constants.intern(TacConstant::Int(2));
    let source_op = match kind {
        "array" => TacOp::NewArray {
            elements: vec![one, two],
        },
        "tuple" => TacOp::NewTuple {
            elements: vec![one, two],
        },
        "set" => TacOp::NewSet {
            elements: vec![one, two],
        },
        "dict_table" => TacOp::NewDictTable {
            entries: vec![("first".to_owned(), one), ("second".to_owned(), two)],
        },
        "dict_column" => TacOp::NewDictColumn {
            entries: vec![("first".to_owned(), one), ("second".to_owned(), two)],
        },
        "str" => {
            categories.insert(source, RegisterClass::ObjHandle);
            let text = constants.intern(TacConstant::Str("小雪A".to_owned()));
            TacOp::LoadConst(text)
        }
        other => panic!("未知可迭代载体 {other}"),
    };
    program(
        categories,
        constants,
        vec![
            TacInstr::with_dst(TacOp::LoadConst(one_id), one, SPAN),
            TacInstr::with_dst(TacOp::LoadConst(two_id), two, SPAN),
            TacInstr::with_dst(source_op, source, SPAN),
            TacInstr::with_dst(TacOp::Len { source }, result, SPAN),
            TacInstr::new(
                TacOp::Return {
                    value: Some(result),
                },
                SPAN,
            ),
        ],
    )
}

/// 构造一个返回动态索引结果的程序。
fn index_program(kind: &str) -> TacProgram {
    let mut constants = ConstPool::new();
    let mut categories = CategoryMap::new();
    let one = VReg::new(0);
    let two = VReg::new(1);
    let index = VReg::new(2);
    let source = VReg::new(3);
    let result = VReg::new(4);
    for register in [one, two, index] {
        categories.insert(register, RegisterClass::Int);
    }
    categories.insert(source, RegisterClass::ObjHandle);
    categories.insert(
        result,
        if kind == "str" {
            RegisterClass::ObjHandle
        } else {
            RegisterClass::Int
        },
    );
    let one_id = constants.intern(TacConstant::Int(1));
    let two_id = constants.intern(TacConstant::Int(2));
    let index_id = constants.intern(TacConstant::Int(-1));
    let source_op = match kind {
        "array" => TacOp::NewArray {
            elements: vec![one, two],
        },
        "tuple" => TacOp::NewTuple {
            elements: vec![one, two],
        },
        "set" => TacOp::NewSet {
            elements: vec![one, two],
        },
        "dict_table" => TacOp::NewDictTable {
            entries: vec![("first".to_owned(), one), ("second".to_owned(), two)],
        },
        "dict_column" => TacOp::NewDictColumn {
            entries: vec![("first".to_owned(), one), ("second".to_owned(), two)],
        },
        "str" => {
            let text = constants.intern(TacConstant::Str("小雪A".to_owned()));
            TacOp::LoadConst(text)
        }
        other => panic!("未知可迭代载体 {other}"),
    };
    program(
        categories,
        constants,
        vec![
            TacInstr::with_dst(TacOp::LoadConst(one_id), one, SPAN),
            TacInstr::with_dst(TacOp::LoadConst(two_id), two, SPAN),
            TacInstr::with_dst(TacOp::LoadConst(index_id), index, SPAN),
            TacInstr::with_dst(source_op, source, SPAN),
            TacInstr::with_dst(TacOp::IndexGetDynamic { source, index }, result, SPAN),
            TacInstr::new(
                TacOp::Return {
                    value: Some(result),
                },
                SPAN,
            ),
        ],
    )
}

#[test]
/// `Len` 必须覆盖所有已知可迭代载体，字符串按 Unicode 码点计数。
fn len_executes_on_all_carriers() {
    for kind in ["array", "tuple", "str", "set", "dict_table", "dict_column"] {
        let tac = len_program(kind);
        for value in [
            run::<StackCarrier>(&tac),
            run::<RegisterCarrier>(&tac),
            run::<HybridCarrier>(&tac),
        ] {
            assert_eq!(value, RuntimeValue::Int(if kind == "str" { 3 } else { 2 }));
        }
    }
}

#[test]
/// 动态索引必须复用统一索引规则，包括负索引、码点和表物理顺序。
fn dynamic_index_executes_on_all_carriers() {
    for kind in ["array", "tuple", "set", "dict_table", "dict_column"] {
        let tac = index_program(kind);
        for value in [
            run::<StackCarrier>(&tac),
            run::<RegisterCarrier>(&tac),
            run::<HybridCarrier>(&tac),
        ] {
            assert_eq!(value, RuntimeValue::Int(2));
        }
    }
    let tac = index_program("str");
    for value in [
        run::<StackCarrier>(&tac),
        run::<RegisterCarrier>(&tac),
        run::<HybridCarrier>(&tac),
    ] {
        let RuntimeValue::Str(handle) = value else {
            panic!("字符串动态索引应返回 str");
        };
        assert_eq!(handle.to_string().expect("字符串句柄应可读取"), "A");
    }
}
