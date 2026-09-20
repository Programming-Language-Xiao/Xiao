//! 09R2B1 选择器结果值验证。
//!
//! 这些用例使用最小手工 TAC 夹具把选择结果返回到 `RunOutcome::value`，避免
//! 只断言「运行成功」而漏掉选择顺序、步长和结果形状。每个夹具都在三种载体上
//! 复用同一份计划和期望值。

use std::collections::BTreeMap;

use xiao_bytecode::research::{
    BlockId, CallSigTable, CategoryMap, ConstPool, PathStep, RegisterClass, TacAbi, TacBlock,
    TacConstant, TacFunction, TacInstr, TacOp, TacProgram, VReg,
};
use xiao_ir::{
    IrArrayShape, IrDictTypeEntry, IrSelectionItemPlan, IrSelectionPath, IrSelectionPathSegment,
    IrSelectionPlan, IrSpan, IrStepPlan, IrType,
};
use xiao_runtime::RuntimeValue;
use xiao_vm::research::{
    Carrier, HybridCarrier, RegisterCarrier, StackCarrier, VmOptions, run_with,
    run_with_machine_seed,
};

/// 手工 TAC 指令共用的稳定源代码跨度。
const SPAN: IrSpan = IrSpan::new(0, 1);

/// 构造最小单函数选择器程序。
struct SelectorProgramBuilder {
    constants: ConstPool,
    instructions: Vec<TacInstr>,
    categories: CategoryMap,
    next_register: u32,
}

impl SelectorProgramBuilder {
    /// 创建空夹具。
    fn new() -> Self {
        Self {
            constants: ConstPool::new(),
            instructions: Vec::new(),
            categories: CategoryMap::new(),
            next_register: 0,
        }
    }

    /// 分配一个带类别的虚拟寄存器。
    fn register(&mut self, class: RegisterClass) -> VReg {
        let register = VReg::new(self.next_register);
        self.next_register += 1;
        self.categories.insert(register, class);
        register
    }

    /// 加载整数常量。
    fn int(&mut self, value: i64) -> VReg {
        let register = self.register(RegisterClass::Int);
        let constant = self.constants.intern(TacConstant::Int(value));
        self.instructions.push(TacInstr::with_dst(
            TacOp::LoadConst(constant),
            register,
            SPAN,
        ));
        register
    }

    /// 加载字符串常量。
    fn string(&mut self, value: &str) -> VReg {
        let register = self.register(RegisterClass::ObjHandle);
        let constant = self.constants.intern(TacConstant::Str(value.to_owned()));
        self.instructions.push(TacInstr::with_dst(
            TacOp::LoadConst(constant),
            register,
            SPAN,
        ));
        register
    }

    /// 构造数组。
    fn array(&mut self, elements: Vec<VReg>) -> VReg {
        let register = self.register(RegisterClass::ObjHandle);
        self.instructions.push(TacInstr::with_dst(
            TacOp::NewArray { elements },
            register,
            SPAN,
        ));
        register
    }

    /// 直接构造整数数组，避免夹具调用处出现交错可变借用。
    fn int_array(&mut self, values: &[i64]) -> VReg {
        let elements = values.iter().map(|value| self.int(*value)).collect();
        self.array(elements)
    }

    /// 构造元组。
    fn tuple(&mut self, elements: Vec<VReg>) -> VReg {
        let register = self.register(RegisterClass::ObjHandle);
        self.instructions.push(TacInstr::with_dst(
            TacOp::NewTuple { elements },
            register,
            SPAN,
        ));
        register
    }

    /// 直接构造整数元组。
    fn int_tuple(&mut self, values: &[i64]) -> VReg {
        let elements = values.iter().map(|value| self.int(*value)).collect();
        self.tuple(elements)
    }

    /// 构造有序字典列。
    fn dict_column(&mut self, entries: Vec<(&str, VReg)>) -> VReg {
        let register = self.register(RegisterClass::ObjHandle);
        self.instructions.push(TacInstr::with_dst(
            TacOp::NewDictColumn {
                entries: entries
                    .into_iter()
                    .map(|(key, value)| (key.to_owned(), value))
                    .collect(),
            },
            register,
            SPAN,
        ));
        register
    }

    /// 直接构造整数字典列。
    fn int_dict_column(&mut self, entries: &[(&str, i64)]) -> VReg {
        let entries = entries
            .iter()
            .map(|(key, value)| (*key, self.int(*value)))
            .collect();
        self.dict_column(entries)
    }

    /// 发出高级选择指令。
    fn selector(
        &mut self,
        source: VReg,
        plan: u32,
        step: Option<VReg>,
        random_counts: Vec<Option<VReg>>,
    ) -> VReg {
        let register = self.register(RegisterClass::ObjHandle);
        self.instructions.push(TacInstr::with_dst(
            TacOp::SelectorApply {
                source,
                plan,
                step,
                random_counts,
            },
            register,
            SPAN,
        ));
        register
    }

    /// 发出单项精确索引指令。
    fn index_get(&mut self, source: VReg, path: Vec<PathStep>, class: RegisterClass) -> VReg {
        let register = self.register(class);
        self.instructions.push(TacInstr::with_dst(
            TacOp::IndexGet { source, path },
            register,
            SPAN,
        ));
        register
    }

    /// 以指定返回寄存器封装程序。
    fn finish(self, selection_plans: Vec<IrSelectionPlan>, return_register: VReg) -> TacProgram {
        let function = TacFunction {
            name: String::new(),
            signature: None,
            entry: BlockId::new(0),
            blocks: vec![TacBlock {
                id: BlockId::new(0),
                scope: 0,
                instructions: self
                    .instructions
                    .into_iter()
                    .chain([TacInstr::new(
                        TacOp::Return {
                            value: Some(return_register),
                        },
                        SPAN,
                    )])
                    .collect(),
            }],
            parameters: Vec::new(),
            locals: Vec::new(),
            categories: self.categories.clone(),
            scopes: vec![0],
            handlers: Vec::new(),
            value_registers: BTreeMap::new(),
            span: SPAN,
        };
        TacProgram {
            version: 1,
            abi: TacAbi {
                bytecode_abi_version: 1,
                runtime_abi_version: 1,
                ir_version: 1,
                language_version: "0.1.0".to_owned(),
                target: "r2b1-selector-test".to_owned(),
            },
            constants: self.constants,
            signatures: CallSigTable::new(),
            functions: vec![function],
            categories: self.categories,
            plans: Vec::new(),
            selection_plans,
            broadcast_assignment_plans: Vec::new(),
            random_seed_plans: Vec::new(),
            table_definitions: Vec::new(),
            unsupported: Vec::new(),
        }
    }
}

/// 构造单段数字路径。
fn index_path(index: usize) -> IrSelectionPath {
    vec![IrSelectionPathSegment::Index {
        raw: index as i128,
        resolved: Some(index),
    }]
}

/// 构造单段键名路径。
fn key_path(key: &str) -> IrSelectionPath {
    vec![IrSelectionPathSegment::Key(key.to_owned())]
}

/// 构造整数标量类型。
fn int_type() -> IrType {
    IrType::Scalar {
        name: "int".to_owned(),
    }
}

/// 构造指定长度的异构数组类型。
fn array_type(length: usize) -> IrType {
    IrType::Array {
        shape: IrArrayShape::Heterogeneous {
            elements: vec![int_type(); length],
        },
    }
}

/// 构造指定长度的元组类型。
fn tuple_type(length: usize) -> IrType {
    IrType::Tuple {
        elements: vec![int_type(); length],
    }
}

/// 构造一份最小选择计划。
fn selection_plan(
    items: Vec<IrSelectionItemPlan>,
    selected_paths: Vec<IrSelectionPath>,
    result_type: IrType,
) -> IrSelectionPlan {
    IrSelectionPlan {
        span: SPAN,
        source_type: IrType::Dynamic,
        result_type,
        items,
        selected_paths,
        target_types: Vec::new(),
        step: None,
        requires_runtime_check: false,
        with_replacement: false,
        has_duplicates: false,
    }
}

/// 从整数数组或元组读取值并同时确认根形状。
fn integer_elements(value: &RuntimeValue) -> Vec<i64> {
    match value {
        RuntimeValue::Array(handle) => handle
            .with_elements(|elements| {
                elements
                    .iter()
                    .map(|element| match element {
                        RuntimeValue::Int(value) => *value,
                        other => panic!("结果元素应为 int，实际为 {other:?}"),
                    })
                    .collect()
            })
            .expect("数组结果应可读取"),
        RuntimeValue::Tuple(handle) => handle
            .with_elements(|elements| {
                elements
                    .iter()
                    .map(|element| match element {
                        RuntimeValue::Int(value) => *value,
                        other => panic!("结果元素应为 int，实际为 {other:?}"),
                    })
                    .collect()
            })
            .expect("元组结果应可读取"),
        other => panic!("结果应为数组或元组，实际为 {other:?}"),
    }
}

/// 从嵌套数组读取整数形状。
fn nested_integer_elements(value: &RuntimeValue) -> Vec<Vec<i64>> {
    let RuntimeValue::Array(handle) = value else {
        panic!("嵌套结果根应为数组，实际为 {value:?}");
    };
    handle
        .with_elements(|elements| elements.iter().map(integer_elements).collect())
        .expect("嵌套数组结果应可读取")
}

/// 读取字典列或元组中的整数，用于形状与顺序断言。
fn dictionary_or_tuple_integers(value: &RuntimeValue) -> Vec<i64> {
    match value {
        RuntimeValue::DictColumn(handle) => handle
            .with_entries(|entries| {
                entries
                    .iter()
                    .map(|(_, value)| match value {
                        RuntimeValue::Int(value) => *value,
                        other => panic!("字典列值应为 int，实际为 {other:?}"),
                    })
                    .collect()
            })
            .expect("字典列结果应可读取"),
        RuntimeValue::Tuple(_) => integer_elements(value),
        other => panic!("结果应为字典列或元组，实际为 {other:?}"),
    }
}

/// 断言一个载体返回成功并交给调用方检查返回值。
fn assert_machine<C: Carrier>(program: &TacProgram, check: &impl Fn(&RuntimeValue), machine: &str) {
    let outcome = run_with::<C>(program, VmOptions::new());
    assert!(
        outcome.result.is_success(),
        "{machine} 选择执行失败: {:?}",
        outcome.result
    );
    let value = outcome
        .value
        .as_ref()
        .unwrap_or_else(|| panic!("{machine} 成功结果缺少返回值"));
    check(value);
}

/// 在三种载体上断言同一选择结果。
fn assert_all_machines(program: &TacProgram, check: impl Fn(&RuntimeValue)) {
    assert_machine::<StackCarrier>(program, &check, "栈式");
    assert_machine::<RegisterCarrier>(program, &check, "分类型寄存器式");
    assert_machine::<HybridCarrier>(program, &check, "混合式");
}

/// 断言一个带固定随机种子的载体结果。
fn assert_seeded_machine<C: Carrier>(
    program: &TacProgram,
    seed: u128,
    check: &impl Fn(&RuntimeValue),
    machine: &str,
) {
    let outcome = run_with_machine_seed::<C>(program, VmOptions::new(), seed);
    assert!(
        outcome.result.is_success(),
        "{machine} 随机选择执行失败: {:?}",
        outcome.result
    );
    let value = outcome
        .value
        .as_ref()
        .unwrap_or_else(|| panic!("{machine} 随机成功结果缺少返回值"));
    check(value);
}

/// 在三种载体上用同一随机种子断言选择结果。
fn assert_all_machines_seeded(program: &TacProgram, seed: u128, check: impl Fn(&RuntimeValue)) {
    assert_seeded_machine::<StackCarrier>(program, seed, &check, "栈式");
    assert_seeded_machine::<RegisterCarrier>(program, seed, &check, "分类型寄存器式");
    assert_seeded_machine::<HybridCarrier>(program, seed, &check, "混合式");
}

#[test]
/// 单项精确索引返回标量，多选保留源码顺序与重复项。
fn selector_returns_exact_and_ordered_multi_values() {
    let mut exact = SelectorProgramBuilder::new();
    let source = exact.int_array(&[10, 20, 30]);
    let result = exact.index_get(source, vec![PathStep::Index(2)], RegisterClass::Int);
    let exact_program = exact.finish(Vec::new(), result);
    assert_all_machines(&exact_program, |value| {
        assert_eq!(value, &RuntimeValue::Int(30));
    });

    let mut multi = SelectorProgramBuilder::new();
    let source = multi.int_array(&[10, 20, 30]);
    let paths = vec![index_path(2), index_path(0), index_path(2)];
    let plan = selection_plan(
        paths
            .iter()
            .cloned()
            .map(|path| IrSelectionItemPlan::Exact { path })
            .collect(),
        paths,
        array_type(3),
    );
    let result = multi.selector(source, 0, None, Vec::new());
    let program = multi.finish(vec![plan], result);
    assert_all_machines(&program, |value| {
        assert!(matches!(value, RuntimeValue::Array(_)), "多选必须返回数组");
        assert_eq!(integer_elements(value), vec![30, 10, 30]);
    });
}

#[test]
/// 动态非单位步长必须对每个选择项独立应用，而不是对合并结果全局抽取。
fn selector_step_is_applied_per_item() {
    let mut builder = SelectorProgramBuilder::new();
    let source = builder.int_array(&[10, 20, 30, 40, 50]);
    let step = builder.int(2);
    let plan = IrSelectionPlan {
        step: Some(IrStepPlan {
            value: None,
            dynamic: true,
        }),
        items: vec![
            IrSelectionItemPlan::Range {
                start: index_path(0),
                end: index_path(2),
                include_start: true,
                include_end: true,
            },
            IrSelectionItemPlan::Range {
                start: index_path(1),
                end: index_path(4),
                include_start: true,
                include_end: true,
            },
        ],
        result_type: array_type(4),
        ..selection_plan(Vec::new(), Vec::new(), array_type(4))
    };
    let result = builder.selector(source, 0, Some(step), Vec::new());
    let program = builder.finish(vec![plan], result);
    assert_all_machines(&program, |value| {
        assert_eq!(integer_elements(value), vec![10, 30, 20, 40]);
    });
}

#[test]
/// 四种单边范围、双端闭区间和全选必须保持包含关系。
fn selector_ranges_and_all_return_expected_values() {
    let mut ranges = SelectorProgramBuilder::new();
    let source = ranges.int_array(&[0, 1, 2, 3, 4]);
    let open = Vec::new();
    let items = vec![
        IrSelectionItemPlan::Range {
            start: open.clone(),
            end: index_path(2),
            include_start: false,
            include_end: false,
        },
        IrSelectionItemPlan::Range {
            start: open.clone(),
            end: index_path(2),
            include_start: false,
            include_end: true,
        },
        IrSelectionItemPlan::Range {
            start: index_path(2),
            end: open.clone(),
            include_start: false,
            include_end: false,
        },
        IrSelectionItemPlan::Range {
            start: index_path(2),
            end: open.clone(),
            include_start: true,
            include_end: false,
        },
        IrSelectionItemPlan::Range {
            start: index_path(1),
            end: index_path(3),
            include_start: true,
            include_end: true,
        },
    ];
    let plan = IrSelectionPlan {
        requires_runtime_check: true,
        items,
        result_type: array_type(13),
        ..selection_plan(Vec::new(), Vec::new(), array_type(13))
    };
    let result = ranges.selector(source, 0, None, Vec::new());
    let program = ranges.finish(vec![plan], result);
    assert_all_machines(&program, |value| {
        assert_eq!(
            integer_elements(value),
            vec![0, 1, 0, 1, 2, 3, 4, 2, 3, 4, 1, 2, 3]
        );
    });

    let mut all = SelectorProgramBuilder::new();
    let source = all.int_array(&[10, 20, 30, 40]);
    let all_plan = IrSelectionPlan {
        requires_runtime_check: true,
        items: vec![IrSelectionItemPlan::All],
        result_type: array_type(4),
        ..selection_plan(Vec::new(), Vec::new(), array_type(4))
    };
    let result = all.selector(source, 0, None, Vec::new());
    let program = all.finish(vec![all_plan], result);
    assert_all_machines(&program, |value| {
        assert_eq!(integer_elements(value), vec![10, 20, 30, 40]);
    });

    let mut reverse = SelectorProgramBuilder::new();
    let source = reverse.int_array(&[10, 20, 30, 40]);
    let step = reverse.int(-2);
    let reverse_plan = IrSelectionPlan {
        requires_runtime_check: true,
        step: Some(IrStepPlan {
            value: None,
            dynamic: true,
        }),
        items: vec![IrSelectionItemPlan::All],
        result_type: array_type(2),
        ..selection_plan(Vec::new(), Vec::new(), array_type(2))
    };
    let result = reverse.selector(source, 0, Some(step), Vec::new());
    let program = reverse.finish(vec![reverse_plan], result);
    assert_all_machines(&program, |value| {
        assert_eq!(integer_elements(value), vec![40, 20]);
    });
}

#[test]
/// 多选、元组、字符串和嵌套路径都必须保留来源根形状。
fn selector_preserves_source_and_nested_shapes() {
    let mut tuple = SelectorProgramBuilder::new();
    let source = tuple.int_tuple(&[10, 20, 30]);
    let paths = vec![index_path(2), index_path(0)];
    let plan = selection_plan(
        paths
            .iter()
            .cloned()
            .map(|path| IrSelectionItemPlan::Exact { path })
            .collect(),
        paths,
        tuple_type(2),
    );
    let result = tuple.selector(source, 0, None, Vec::new());
    let program = tuple.finish(vec![plan], result);
    assert_all_machines(&program, |value| {
        assert!(
            matches!(value, RuntimeValue::Tuple(_)),
            "元组选择不能退化为数组"
        );
        assert_eq!(integer_elements(value), vec![30, 10]);
    });

    let mut text = SelectorProgramBuilder::new();
    let source = text.string("abcd");
    let text_plan = IrSelectionPlan {
        requires_runtime_check: true,
        items: vec![IrSelectionItemPlan::All],
        result_type: IrType::Scalar {
            name: "str".to_owned(),
        },
        ..selection_plan(
            Vec::new(),
            Vec::new(),
            IrType::Scalar {
                name: "str".to_owned(),
            },
        )
    };
    let result = text.selector(source, 0, None, Vec::new());
    let program = text.finish(vec![text_plan], result);
    assert_all_machines(&program, |value| {
        let RuntimeValue::Str(handle) = value else {
            panic!("字符串选择必须返回 str，实际为 {value:?}");
        };
        assert_eq!(handle.to_string().expect("应读取字符串"), "abcd");
    });

    let mut nested = SelectorProgramBuilder::new();
    let first = nested.int_array(&[1, 2]);
    let second = nested.int_array(&[3, 4]);
    let source = nested.array(vec![first, second]);
    let paths = vec![
        vec![
            IrSelectionPathSegment::Index {
                raw: 0,
                resolved: Some(0),
            },
            IrSelectionPathSegment::Index {
                raw: 0,
                resolved: Some(0),
            },
        ],
        vec![
            IrSelectionPathSegment::Index {
                raw: 0,
                resolved: Some(0),
            },
            IrSelectionPathSegment::Index {
                raw: 1,
                resolved: Some(1),
            },
        ],
        vec![
            IrSelectionPathSegment::Index {
                raw: 1,
                resolved: Some(1),
            },
            IrSelectionPathSegment::Index {
                raw: 1,
                resolved: Some(1),
            },
        ],
    ];
    let plan = selection_plan(
        paths
            .iter()
            .cloned()
            .map(|path| IrSelectionItemPlan::Exact { path })
            .collect(),
        paths,
        IrType::Array {
            shape: IrArrayShape::Heterogeneous {
                elements: vec![array_type(2), array_type(1)],
            },
        },
    );
    let result = nested.selector(source, 0, None, Vec::new());
    let program = nested.finish(vec![plan], result);
    assert_all_machines(&program, |value| {
        assert_eq!(nested_integer_elements(value), vec![vec![1, 2], vec![4]]);
    });
}

#[test]
/// 字典列重复直接键和放回随机都必须产生按选择顺序排列的元组。
fn dictionary_repetition_returns_ordered_tuple() {
    let mut repeated = SelectorProgramBuilder::new();
    let source = repeated.int_dict_column(&[("name", 11), ("level", 22)]);
    let paths = vec![index_path(0), key_path("name")];
    let plan = selection_plan(
        paths
            .iter()
            .cloned()
            .map(|path| IrSelectionItemPlan::Exact { path })
            .collect(),
        paths,
        tuple_type(2),
    );
    let result = repeated.selector(source, 0, None, Vec::new());
    let program = repeated.finish(vec![plan], result);
    assert_all_machines(&program, |value| {
        assert!(
            matches!(value, RuntimeValue::Tuple(_)),
            "重复键不能伪造字典列"
        );
        assert_eq!(dictionary_or_tuple_integers(value), vec![11, 11]);
    });

    let mut ordinary = SelectorProgramBuilder::new();
    let source = ordinary.int_dict_column(&[("name", 11), ("level", 22)]);
    let paths = vec![index_path(1)];
    let plan = selection_plan(
        vec![IrSelectionItemPlan::Exact {
            path: paths[0].clone(),
        }],
        paths,
        IrType::DictColumn {
            entries: vec![IrDictTypeEntry {
                key: "level".to_owned(),
                value: Box::new(int_type()),
            }],
        },
    );
    let result = ordinary.selector(source, 0, None, Vec::new());
    let program = ordinary.finish(vec![plan], result);
    assert_all_machines(&program, |value| {
        assert!(
            matches!(value, RuntimeValue::DictColumn(_)),
            "普通选择必须保留字典列"
        );
        assert_eq!(dictionary_or_tuple_integers(value), vec![22]);
    });

    let mut random = SelectorProgramBuilder::new();
    let source = random.int_dict_column(&[("name", 11), ("level", 22)]);
    let plan = IrSelectionPlan {
        with_replacement: true,
        items: vec![IrSelectionItemPlan::Random {
            mode: "with_replacement".to_owned(),
            count: Some(5),
            dynamic_count: false,
        }],
        result_type: tuple_type(5),
        ..selection_plan(Vec::new(), Vec::new(), tuple_type(5))
    };
    let result = random.selector(source, 0, None, vec![None]);
    let program = random.finish(vec![plan], result);
    assert_all_machines_seeded(&program, 7, |value| {
        assert!(
            matches!(value, RuntimeValue::Tuple(_)),
            "放回重复键必须返回元组"
        );
        assert_eq!(
            dictionary_or_tuple_integers(value),
            vec![11, 11, 11, 11, 22]
        );
    });
}

#[test]
/// 随机选择必须断言抽取顺序、无放回约束和放回数量，而不是只断言成功。
fn random_selection_returns_observable_values() {
    let mut builder = SelectorProgramBuilder::new();
    let source = builder.int_array(&[10, 20, 30]);
    let plan = IrSelectionPlan {
        items: vec![IrSelectionItemPlan::Random {
            mode: "without_replacement".to_owned(),
            count: Some(2),
            dynamic_count: false,
        }],
        result_type: array_type(2),
        ..selection_plan(Vec::new(), Vec::new(), array_type(2))
    };
    let result = builder.selector(source, 0, None, vec![None]);
    let program = builder.finish(vec![plan], result);
    assert_all_machines_seeded(&program, 7, |value| {
        assert_eq!(integer_elements(value), vec![20, 10]);
    });
}

#[test]
/// 零数量选择返回与来源同根类型的空容器。
fn empty_selection_preserves_root_shape() {
    let mut array = SelectorProgramBuilder::new();
    let source = array.int_array(&[1, 2]);
    let plan = selection_plan(Vec::new(), Vec::new(), array_type(0));
    let result = array.selector(source, 0, None, Vec::new());
    let program = array.finish(vec![plan], result);
    assert_all_machines(&program, |value| {
        let RuntimeValue::Array(handle) = value else {
            panic!("空数组选择根形状错误: {value:?}");
        };
        assert!(handle.is_empty());
    });

    let mut tuple = SelectorProgramBuilder::new();
    let source = tuple.int_tuple(&[1, 2]);
    let plan = selection_plan(Vec::new(), Vec::new(), tuple_type(0));
    let result = tuple.selector(source, 0, None, Vec::new());
    let program = tuple.finish(vec![plan], result);
    assert_all_machines(&program, |value| {
        let RuntimeValue::Tuple(handle) = value else {
            panic!("空元组选择根形状错误: {value:?}");
        };
        assert!(handle.is_empty());
    });

    let mut column = SelectorProgramBuilder::new();
    let source = column.int_dict_column(&[("a", 1), ("b", 2)]);
    let plan = selection_plan(
        Vec::new(),
        Vec::new(),
        IrType::DictColumn {
            entries: Vec::new(),
        },
    );
    let result = column.selector(source, 0, None, Vec::new());
    let program = column.finish(vec![plan], result);
    assert_all_machines(&program, |value| {
        let RuntimeValue::DictColumn(handle) = value else {
            panic!("空字典列选择根形状错误: {value:?}");
        };
        assert!(handle.is_empty());
    });

    let mut text = SelectorProgramBuilder::new();
    let source = text.string("abc");
    let text_type = IrType::Scalar {
        name: "str".to_owned(),
    };
    let plan = selection_plan(Vec::new(), Vec::new(), text_type);
    let result = text.selector(source, 0, None, Vec::new());
    let program = text.finish(vec![plan], result);
    assert_all_machines(&program, |value| {
        let RuntimeValue::Str(handle) = value else {
            panic!("空字符串选择根形状错误: {value:?}");
        };
        assert!(handle.is_empty());
    });

    let mut random = SelectorProgramBuilder::new();
    let source = random.int_array(&[1, 2, 3]);
    let plan = IrSelectionPlan {
        items: vec![IrSelectionItemPlan::Random {
            mode: "without_replacement".to_owned(),
            count: Some(0),
            dynamic_count: false,
        }],
        result_type: array_type(0),
        ..selection_plan(Vec::new(), Vec::new(), array_type(0))
    };
    let result = random.selector(source, 0, None, vec![None]);
    let program = random.finish(vec![plan], result);
    assert_all_machines_seeded(&program, 7, |value| {
        let RuntimeValue::Array(handle) = value else {
            panic!("零数量随机必须返回数组，实际为 {value:?}");
        };
        assert!(handle.is_empty());
    });
}
