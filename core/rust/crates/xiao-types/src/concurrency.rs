//! 22B 并发类型能力表与 `std.concurrent` 签名夹具。
//!
//! 本模块只描述静态事实：它不创建 Runtime、线程、任务或共享对象。能力判断
//! 采用 22A-FIX 冻结的候选 A 实际类型表；签名表供前端/测试在标准库实体实现
//! 之前检查参数形状、返回类型和消费关系。

use std::collections::BTreeMap;

use crate::types::Type;

/// 跨线程边界允许的最大值参数数目。
pub const MAX_BOUNDARY_ARGUMENTS: usize = 64;

/// 并发能力的逐项静态结果。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConcurrencyCapabilities {
    /// 是否能作为普通边界值发送。
    pub send: bool,
    /// 是否能被多个线程安全读取。
    pub sync: bool,
    /// 是否为显式只读共享句柄。
    pub shared_read: bool,
    /// 是否为显式可变共享句柄；它只允许同一 Runtime 使用。
    pub shared_mut: bool,
}

impl ConcurrencyCapabilities {
    /// 返回一个完全拒绝的能力集。
    #[must_use]
    pub const fn rejected() -> Self {
        Self {
            send: false,
            sync: false,
            shared_read: false,
            shared_mut: false,
        }
    }

    /// 判断值是否能以只读方式跨 Runtime 传递。
    #[must_use]
    pub const fn cross_runtime_read(self) -> bool {
        self.send && self.sync || self.shared_read
    }
}

/// 返回类型的候选 A 能力表。
#[must_use]
pub fn concurrency_capabilities(ty: &Type) -> ConcurrencyCapabilities {
    match ty {
        // none、固定宽度数值、bool、str 均为不可变边界标量。
        Type::None | Type::Scalar(_) => ConcurrencyCapabilities {
            send: true,
            sync: true,
            shared_read: false,
            shared_mut: false,
        },
        // 只有递归安全元组接受；当前产品数组/集合/字典均是可变容器。
        Type::Tuple(items) => {
            let safe = items.iter().all(|item| {
                let capability = concurrency_capabilities(item);
                capability.send && capability.sync
            });
            ConcurrencyCapabilities {
                send: safe,
                sync: safe,
                shared_read: false,
                shared_mut: false,
            }
        }
        Type::Shared(value) => {
            let capability = concurrency_capabilities(value);
            ConcurrencyCapabilities {
                send: capability.send && capability.sync,
                sync: capability.send && capability.sync,
                shared_read: capability.send && capability.sync,
                shared_mut: false,
            }
        }
        Type::SharedMut(value) => {
            let capability = concurrency_capabilities(value);
            ConcurrencyCapabilities {
                send: false,
                sync: false,
                shared_read: false,
                shared_mut: capability.send && capability.sync,
            }
        }
        Type::Array(_)
        | Type::Set(_)
        | Type::DictTable(_)
        | Type::DictColumn(_)
        | Type::Function { .. }
        | Type::Table(_)
        | Type::Resource(_)
        | Type::Dynamic
        | Type::Variable(_)
        | Type::Thread(_)
        | Type::Task(_)
        | Type::Runtime
        | Type::Pool
        | Type::LockGuard(_)
        | Type::OrdinaryHandle => ConcurrencyCapabilities::rejected(),
    }
}

/// 判断类型是否满足候选 A 的 Send 条件。
#[must_use]
pub fn is_send(ty: &Type) -> bool {
    concurrency_capabilities(ty).send
}

/// 判断类型是否满足候选 A 的 Sync 条件。
#[must_use]
pub fn is_sync(ty: &Type) -> bool {
    concurrency_capabilities(ty).sync
}

/// `is_send` 的语义化别名，便于边界检查调用方阅读。
#[must_use]
pub fn is_sendable(ty: &Type) -> bool {
    is_send(ty)
}

/// `is_sync` 的语义化别名，便于边界检查调用方阅读。
#[must_use]
pub fn is_syncable(ty: &Type) -> bool {
    is_sync(ty)
}

/// 判断类型是否可读方式跨 Runtime。
#[must_use]
pub fn can_cross_runtime_read(ty: &Type) -> bool {
    concurrency_capabilities(ty).cross_runtime_read()
}

/// 判断类型是否能在同一 Runtime 内以共享可变句柄使用。
#[must_use]
pub fn can_share_mutably(ty: &Type) -> bool {
    concurrency_capabilities(ty).shared_mut
}

/// 边界值参数数量静态错误。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundaryArgumentError {
    /// 实际边界值参数数量。
    pub count: usize,
    /// 冻结的最大参数数量。
    pub limit: usize,
}

/// 检查一次显式边界调用的值参数数量。
pub fn check_boundary_argument_count(count: usize) -> Result<(), BoundaryArgumentError> {
    if count > MAX_BOUNDARY_ARGUMENTS {
        Err(BoundaryArgumentError {
            count,
            limit: MAX_BOUNDARY_ARGUMENTS,
        })
    } else {
        Ok(())
    }
}

/// 并发句柄的静态身份。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ConcurrentHandleKind {
    /// `thread<T>`。
    Thread,
    /// `task<T>`。
    Task,
    /// `runtime`。
    Runtime,
    /// `pool`。
    Pool,
    /// `shared<T>`。
    Shared,
    /// `shared_mut<T>`。
    SharedMut,
    /// `lock_guard<T>`。
    LockGuard,
    /// 内部普通句柄。
    Ordinary,
}

/// 会影响句柄消费状态的静态操作。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HandleOperation {
    /// `join(thread<T>)`。
    Join,
    /// `detach(thread<T>)`。
    Detach,
    /// `wait(task<T>)`。
    Wait,
    /// `cancel(thread<T>|task<T>)`，只借用句柄。
    Cancel,
    /// `runtime_close(runtime)`。
    RuntimeClose,
    /// `shutdown(pool)`。
    Shutdown,
}

/// 判断一项操作是否消费传入句柄。
#[must_use]
pub const fn operation_consumes_handle(
    kind: ConcurrentHandleKind,
    operation: HandleOperation,
) -> bool {
    matches!(
        (kind, operation),
        (ConcurrentHandleKind::Thread, HandleOperation::Join)
            | (ConcurrentHandleKind::Thread, HandleOperation::Detach)
            | (ConcurrentHandleKind::Task, HandleOperation::Wait)
            | (ConcurrentHandleKind::Runtime, HandleOperation::RuntimeClose)
            | (ConcurrentHandleKind::Pool, HandleOperation::Shutdown)
    )
}

/// 一个可供类型检查器或 lowering 使用的消费状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandleConsumption {
    /// 句柄种类。
    pub kind: ConcurrentHandleKind,
    /// 是否已经被消费。
    pub consumed: bool,
}

impl HandleConsumption {
    /// 创建尚未消费的句柄状态。
    #[must_use]
    pub const fn new(kind: ConcurrentHandleKind) -> Self {
        Self {
            kind,
            consumed: false,
        }
    }

    /// 尝试记录一次操作；重复消费返回 `false`。
    pub fn apply(&mut self, operation: HandleOperation) -> bool {
        if self.consumed && operation_consumes_handle(self.kind, operation) {
            return false;
        }
        if operation_consumes_handle(self.kind, operation) {
            self.consumed = true;
        }
        true
    }
}

/// `std.concurrent` 的签名-only 参数描述。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConcurrentSignature {
    /// 函数名。
    pub name: &'static str,
    /// 固定参数类型；边界 API 的值参数之后还可以有可变尾部。
    pub parameters: Vec<Type>,
    /// 返回类型。
    pub return_type: Type,
    /// 是否接受任意数量的边界值参数（静态上限仍为 64）。
    pub variadic_boundary_values: bool,
    /// 被操作消费的固定参数位置。
    pub consumed_parameters: Vec<usize>,
}

/// 22B 提供的 `std.concurrent` 签名表。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConcurrentSignatureTable {
    signatures: BTreeMap<&'static str, ConcurrentSignature>,
}

impl ConcurrentSignatureTable {
    /// 返回一个空表。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 插入一条签名。
    pub fn insert(&mut self, signature: ConcurrentSignature) {
        self.signatures.insert(signature.name, signature);
    }

    /// 按名称读取签名。
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ConcurrentSignature> {
        self.signatures.get(name)
    }

    /// 返回签名数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.signatures.len()
    }

    /// 判断表是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.signatures.is_empty()
    }

    /// 按稳定名称顺序遍历签名。
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &ConcurrentSignature)> {
        self.signatures.iter().map(|(name, signature)| (*name, signature))
    }
}

/// 建立冻结的 `std.concurrent` 签名表；不会导入或执行标准库实现。
#[must_use]
pub fn std_concurrent_signatures() -> ConcurrentSignatureTable {
    let mut table = ConcurrentSignatureTable::new();
    let dynamic = Type::Dynamic;
    let function = Type::Function {
        parameters: Vec::new(),
        return_type: Box::new(dynamic.clone()),
    };
    let mut add = |name, parameters, return_type, variadic_boundary_values, consumed_parameters| {
        table.insert(ConcurrentSignature {
            name,
            parameters,
            return_type,
            variadic_boundary_values,
            consumed_parameters,
        });
    };
    add("runtime_create", Vec::new(), Type::Runtime, false, Vec::new());
    add("current_runtime", Vec::new(), Type::Runtime, false, Vec::new());
    add("runtime_close", vec![Type::Runtime], Type::None, false, vec![0]);
    add(
        "spawn",
        vec![Type::Runtime, function.clone()],
        Type::thread(dynamic.clone()),
        true,
        Vec::new(),
    );
    add("join", vec![Type::thread(dynamic.clone())], dynamic.clone(), false, vec![0]);
    add("detach", vec![Type::thread(dynamic.clone())], Type::None, false, vec![0]);
    add("cancel", vec![Type::task(dynamic.clone())], Type::None, false, Vec::new());
    add("shared_read", vec![dynamic.clone()], Type::shared(dynamic.clone()), false, vec![0]);
    add(
        "shared_mut",
        vec![dynamic.clone()],
        Type::shared_mut(dynamic.clone()),
        false,
        vec![0],
    );
    add("read", vec![Type::shared(dynamic.clone())], dynamic.clone(), false, Vec::new());
    add(
        "lock",
        vec![Type::shared_mut(dynamic.clone())],
        Type::lock_guard(dynamic.clone()),
        false,
        Vec::new(),
    );
    add(
        "lock_read",
        vec![Type::lock_guard(dynamic.clone())],
        dynamic.clone(),
        false,
        Vec::new(),
    );
    add(
        "lock_write",
        vec![Type::lock_guard(dynamic.clone()), dynamic.clone()],
        Type::None,
        false,
        Vec::new(),
    );
    add("pool_create", vec![Type::Runtime, Type::scalar(xiao_syntax::ScalarType::Int)], Type::Pool, false, Vec::new());
    add(
        "submit",
        vec![Type::Pool, function],
        Type::task(dynamic.clone()),
        true,
        Vec::new(),
    );
    add("wait", vec![Type::task(dynamic.clone())], dynamic, false, vec![0]);
    add("shutdown", vec![Type::Pool], Type::None, false, vec![0]);
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use xiao_syntax::ScalarType;

    #[test]
    fn candidate_a_accepts_scalars_and_safe_tuples() {
        let safe = Type::Tuple(vec![Type::None, Type::scalar(ScalarType::Int)]);
        assert!(is_send(&safe));
        assert!(is_sync(&safe));
        assert!(!is_send(&Type::array(Type::scalar(ScalarType::Int))));
        assert!(!is_send(&Type::OrdinaryHandle));
        assert!(can_cross_runtime_read(&Type::shared(Type::scalar(ScalarType::Str))));
        assert!(!is_send(&Type::shared_mut(Type::scalar(ScalarType::Str))));
    }

    #[test]
    fn signatures_and_consumption_are_frozen() {
        let table = std_concurrent_signatures();
        assert_eq!(table.len(), 17);
        assert!(table.get("spawn").is_some_and(|signature| signature.variadic_boundary_values));
        assert_eq!(
            table.get("join").map(|signature| signature.consumed_parameters.as_slice()),
            Some([0].as_slice())
        );
        assert!(check_boundary_argument_count(MAX_BOUNDARY_ARGUMENTS).is_ok());
        assert!(check_boundary_argument_count(MAX_BOUNDARY_ARGUMENTS + 1).is_err());
        let mut state = HandleConsumption::new(ConcurrentHandleKind::Thread);
        assert!(state.apply(HandleOperation::Cancel));
        assert!(state.apply(HandleOperation::Join));
        assert!(!state.apply(HandleOperation::Detach));
    }
}
