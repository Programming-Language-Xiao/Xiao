//! 06-A 生命周期静态闭环规格测试。

use xiao_lifetime::{
    ExitKind, GraphError, LifetimeAnalyzer, OwnershipEdge, OwnershipEdgeReason, OwnershipGraph,
    OwnershipKind, ScopeKind, StorageClass, ValueId,
};
use xiao_source::SourceFile;
use xiao_syntax::Parser;
use xiao_types::TypeChecker;

/// 解析、类型检查并执行生命周期分析。
fn analyze(source_text: &str) -> xiao_lifetime::LifetimeResult {
    let source = SourceFile::from_text(source_text);
    let parsed = Parser::new(&source).parse();
    assert!(
        parsed.diagnostics.is_empty(),
        "语法诊断: {:?}",
        parsed.diagnostics
    );
    let program = parsed.program.expect("应产生程序 AST");
    let typed = TypeChecker::check(&source, &program);
    LifetimeAnalyzer::new(&source).analyze(&program, &typed)
}

#[test]
/// 单作用域字符串绑定生成确定性强释放动作。
fn single_scope_has_normal_release_plan() {
    let result = analyze("value = \"x\"\n");
    let root = result
        .scopes
        .values()
        .find(|scope| scope.kind == ScopeKind::Program)
        .expect("程序作用域");
    let plan = result
        .release_plan(root.id, ExitKind::Normal)
        .expect("正常释放计划");
    assert_eq!(plan.actions.len(), 1);
    assert_eq!(
        plan.actions[0].kind,
        xiao_lifetime::ReleaseActionKind::Strong
    );
    assert!(
        result.is_success(),
        "生命周期诊断: {:?}",
        result.diagnostics
    );
}

#[test]
/// 分支和循环各自建立作用域，且所有退出边都有计划。
fn nested_control_flow_is_scoped() {
    let result = analyze(
        "value = \"root\"\nif true\n    branch = \"branch\"\nwhile false\n    loop_value = \"loop\"\n    break\n",
    );
    assert!(
        result
            .scopes
            .values()
            .any(|scope| scope.kind == ScopeKind::Branch)
    );
    assert!(
        result
            .scopes
            .values()
            .any(|scope| scope.kind == ScopeKind::Loop)
    );
    for scope in result.scopes.values() {
        assert!(
            result.release_plan(scope.id, ExitKind::Normal).is_some(),
            "作用域 {:?} 缺少正常计划",
            scope.id
        );
        assert!(
            result
                .release_plan(scope.id, ExitKind::DynamicCheckFailure)
                .is_some(),
            "作用域 {:?} 缺少动态失败计划",
            scope.id
        );
    }
}

#[test]
/// elif/else 分支均参与合流且保留独立的释放计划。
fn elif_and_else_have_independent_scopes() {
    let result = analyze(
        "if true\n    first = \"a\"\nelif false\n    second = \"b\"\nelse\n    third = \"c\"\n",
    );
    let branches = result
        .scopes
        .values()
        .filter(|scope| scope.kind == ScopeKind::Branch)
        .collect::<Vec<_>>();
    assert_eq!(branches.len(), 3);
    assert!(branches.iter().all(|scope| {
        result
            .release_plan(scope.id, ExitKind::Normal)
            .is_some_and(|plan| plan.actions.len() == 1)
    }));
}

#[test]
/// for/while 的 continue/break 计划互不覆盖，控制流图保留对应边。
fn loop_exits_are_distinct() {
    let result = analyze(
        "for item in [\"x\"]\n    local = \"a\"\n    continue\nwhile true\n    other = \"b\"\n    break\n",
    );
    let loops = result
        .scopes
        .values()
        .filter(|scope| scope.kind == ScopeKind::Loop)
        .collect::<Vec<_>>();
    assert_eq!(loops.len(), 2);
    assert!(loops.iter().all(|scope| {
        result.release_plan(scope.id, ExitKind::Break).is_some()
            && result.release_plan(scope.id, ExitKind::Continue).is_some()
    }));
    assert!(result.control_flow.blocks.values().any(|block| {
        block
            .successors
            .iter()
            .any(|(_, kind)| *kind == xiao_lifetime::ControlFlowEdgeKind::Break)
    }));
    assert!(result.control_flow.blocks.values().any(|block| {
        block
            .successors
            .iter()
            .any(|(_, kind)| *kind == xiao_lifetime::ControlFlowEdgeKind::Continue)
    }));
}

#[test]
/// 返回局部值时标记 Returned，并从函数返回计划中转移。
fn return_transfers_local_value() {
    let result = analyze("def make() -> str\n    value = \"x\"\n    return value\n");
    let function_scope = result
        .scopes
        .values()
        .find(|scope| scope.kind == ScopeKind::Function)
        .expect("函数作用域");
    let local = function_scope
        .values
        .iter()
        .find_map(|id| result.value(*id).filter(|value| value.name.is_some()));
    let local = local.expect("函数局部值");
    assert!(
        local
            .escapes
            .contains(&xiao_lifetime::EscapeReason::Returned)
    );
    let plan = result
        .release_plan(function_scope.id, ExitKind::Return)
        .expect("返回计划");
    assert!(plan.transferred.contains(&local.id));
    assert!(!plan.actions.iter().any(|action| action.value == local.id));
}

#[test]
/// 返回新构造的容器时只转移结果，不转移参与构造的局部绑定。
fn constructed_return_keeps_input_binding_local() {
    let result = analyze("def make()\n    value = \"x\"\n    return [value]\n");
    let function_scope = result
        .scopes
        .values()
        .find(|scope| scope.kind == ScopeKind::Function)
        .expect("函数作用域");
    let value = function_scope
        .values
        .iter()
        .find_map(|id| {
            result
                .value(*id)
                .filter(|value| value.name.as_deref() == Some("ascii:value"))
        })
        .expect("局部 value");
    let plan = result
        .release_plan(function_scope.id, ExitKind::Return)
        .expect("返回计划");
    assert!(!plan.transferred.contains(&value.id));
    assert!(plan.actions.iter().any(|action| action.value == value.id));
}

#[test]
/// 嵌套函数读取外层名称时生成闭包捕获强边。
fn nested_function_captures_outer_value() {
    let result = analyze(
        "def outer() -> str\n    value = \"x\"\n    def inner() -> str\n        return value\n    return value\n",
    );
    assert!(result.values.values().any(|value| {
        value
            .escapes
            .contains(&xiao_lifetime::EscapeReason::CapturedByClosure)
    }));
    assert!(
        result
            .strong_edges
            .iter()
            .any(|edge| edge.reason == OwnershipEdgeReason::ClosureCapture)
    );
}

#[test]
/// 同一函数内跨分支读取参数只是普通词法访问，不能误判为闭包捕获。
fn nested_blocks_do_not_create_closure_capture() {
    let result = analyze(
        "def inspect(str value) -> str\n    if true\n        copy = value\n    return value\n",
    );
    assert!(
        result
            .strong_edges
            .iter()
            .all(|edge| edge.reason != OwnershipEdgeReason::ClosureCapture)
    );
}

#[test]
/// 函数参数的堆/栈类别应遵循已解析的函数签名，而不是固定写死为栈。
fn parameter_storage_uses_function_signature() {
    let result = analyze("def inspect(str value) -> str\n    return value\n");
    let function_scope = result
        .scopes
        .values()
        .find(|scope| scope.kind == ScopeKind::Function)
        .expect("函数作用域");
    let parameter = function_scope
        .values
        .iter()
        .find_map(|id| result.value(*id).filter(|value| value.parameter))
        .expect("参数");
    assert_eq!(parameter.storage, StorageClass::HeapStrong);
}

#[test]
/// 内层函数的 return 不会截断外围函数声明之后的正常控制流。
fn nested_function_exit_stays_inside_function() {
    let result = analyze(
        "def outer() -> none\n    def inner() -> int\n        return 1\n    value = \"x\"\n",
    );
    let outer = result
        .scopes
        .values()
        .filter(|scope| scope.kind == ScopeKind::Function)
        .min_by_key(|scope| scope.depth)
        .expect("outer function scope");
    assert!(result.control_flow.blocks.values().any(|block| {
        block.scope == outer.id
            && block.successors.iter().any(|(target, kind)| {
                *kind == xiao_lifetime::ControlFlowEdgeKind::Next
                    && result
                        .control_flow
                        .block(*target)
                        .is_some_and(|target| target.scope == outer.id && target.span.is_none())
            })
    }));
}

#[test]
/// 容器绑定对其中引用的堆值生成 ContainerElement 强边。
fn container_records_owned_values() {
    let result = analyze("item = \"x\"\nvalues = [item]\n");
    let item = result
        .values
        .values()
        .find(|value| value.name.as_deref() == Some("ascii:item"))
        .expect("item");
    let values = result
        .values
        .values()
        .find(|value| value.name.as_deref() == Some("ascii:values"))
        .expect("values");
    let item_object = result
        .strong_edges
        .iter()
        .find(|edge| edge.from == item.id && edge.reason == OwnershipEdgeReason::Alias)
        .map(|edge| edge.to)
        .expect("item object");
    let container_object = result
        .strong_edges
        .iter()
        .find(|edge| edge.from == values.id && edge.reason == OwnershipEdgeReason::Alias)
        .map(|edge| edge.to)
        .expect("container object");
    assert!(result.strong_edges.iter().any(|edge| {
        edge.from == container_object
            && edge.to == item_object
            && edge.reason == OwnershipEdgeReason::ContainerElement
    }));
    let root = result
        .scopes
        .values()
        .find(|scope| scope.kind == ScopeKind::Program)
        .expect("root");
    let plan = result
        .release_plan(root.id, ExitKind::Normal)
        .expect("normal plan");
    let owner_position = plan
        .actions
        .iter()
        .position(|action| action.value == values.id)
        .expect("container release");
    let item_position = plan
        .actions
        .iter()
        .position(|action| action.value == item.id)
        .expect("element release");
    assert!(owner_position < item_position);
}

#[test]
/// 仅由静态标量组成的元组可留在栈上，含字符串时转为堆管理。
fn tuple_storage_follows_recursive_type() {
    let result = analyze("numbers = (1, true)\ntextual = (1, \"x\")\n");
    let numbers = result
        .values
        .values()
        .find(|value| value.name.as_deref() == Some("ascii:numbers"))
        .expect("numbers");
    let textual = result
        .values
        .values()
        .find(|value| value.name.as_deref() == Some("ascii:textual"))
        .expect("textual");
    assert_eq!(numbers.storage, StorageClass::Stack);
    assert_eq!(textual.storage, StorageClass::HeapStrong);
}

#[test]
/// 两个容器对象相互持有形成强引用环时必须拒绝。
fn strong_cycle_is_reported() {
    let result = analyze("a = []\nb = [a]\na[0] = b\n");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code() == xiao_lifetime::STRONG_CYCLE_CODE })
    );
}

#[test]
/// 容器直接持有自身是单节点强环，不能被普通名称自赋值规则吞掉。
fn self_referential_container_is_rejected() {
    let result = analyze("value = []\nvalue[0] = value\n");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == xiao_lifetime::STRONG_CYCLE_CODE)
    );
}

#[test]
/// 返回 bool 的比较表达式不能继承字符串操作数的临时对象。
fn scalar_expression_does_not_retain_heap_operand() {
    let result = analyze("left = \"a\"\nright = \"b\"\nsame = left == right\n");
    let same = result
        .values
        .values()
        .find(|value| value.name.as_deref() == Some("ascii:same"))
        .expect("same");
    assert_eq!(same.storage, StorageClass::Stack);
    assert!(result.strong_edges.iter().all(|edge| edge.from != same.id));
}

#[test]
/// 转为 str 会产生独立堆结果，并由目标绑定持有。
fn cast_to_string_produces_heap_result() {
    let result = analyze("flag = true\ntext = flag as str\n");
    let text = result
        .values
        .values()
        .find(|value| value.name.as_deref() == Some("ascii:text"))
        .expect("text");
    assert_eq!(text.storage, StorageClass::HeapStrong);
    assert!(result.strong_edges.iter().any(|edge| {
        edge.from == text.id
            && edge.reason == OwnershipEdgeReason::Alias
            && result.value(edge.to).is_some_and(|value| value.temporary)
    }));
}

#[test]
/// 名称重绑定共享同一对象不等同于对象相互持有，不能误报强环。
fn plain_aliases_do_not_form_object_cycles() {
    let result = analyze("a = \"a\"\nb = a\na = b\n");
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == xiao_lifetime::STRONG_CYCLE_CODE)
    );
}

#[test]
/// 动态值采用堆强拥有并留下 Runtime 检查，而不是让分析器崩溃。
fn dynamic_values_are_conservative() {
    let result = analyze("value = unknown()\n");
    assert!(!result.dynamic_checks.is_empty());
    assert!(result.values.values().any(|value| {
        value.storage == StorageClass::HeapStrong
            && value
                .escapes
                .contains(&xiao_lifetime::EscapeReason::DynamicValue)
    }));
}

#[test]
/// 动态检查失败计划释放已建立绑定，且所有计划都无重复动作。
fn failure_plans_are_idempotent() {
    let result = analyze("value = unknown()\n");
    let root = result
        .scopes
        .values()
        .find(|scope| scope.kind == ScopeKind::Program)
        .expect("root");
    let value = result
        .values
        .values()
        .find(|value| value.name.as_deref() == Some("ascii:value"))
        .expect("value");
    let failure = result
        .release_plan(root.id, ExitKind::DynamicCheckFailure)
        .expect("dynamic failure plan");
    assert!(
        failure
            .actions
            .iter()
            .any(|action| action.value == value.id)
    );
    for plan in result.release_plans.values() {
        let ids = plan
            .actions
            .iter()
            .map(|action| action.value)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), plan.actions.len());
        assert!(
            plan.actions
                .iter()
                .enumerate()
                .all(|(index, action)| action.order == index)
        );
    }
}

#[test]
/// new 构造调用在控制流和释放计划中保留构造失败边。
fn constructor_failure_has_cleanup_plan() {
    let result = analyze("[[Item]]\n    value = \"x\"\nitem = new Item()\n");
    let root = result
        .scopes
        .values()
        .find(|scope| scope.kind == ScopeKind::Program)
        .expect("root");
    assert!(
        result
            .release_plan(root.id, ExitKind::ConstructFailure)
            .is_some()
    );
    assert!(
        result
            .control_flow
            .blocks
            .values()
            .any(|block| block.exits.contains(&ExitKind::ConstructFailure))
    );
}

#[test]
/// 强图按拓扑层排序，同层按声明序号逆序；弱边不形成强环。
fn graph_orders_and_weak_edges() {
    let mut graph = OwnershipGraph::new();
    graph.add_node(ValueId::new(0), 0).expect("节点");
    graph.add_node(ValueId::new(1), 1).expect("节点");
    graph.add_node(ValueId::new(2), 2).expect("节点");
    graph
        .add_edge(OwnershipEdge::new(
            ValueId::new(0),
            ValueId::new(1),
            OwnershipKind::Strong,
            OwnershipEdgeReason::Explicit,
            None,
        ))
        .expect("强边");
    graph
        .add_edge(OwnershipEdge::weak(
            ValueId::new(1),
            ValueId::new(0),
            OwnershipEdgeReason::Explicit,
        ))
        .expect("弱边");
    assert_eq!(
        graph.release_order_all().expect("拓扑顺序"),
        vec![ValueId::new(2), ValueId::new(0), ValueId::new(1)]
    );
    assert!(graph.strong_cycle().is_none());
}

#[test]
/// 纯强环被图算法拒绝，返回错误而不是产生部分成功顺序。
fn graph_rejects_strong_cycle() {
    let mut graph = OwnershipGraph::new();
    graph.add_node(ValueId::new(0), 0).expect("node");
    graph.add_node(ValueId::new(1), 1).expect("node");
    graph
        .add_strong_edge(
            ValueId::new(0),
            ValueId::new(1),
            OwnershipEdgeReason::Explicit,
        )
        .expect("edge");
    graph
        .add_strong_edge(
            ValueId::new(1),
            ValueId::new(0),
            OwnershipEdgeReason::Explicit,
        )
        .expect("edge");
    assert_eq!(
        graph.strong_cycle(),
        Some(vec![ValueId::new(0), ValueId::new(1)])
    );
    assert!(matches!(
        graph.release_order_all(),
        Err(GraphError::StrongCycle(_))
    ));
}

#[test]
/// 损坏图输入返回结构化错误，不使用 panic。
fn malformed_graph_is_recoverable() {
    let mut graph = OwnershipGraph::new();
    let error = graph
        .add_edge(OwnershipEdge::strong(
            ValueId::new(1),
            ValueId::new(2),
            OwnershipEdgeReason::Explicit,
        ))
        .expect_err("缺失节点应报错");
    assert!(matches!(error, GraphError::MissingNode(id) if id == ValueId::new(1)));
    let duplicate = graph
        .add_node(ValueId::new(0), 0)
        .and_then(|_| graph.add_node(ValueId::new(0), 1))
        .expect_err("重复节点应报错");
    assert!(matches!(duplicate, GraphError::DuplicateNode(id) if id == ValueId::new(0)));
}
