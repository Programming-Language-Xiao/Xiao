//! 09R2H 表对象执行、错误链和最后强引用析构的三机型回归。

use xiao_bytecode::research::{OperandWidth, TacProgram, decode, encode, lower_program};
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_runtime::RuntimeValue;
use xiao_vm::research::{RunOutcome, RunResult, VmEvent, VmOptions, run, run_hybrid, run_register};

/// 用真实前端产生表签名和释放计划，同时要求两种编码均能完整往返。
fn compile(source: &str, entry: Option<&str>) -> TacProgram {
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap_or_else(|error| panic!("前端失败：{:?}", error.diagnostics()));
    let mut program = lower_program(&artifact.ir);
    assert!(
        program.unsupported.is_empty(),
        "未降低：{:?}",
        program.unsupported
    );
    for width in [OperandWidth::Leb128, OperandWidth::FixedU16] {
        let encoded = encode(&program, width).expect("表程序应能编码");
        assert_eq!(decode(&encoded.bytes).expect("应能解码"), program);
    }
    if let Some(entry) = entry {
        program.functions[0] = program
            .functions
            .iter()
            .find(|function| function.name == entry)
            .expect("入口应存在")
            .clone();
    }
    program
}

/// 用同一个程序和参数运行三载体。
fn outcomes(program: &TacProgram, options: VmOptions) -> [RunOutcome; 3] {
    [
        run(program, options),
        run_register(program, options),
        run_hybrid(program, options),
    ]
}

/// 精确统计一个方法真正进入执行的次数。
fn calls(outcome: &RunOutcome, name: &str) -> usize {
    outcome
        .events
        .iter()
        .filter(
            |event| matches!(event, VmEvent::FunctionEntered { function, .. } if function == name),
        )
        .count()
}

#[test]
/// 默认字段先写入，再运行 init；私有字段和方法互调复用静态签名。
fn constructs_instances_and_calls_methods() {
    let program = compile(
        "[[Counter]]\n    _value = 2\n    def init(self, int start) -> none\n        self._value += start\n    def read(self) -> int\n        return self._value\n    def add(self, int amount = 3) -> int\n        self._value += amount\n        return self.read()\ndef result() -> int\n    first = new Counter(5)\n    second = new Counter(1)\n    return first.add() + second.read()\n",
        Some("result"),
    );
    for outcome in outcomes(&program, VmOptions::new()) {
        assert!(outcome.result.is_success(), "{:?}", outcome.result);
        assert_eq!(outcome.value, Some(RuntimeValue::Int(13)));
        assert_eq!(calls(&outcome, "Counter::ascii:init"), 2);
    }
}

#[test]
/// 单例在声明处初始化一次，方法可通过全局单例访问同一对象。
fn singleton_and_drop_read_only_receiver() {
    let source = "[State]\n    count = 0\n[[Item]]\n    value = 7\n    def read(self) -> int\n        return self.value\n    def drop(self) -> none\n        State.count += self.read()\ndef work() -> none\n    first = new Item()\n    alias = first\n    items = [alias]\nwork()\nif State.count != 7\n    raise ArithmeticError(code = \"WRONG_DROP\", message = \"析构计数不符\")\n";
    let program = compile(source, None);
    for outcome in outcomes(&program, VmOptions::new()) {
        assert!(outcome.result.is_success(), "{:?}", outcome.result);
        assert_eq!(calls(&outcome, "Item::ascii:drop"), 1);
        assert_eq!(calls(&outcome, "State::<fields>"), 1);
    }
}

#[test]
/// 初始化错误必须回滚，并把析构错误作为次生错误保留。
fn initialization_failure_rolls_back_and_preserves_errors() {
    let source = "[[Broken]]\n    value = 7\n    def init(self) -> none\n        raise ArithmeticError(code = \"INIT\", message = \"初始化失败\")\n    def drop(self) -> none\n        observed = self.value\n        raise ArithmeticError(code = \"DROP\", message = \"清理失败\")\nitem = new Broken()\n";
    let program = compile(source, None);
    for outcome in outcomes(&program, VmOptions::new()) {
        let RunResult::Error(ref error) = outcome.result else {
            panic!("应失败：{:?}", outcome.result);
        };
        assert_eq!(error.code(), "X06-RUNTIME-007");
        assert_eq!(error.cause().expect("初始化原因").code(), "INIT");
        assert_eq!(error.suppressed().len(), 1);
        assert_eq!(error.suppressed()[0].code(), "X06-RUNTIME-008");
        assert_eq!(
            error.suppressed()[0].cause().expect("析构原因").code(),
            "DROP"
        );
        assert_eq!(calls(&outcome, "Broken::ascii:drop"), 1);
    }
}

#[test]
/// 致命栈溢出后，宿主清理不能重新进入用户 drop。
fn fatal_does_not_execute_drop() {
    let source = "[[Item]]\n    value = 1\n    def drop(self) -> none\n        raise ArithmeticError(code = \"DROP\", message = \"不应执行\")\ndef recurse(int value) -> int\n    item = new Item()\n    return recurse(value)\nresult = recurse(1)\n";
    let program = compile(source, None);
    for outcome in outcomes(
        &program,
        VmOptions {
            max_call_depth: 8,
            ..VmOptions::default()
        },
    ) {
        assert!(
            matches!(outcome.result, RunResult::Fatal(_)),
            "{:?}",
            outcome.result
        );
        assert_eq!(calls(&outcome, "Item::ascii:drop"), 0);
    }
}

#[test]
/// 容器默认值每次创建，const 与纯转换字段先于 init 可读。
fn defaults_are_fresh_and_constants_are_visible() {
    let source = "const BASE = 4\n[[Bag]]\n    values = [BASE, 2]\n    const marker = 9\n    converted = int(3.0)\n    def init(self, int value = 6) -> none\n        self.converted = self.converted + value\ndef result() -> int\n    first = new Bag()\n    second = new Bag(value = 1)\n    first.values = [8, 2]\n    return first.values[0] + second.values[0] + first.converted + second.marker\n";
    let program = compile(source, Some("result"));
    for outcome in outcomes(&program, VmOptions::new()) {
        assert!(outcome.result.is_success(), "{:?}", outcome.result);
        assert_eq!(outcome.value, Some(RuntimeValue::Int(30)));
    }
}

#[test]
/// 返回值转移到调用者，最后的容器持有释放时才运行析构。
fn returned_instance_and_overwrite_release_exactly_once() {
    let source = "[State]\n    count = 0\n[[Item]]\n    value = 1\n    def drop(self) -> none\n        State.count = State.count + self.value\ndef make()\n    return new Item()\ndef work() -> none\n    item = make()\n    items = [item]\n    item = new Item()\nwork()\nif State.count != 2\n    raise ArithmeticError(code = \"COUNT\")\n";
    let program = compile(source, None);
    for outcome in outcomes(&program, VmOptions::new()) {
        assert!(outcome.result.is_success(), "{:?}", outcome.result);
        assert_eq!(calls(&outcome, "Item::ascii:drop"), 2);
    }
}

#[test]
/// 异常展开保持原错误，并完整收集容器中多个析构错误。
fn unwind_keeps_primary_and_nested_drop_errors() {
    let source = "[[Item]]\n    value = 1\n    def drop(self) -> none\n        raise ArithmeticError(code = \"DROP\")\ndef work() -> none\n    items = [new Item(), new Item()]\n    raise ArithmeticError(code = \"PRIMARY\")\nwork()\n";
    let program = compile(source, None);
    for outcome in outcomes(&program, VmOptions::new()) {
        let RunResult::Error(ref error) = outcome.result else {
            panic!("{:?}", outcome.result);
        };
        assert_eq!(error.code(), "PRIMARY");
        assert_eq!(error.suppressed().len(), 2, "{error:?}");
        assert!(
            error
                .suppressed()
                .iter()
                .all(|error| error.code() == "X06-RUNTIME-008"
                    && error.cause().is_some_and(|cause| cause.code() == "DROP"))
        );
        assert_eq!(calls(&outcome, "Item::ascii:drop"), 2);
    }
}

#[test]
/// 正常作用域清理失败能够被 Xiao catch 捕获，catch 之后仍可继续。
fn normal_drop_error_is_catchable() {
    let source = "[State]\n    caught = false\n[[Item]]\n    value = 1\n    def drop(self) -> none\n        raise ArithmeticError(code = \"DROP\")\ndef work() -> none\n    item = new Item()\ntry\n    work()\ncatch err as Error\n    State.caught = true\nif not State.caught\n    raise ArithmeticError(code = \"NOT_CAUGHT\")\n";
    let program = compile(source, None);
    for outcome in outcomes(&program, VmOptions::new()) {
        assert!(outcome.result.is_success(), "{:?}", outcome.result);
        assert_eq!(calls(&outcome, "Item::ascii:drop"), 1);
    }
}

#[test]
/// 析构写入自身被 Runtime 状态门拒绝，回调结束后对象仍被销毁。
fn drop_cannot_write_receiver() {
    let program = compile(
        "[[Item]]\n    value = 1\n    def drop(self) -> none\n        self.value = 2\nitem = new Item()\n",
        None,
    );
    for outcome in outcomes(&program, VmOptions::new()) {
        let RunResult::Error(ref error) = outcome.result else {
            panic!("{:?}", outcome.result);
        };
        assert_eq!(error.code(), "X06-RUNTIME-008");
        assert_eq!(
            error.cause().expect("状态错误").code(),
            xiao_runtime::TABLE_STATE_CODE
        );
        assert_eq!(calls(&outcome, "Item::ascii:drop"), 1);
    }
}

#[test]
/// 析构递归构造也使用全局调用深度，不能绕过最大深度限制。
fn recursive_drop_obeys_call_depth() {
    let program = compile(
        "[[Item]]\n    value = 1\n    def drop(self) -> none\n        other = new Item()\nitem = new Item()\n",
        None,
    );
    for outcome in outcomes(
        &program,
        VmOptions {
            max_call_depth: 8,
            ..VmOptions::default()
        },
    ) {
        assert!(
            matches!(outcome.result, RunResult::Fatal(_)),
            "{:?}",
            outcome.result
        );
        assert_eq!(outcome.metrics.max_call_depth, 8);
        assert!(calls(&outcome, "Item::ascii:drop") > 1);
    }
}

#[test]
/// 单例声明也填充 init 缺省参数，声明前的访问报告未执行状态。
fn singleton_defaults_and_declaration_order() {
    let program = compile(
        "[Config]\n    value = 0\n    def init(self, int value = 9) -> none\n        self.value = value\nif Config.value != 9\n    raise ArithmeticError(code = \"DEFAULT\")\n",
        None,
    );
    for outcome in outcomes(&program, VmOptions::new()) {
        assert!(outcome.result.is_success(), "{:?}", outcome.result);
        assert_eq!(calls(&outcome, "Config::ascii:init"), 1);
    }
    let program = compile(
        "[Config]\n    value = 1\ndef read() -> int\n    return Config.value\n",
        Some("read"),
    );
    for outcome in outcomes(&program, VmOptions::new()) {
        let RunResult::Error(error) = outcome.result else {
            panic!("应拒绝未执行的声明")
        };
        assert_eq!(error.code(), xiao_runtime::INVALID_VALUE_CODE);
        assert!(error.message().contains("单例声明尚未执行"));
    }
}

#[test]
/// 无初始化器的字段不伪造默认值，读取必须产生结构化错误。
fn uninitialized_field_read_is_rejected() {
    let program = compile(
        "[[Item]]\n    int value\nitem = new Item()\nread = item.value\n",
        None,
    );
    for outcome in outcomes(&program, VmOptions::new()) {
        let RunResult::Error(error) = outcome.result else {
            panic!("应拒绝未初始化读取")
        };
        assert_eq!(error.code(), xiao_runtime::INVALID_VALUE_CODE);
        assert!(error.message().contains("尚未初始化"));
    }
}

#[test]
/// 表方法只在顶层函数之后追加编号，损坏的表引用与旧布局必须拒绝。
fn stable_function_ids_and_table_encoding_guards() {
    use xiao_bytecode::research::{EncodeError, FuncId, TacOp};
    let program = compile(
        "def first() -> int\n    return 1\n[[Item]]\n    value = 1\n    def read(self) -> int\n        return self.value\ndef second() -> int\n    return 2\nitem = new Item()\n",
        None,
    );
    assert_eq!(program.functions[1].name, "first");
    assert_eq!(program.functions[2].name, "second");
    assert_eq!(program.table_definitions[0].fields, FuncId::new(3));
    assert_eq!(
        program.table_definitions[0].methods["ascii:read"],
        FuncId::new(4)
    );
    for width in [OperandWidth::Leb128, OperandWidth::FixedU16] {
        let mut bytes = encode(&program, width).unwrap().bytes;
        bytes[4] = 2;
        assert!(matches!(
            decode(&bytes),
            Err(EncodeError::UnsupportedFormatVersion {
                expected: 3,
                actual: 2
            })
        ));
        let mut bad = program.clone();
        bad.table_definitions[0].fields = FuncId::new(u32::MAX);
        assert!(encode(&bad, width).is_err());
        let mut bad = program.clone();
        bad.table_definitions[0].methods.clear();
        assert!(encode(&bad, width).is_err());
        let mut bad = program.clone();
        bad.table_definitions.push(bad.table_definitions[0].clone());
        assert!(encode(&bad, width).is_err());
        let mut bad = program.clone();
        let instruction = bad.functions[0]
            .blocks
            .iter_mut()
            .flat_map(|block| &mut block.instructions)
            .find(|instruction| matches!(instruction.op, TacOp::LoadTable { .. }))
            .unwrap();
        if let TacOp::LoadTable { table, .. } = &mut instruction.op {
            *table = 99;
        }
        assert!(encode(&bad, width).is_err());
        let mut bad = program.clone();
        let instruction = bad.functions[0]
            .blocks
            .iter_mut()
            .flat_map(|block| &mut block.instructions)
            .find(|instruction| matches!(instruction.op, TacOp::LoadTable { .. }))
            .unwrap();
        if let TacOp::LoadTable { construct, .. } = &mut instruction.op {
            *construct = false;
        }
        assert!(encode(&bad, width).is_err());
    }
}
