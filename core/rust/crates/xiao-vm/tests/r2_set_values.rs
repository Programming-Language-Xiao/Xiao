//! 09R2F 集合指令的真实执行夹具。
//!
//! 这些用例手工构造最小 TAC，把集合结果返回到 `RunOutcome::value`，确保
//! `SetOp`/`SetCompare` 的 `step` 分支在三种载体上都真正执行，而不是只做编码往返。

use std::collections::BTreeMap;

use xiao_bytecode::research::{
    BlockId, CallSigTable, CategoryMap, ConstPool, RegisterClass, SetCompareOp, SetOpKind, TacAbi,
    TacBlock, TacConstant, TacFunction, TacInstr, TacOp, TacProgram, VReg,
};
use xiao_ir::IrSpan;
use xiao_runtime::{ArrayHandle, RuntimeValue, SetHandle};
use xiao_vm::research::{
    Carrier, HybridCarrier, RegisterCarrier, RunResult, StackCarrier, VmOptions, run_with,
    run_with_values,
};

/// 手工 TAC 夹具共用的零宽源码区间。
const SPAN: IrSpan = IrSpan::new(0, 1);

/// 构造指定集合代数的最小单函数程序。
fn program_for(op: SetOpKind) -> TacProgram {
    let mut constants = ConstPool::new();
    let mut categories = CategoryMap::new();
    let one = VReg::new(0);
    let two = VReg::new(1);
    let three = VReg::new(2);
    let left = VReg::new(3);
    let right = VReg::new(4);
    let result = VReg::new(5);
    for register in [one, two, three, left, right] {
        categories.insert(register, RegisterClass::Int);
    }
    categories.insert(result, RegisterClass::ObjHandle);
    let one_id = constants.intern(TacConstant::Int(1));
    let two_id = constants.intern(TacConstant::Int(2));
    let three_id = constants.intern(TacConstant::Int(3));
    let instructions = vec![
        TacInstr::with_dst(TacOp::LoadConst(one_id), one, SPAN),
        TacInstr::with_dst(TacOp::LoadConst(two_id), two, SPAN),
        TacInstr::with_dst(TacOp::LoadConst(three_id), three, SPAN),
        TacInstr::with_dst(
            TacOp::NewSet {
                elements: vec![one, two],
            },
            left,
            SPAN,
        ),
        TacInstr::with_dst(
            TacOp::NewSet {
                elements: vec![two, three],
            },
            right,
            SPAN,
        ),
        TacInstr::with_dst(TacOp::SetOp { op, left, right }, result, SPAN),
        TacInstr::new(
            TacOp::Return {
                value: Some(result),
            },
            SPAN,
        ),
    ];
    single_function_program(constants, categories, instructions)
}

/// 构造指定关系比较和两个整数集合的最小程序。
fn compare_program(op: SetCompareOp, left_values: &[i64], right_values: &[i64]) -> TacProgram {
    let mut constants = ConstPool::new();
    let mut categories = CategoryMap::new();
    let mut next = 0_u32;
    let mut load = |value: i64, instructions: &mut Vec<TacInstr>| {
        let register = VReg::new(next);
        next += 1;
        categories.insert(register, RegisterClass::Int);
        let constant = constants.intern(TacConstant::Int(value));
        instructions.push(TacInstr::with_dst(
            TacOp::LoadConst(constant),
            register,
            SPAN,
        ));
        register
    };
    let mut instructions = Vec::new();
    let left_elements = left_values
        .iter()
        .map(|value| load(*value, &mut instructions))
        .collect();
    let right_elements = right_values
        .iter()
        .map(|value| load(*value, &mut instructions))
        .collect();
    let left = VReg::new(next);
    next += 1;
    categories.insert(left, RegisterClass::ObjHandle);
    let right = VReg::new(next);
    next += 1;
    categories.insert(right, RegisterClass::ObjHandle);
    let result = VReg::new(next);
    categories.insert(result, RegisterClass::Bool);
    instructions.push(TacInstr::with_dst(
        TacOp::NewSet {
            elements: left_elements,
        },
        left,
        SPAN,
    ));
    instructions.push(TacInstr::with_dst(
        TacOp::NewSet {
            elements: right_elements,
        },
        right,
        SPAN,
    ));
    instructions.push(TacInstr::with_dst(
        TacOp::SetCompare { op, left, right },
        result,
        SPAN,
    ));
    instructions.push(TacInstr::new(
        TacOp::Return {
            value: Some(result),
        },
        SPAN,
    ));
    single_function_program(constants, categories, instructions)
}

/// 构造成员判断方向的最小程序。
fn membership_program(member: i64, op: SetCompareOp) -> TacProgram {
    let mut constants = ConstPool::new();
    let mut categories = CategoryMap::new();
    let member_register = VReg::new(0);
    let first = VReg::new(1);
    let second = VReg::new(2);
    let set = VReg::new(3);
    let result = VReg::new(4);
    for register in [member_register, first, second] {
        categories.insert(register, RegisterClass::Int);
    }
    categories.insert(set, RegisterClass::ObjHandle);
    categories.insert(result, RegisterClass::Bool);
    let member_id = constants.intern(TacConstant::Int(member));
    let first_id = constants.intern(TacConstant::Int(1));
    let second_id = constants.intern(TacConstant::Int(2));
    let instructions = vec![
        TacInstr::with_dst(TacOp::LoadConst(member_id), member_register, SPAN),
        TacInstr::with_dst(TacOp::LoadConst(first_id), first, SPAN),
        TacInstr::with_dst(TacOp::LoadConst(second_id), second, SPAN),
        TacInstr::with_dst(
            TacOp::NewSet {
                elements: vec![first, second],
            },
            set,
            SPAN,
        ),
        TacInstr::with_dst(
            TacOp::SetCompare {
                op,
                left: member_register,
                right: set,
            },
            result,
            SPAN,
        ),
        TacInstr::new(
            TacOp::Return {
                value: Some(result),
            },
            SPAN,
        ),
    ];
    single_function_program(constants, categories, instructions)
}

/// 构造右操作数为数组的非法集合运算程序。
fn invalid_operation_program() -> TacProgram {
    let mut constants = ConstPool::new();
    let mut categories = CategoryMap::new();
    let value = VReg::new(0);
    let element = VReg::new(1);
    let set = VReg::new(2);
    let array = VReg::new(3);
    let result = VReg::new(4);
    for register in [value, element] {
        categories.insert(register, RegisterClass::Int);
    }
    categories.insert(set, RegisterClass::ObjHandle);
    categories.insert(array, RegisterClass::ObjHandle);
    categories.insert(result, RegisterClass::ObjHandle);
    let constant = constants.intern(TacConstant::Int(1));
    let instructions = vec![
        TacInstr::with_dst(TacOp::LoadConst(constant), value, SPAN),
        TacInstr::with_dst(TacOp::LoadConst(constant), element, SPAN),
        TacInstr::with_dst(
            TacOp::NewSet {
                elements: vec![element],
            },
            set,
            SPAN,
        ),
        TacInstr::with_dst(
            TacOp::NewArray {
                elements: vec![value],
            },
            array,
            SPAN,
        ),
        TacInstr::with_dst(
            TacOp::SetOp {
                op: SetOpKind::Union,
                left: set,
                right: array,
            },
            result,
            SPAN,
        ),
        TacInstr::new(
            TacOp::Return {
                value: Some(result),
            },
            SPAN,
        ),
    ];
    single_function_program(constants, categories, instructions)
}

/// 在入口检查失败块和后续操作之间组装参数化 TAC 程序。
fn dynamic_check_program(
    kind: &str,
    operation: TacOp,
    result: VReg,
    categories: CategoryMap,
    constants: ConstPool,
    parameters: Vec<VReg>,
    instructions: Vec<TacInstr>,
) -> TacProgram {
    let mut instructions = instructions;
    instructions.insert(
        0,
        TacInstr::new(
            TacOp::Check {
                kind: kind.to_owned(),
                value: parameters[0],
                on_failure: BlockId::new(1),
            },
            SPAN,
        ),
    );
    instructions.push(TacInstr::with_dst(operation, result, SPAN));
    instructions.push(TacInstr::new(
        TacOp::Return {
            value: Some(result),
        },
        SPAN,
    ));
    let error = VReg::new(20);
    let mut categories = categories;
    categories.insert(error, RegisterClass::Dynamic);
    parameter_program(
        constants,
        categories,
        parameters,
        vec![
            TacBlock {
                id: BlockId::new(0),
                scope: 0,
                instructions,
            },
            TacBlock {
                id: BlockId::new(1),
                scope: 0,
                instructions: vec![
                    TacInstr::with_dst(
                        TacOp::MakeError {
                            type_name: "TypeError".to_owned(),
                            code: None,
                            message: None,
                        },
                        error,
                        SPAN,
                    ),
                    TacInstr::new(TacOp::Raise { value: error }, SPAN),
                ],
            },
        ],
    )
}

/// 构造带动态集合运算检查的参数化程序。
fn dynamic_set_operation_program() -> TacProgram {
    let mut constants = ConstPool::new();
    let mut categories = CategoryMap::new();
    let argument = VReg::new(0);
    let element = VReg::new(1);
    let left = VReg::new(2);
    let right = VReg::new(3);
    let result = VReg::new(4);
    categories.insert(argument, RegisterClass::Dynamic);
    categories.insert(element, RegisterClass::Int);
    categories.insert(left, RegisterClass::ObjHandle);
    categories.insert(right, RegisterClass::ObjHandle);
    categories.insert(result, RegisterClass::ObjHandle);
    let one = constants.intern(TacConstant::Int(1));
    let two = constants.intern(TacConstant::Int(2));
    dynamic_check_program(
        "set_operation",
        TacOp::SetOp {
            op: SetOpKind::Union,
            left,
            right,
        },
        result,
        categories,
        constants,
        vec![argument],
        vec![
            TacInstr::with_dst(TacOp::LoadConst(one), element, SPAN),
            TacInstr::with_dst(
                TacOp::NewSet {
                    elements: vec![element],
                },
                left,
                SPAN,
            ),
            TacInstr::with_dst(TacOp::LoadConst(two), element, SPAN),
            TacInstr::with_dst(
                TacOp::NewSet {
                    elements: vec![element],
                },
                right,
                SPAN,
            ),
        ],
    )
}

/// 构造带动态集合比较检查的参数化程序。
fn dynamic_set_comparison_program() -> TacProgram {
    let mut constants = ConstPool::new();
    let mut categories = CategoryMap::new();
    let argument = VReg::new(0);
    let element = VReg::new(1);
    let left = VReg::new(2);
    let right = VReg::new(3);
    let result = VReg::new(4);
    categories.insert(argument, RegisterClass::Dynamic);
    categories.insert(element, RegisterClass::Int);
    categories.insert(left, RegisterClass::ObjHandle);
    categories.insert(right, RegisterClass::ObjHandle);
    categories.insert(result, RegisterClass::Bool);
    let one = constants.intern(TacConstant::Int(1));
    dynamic_check_program(
        "set_comparison",
        TacOp::SetCompare {
            op: SetCompareOp::Equal,
            left,
            right,
        },
        result,
        categories,
        constants,
        vec![argument],
        vec![
            TacInstr::with_dst(TacOp::LoadConst(one), element, SPAN),
            TacInstr::with_dst(
                TacOp::NewSet {
                    elements: vec![element],
                },
                left,
                SPAN,
            ),
            TacInstr::with_dst(TacOp::LoadConst(one), element, SPAN),
            TacInstr::with_dst(
                TacOp::NewSet {
                    elements: vec![element],
                },
                right,
                SPAN,
            ),
        ],
    )
}

/// 构造带动态成员检查的参数化程序。
fn dynamic_set_membership_program() -> TacProgram {
    let mut constants = ConstPool::new();
    let mut categories = CategoryMap::new();
    let argument = VReg::new(0);
    let element = VReg::new(1);
    let right = VReg::new(2);
    let result = VReg::new(3);
    categories.insert(argument, RegisterClass::Dynamic);
    categories.insert(element, RegisterClass::Int);
    categories.insert(right, RegisterClass::ObjHandle);
    categories.insert(result, RegisterClass::Bool);
    let one = constants.intern(TacConstant::Int(1));
    dynamic_check_program(
        "set_membership",
        TacOp::SetCompare {
            op: SetCompareOp::Member,
            left: element,
            right,
        },
        result,
        categories,
        constants,
        vec![argument],
        vec![
            TacInstr::with_dst(TacOp::LoadConst(one), element, SPAN),
            TacInstr::with_dst(
                TacOp::NewSet {
                    elements: vec![element],
                },
                right,
                SPAN,
            ),
        ],
    )
}

/// 构造带可哈希性检查的参数化程序。
fn dynamic_set_hashability_program() -> TacProgram {
    let constants = ConstPool::new();
    let mut categories = CategoryMap::new();
    let argument = VReg::new(0);
    let result = VReg::new(1);
    categories.insert(argument, RegisterClass::Dynamic);
    categories.insert(result, RegisterClass::Dynamic);
    dynamic_check_program(
        "set_hashability",
        TacOp::Move(argument),
        result,
        categories,
        constants,
        vec![argument],
        Vec::new(),
    )
}

/// 用单个基本块包装无参数函数程序。
fn single_function_program(
    constants: ConstPool,
    categories: CategoryMap,
    instructions: Vec<TacInstr>,
) -> TacProgram {
    parameter_program(
        constants,
        categories,
        Vec::new(),
        vec![TacBlock {
            id: BlockId::new(0),
            scope: 0,
            instructions,
        }],
    )
}

/// 构造带入口参数和指定基本块的完整 TAC 程序。
fn parameter_program(
    constants: ConstPool,
    categories: CategoryMap,
    parameters: Vec<VReg>,
    blocks: Vec<TacBlock>,
) -> TacProgram {
    TacProgram {
        version: 1,
        abi: TacAbi {
            bytecode_abi_version: 1,
            runtime_abi_version: 1,
            ir_version: 1,
            language_version: "0.1.0".to_owned(),
            target: "r2f-set-test".to_owned(),
        },
        constants,
        signatures: CallSigTable::new(),
        functions: vec![TacFunction {
            name: String::new(),
            signature: None,
            entry: BlockId::new(0),
            blocks,
            parameters,
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

/// 提取整数集合的顺序元素，供结果断言使用。
fn set_values(value: &RuntimeValue) -> Vec<i64> {
    let RuntimeValue::Set(handle) = value else {
        panic!("期望集合结果，实际为 {value:?}");
    };
    handle
        .with_elements(|elements| {
            elements
                .iter()
                .map(|element| match element {
                    RuntimeValue::Int(value) => *value,
                    other => panic!("夹具只应产生 int 集合，实际为 {other:?}"),
                })
                .collect()
        })
        .expect("集合句柄应可读取")
}

/// 在指定载体上执行程序并断言集合结果。
fn assert_set_result<C: Carrier>(program: &TacProgram, expected: &[i64]) {
    let outcome = run_with::<C>(program, VmOptions::new());
    assert!(
        outcome.result.is_success(),
        "集合指令应成功: {:?}",
        outcome.result
    );
    assert_eq!(
        set_values(outcome.value.as_ref().expect("应返回集合")),
        expected
    );
}

/// 在基础并集夹具中插入重复元素。
fn duplicate_set_program() -> TacProgram {
    let mut program = program_for(SetOpKind::Union);
    if let TacOp::NewSet { elements } = &mut program.functions[0].blocks[0].instructions[3].op {
        elements.insert(1, VReg::new(0));
    }
    if let TacOp::NewSet { elements } = &mut program.functions[0].blocks[0].instructions[4].op {
        elements.insert(1, VReg::new(1));
    }
    program
}

/// 在指定载体上执行参数化检查程序并断言稳定错误码。
fn assert_dynamic_check_error<C: Carrier>(
    program: &TacProgram,
    value: RuntimeValue,
    expected_code: &str,
) {
    let outcome = run_with_values::<C>(program, VmOptions::new(), &[value]);
    assert_eq!(outcome.result.error_code(), Some(expected_code));
    assert!(outcome.value.is_none(), "失败路径不应返回值");
}

#[test]
/// 四种集合代数都必须在三种载体上真实执行，并保留冻结的操作数顺序。
fn executes_all_set_algebras_on_all_carriers() {
    for (op, expected) in [
        (SetOpKind::Union, vec![1, 2, 3]),
        (SetOpKind::Intersection, vec![2]),
        (SetOpKind::Difference, vec![1]),
        (SetOpKind::SymmetricDifference, vec![1, 3]),
    ] {
        let program = program_for(op);
        assert_set_result::<StackCarrier>(&program, &expected);
        assert_set_result::<RegisterCarrier>(&program, &expected);
        assert_set_result::<HybridCarrier>(&program, &expected);
    }
}

#[test]
/// 集合比较、成员方向和真子集边界必须返回可观察布尔值。
fn executes_set_comparisons_and_membership_direction() {
    let equal = compare_program(SetCompareOp::Equal, &[1], &[1]);
    let proper_subset = compare_program(SetCompareOp::ProperSubset, &[1], &[1, 2]);
    for program in [&equal, &proper_subset] {
        for outcome in [
            run_with::<StackCarrier>(program, VmOptions::new()),
            run_with::<RegisterCarrier>(program, VmOptions::new()),
            run_with::<HybridCarrier>(program, VmOptions::new()),
        ] {
            assert!(matches!(outcome.result, RunResult::Success));
            assert_eq!(outcome.value, Some(RuntimeValue::Bool(true)));
        }
    }
    let equal_sets = compare_program(SetCompareOp::Equal, &[1, 2], &[2, 1]);
    let proper_equal = compare_program(SetCompareOp::ProperSubset, &[1, 2], &[1, 2]);
    for (program, expected) in [(&equal_sets, true), (&proper_equal, false)] {
        let outcome = run_with::<StackCarrier>(program, VmOptions::new());
        assert_eq!(outcome.value, Some(RuntimeValue::Bool(expected)));
    }
    let member = membership_program(1, SetCompareOp::Member);
    let not_member = membership_program(3, SetCompareOp::NotMember);
    for program in [&member, &not_member] {
        let outcome = run_with::<HybridCarrier>(program, VmOptions::new());
        assert_eq!(outcome.value, Some(RuntimeValue::Bool(true)));
    }
}

#[test]
/// 非集合操作数仍必须沿用集合专用稳定错误码。
fn set_operation_reports_stable_error_for_non_set_operand() {
    let program = invalid_operation_program();
    for outcome in [
        run_with::<StackCarrier>(&program, VmOptions::new()),
        run_with::<RegisterCarrier>(&program, VmOptions::new()),
        run_with::<HybridCarrier>(&program, VmOptions::new()),
    ] {
        assert_eq!(
            outcome.result.error_code(),
            Some(xiao_runtime::SET_OPERATION_CODE)
        );
    }
}

#[test]
/// 集合构造的重复元素必须在运行时去重，而不能只依赖静态字面量检查。
fn repeated_runtime_set_elements_are_deduplicated() {
    let program = duplicate_set_program();
    assert_set_result::<StackCarrier>(&program, &[1, 2, 3]);
    assert_set_result::<RegisterCarrier>(&program, &[1, 2, 3]);
    assert_set_result::<HybridCarrier>(&program, &[1, 2, 3]);
}

#[test]
/// 动态集合运算、比较与成员判断必须先经过对应 Check 失败块。
fn dynamic_set_checks_reject_non_set_and_unhashable_values() {
    let array =
        RuntimeValue::Array(ArrayHandle::new(vec![RuntimeValue::Int(1)]).expect("数组应可分配"));
    let operation = dynamic_set_operation_program();
    let comparison = dynamic_set_comparison_program();
    let membership = dynamic_set_membership_program();
    let hashability = dynamic_set_hashability_program();
    for (program, code) in [
        (&operation, xiao_runtime::SET_OPERATION_CODE),
        (&comparison, xiao_runtime::SET_COMPARISON_CODE),
        (&membership, xiao_runtime::SET_MEMBERSHIP_CODE),
        (&hashability, xiao_runtime::CONTAINER_HASHABILITY_CODE),
    ] {
        assert_dynamic_check_error::<StackCarrier>(program, array.clone(), code);
        assert_dynamic_check_error::<RegisterCarrier>(program, array.clone(), code);
        assert_dynamic_check_error::<HybridCarrier>(program, array.clone(), code);
    }
}

#[test]
/// 真实集合实参通过动态检查后仍应完成集合指令，而不是被检查路径吞掉。
fn dynamic_set_values_pass_checks_and_execute() {
    let set = RuntimeValue::Set(SetHandle::new(vec![RuntimeValue::Int(1)]).expect("集合应可分配"));
    let operation = run_with_values::<StackCarrier>(
        &dynamic_set_operation_program(),
        VmOptions::new(),
        std::slice::from_ref(&set),
    );
    assert!(operation.result.is_success());
    assert_eq!(
        set_values(operation.value.as_ref().expect("应返回集合")),
        vec![1, 2]
    );

    let comparison = run_with_values::<RegisterCarrier>(
        &dynamic_set_comparison_program(),
        VmOptions::new(),
        std::slice::from_ref(&set),
    );
    assert_eq!(comparison.value, Some(RuntimeValue::Bool(true)));

    let membership = run_with_values::<HybridCarrier>(
        &dynamic_set_membership_program(),
        VmOptions::new(),
        &[RuntimeValue::Int(1)],
    );
    assert_eq!(membership.value, Some(RuntimeValue::Bool(true)));

    let hashability = run_with_values::<StackCarrier>(
        &dynamic_set_hashability_program(),
        VmOptions::new(),
        &[RuntimeValue::Int(1)],
    );
    assert_eq!(hashability.value, Some(RuntimeValue::Int(1)));
}
