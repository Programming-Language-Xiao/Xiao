//! B4-5证据宿主：用真实前端/字节码和三VM载体逐项核对共享pow；不实现数学。
use xiao_bytecode::{TacProgram, lower_program};
use xiao_driver::{FrontendCompiler, FrontendRequest};
use xiao_runtime::RuntimeValue;

fn program(operator: bool) -> TacProgram {
    let expression = if operator { "base ** exponent" } else { "cmath_pow(exponent = exponent, base = base)" };
    let source = format!("def probe(float base, float exponent) -> float\n    return {expression}\n");
    let ir = FrontendCompiler::new().compile(&FrontendRequest::from_text(source)).unwrap().ir;
    let mut tac = lower_program(&ir);
    let position = tac.functions.iter().position(|f| f.name == "probe").unwrap();
    tac.functions.swap(0, position);
    tac.categories = tac.functions[0].categories.clone();
    // 既有研究入口只用于注入精确binary64位模式；合法源码及公开模块执行另有门禁。
    let encoded = xiao_bytecode::encode(&tac, xiao_bytecode::OperandWidth::Leb128).unwrap();
    xiao_bytecode::decode(&encoded.bytes).unwrap()
}

fn result(value: xiao_runtime::RuntimeResult<f64>) -> String {
    value.map(|v| format!("{:016x}", v.to_bits())).unwrap_or_else(|e| e.code().to_owned())
}

fn vm(program: &TacProgram, a: f64, b: f64, carrier: usize) -> String {
    let values = [RuntimeValue::Float(a), RuntimeValue::Float(b)];
    let outcome = match carrier {
        0 => xiao_vm::run_with_values::<xiao_vm::StackCarrier>(program, Default::default(), &values),
        1 => xiao_vm::run_with_values::<xiao_vm::RegisterCarrier>(program, Default::default(), &values),
        _ => xiao_vm::run_with_values::<xiao_vm::HybridCarrier>(program, Default::default(), &values),
    };
    match outcome.value {
        Some(RuntimeValue::Float(value)) if outcome.result.is_success() => format!("{:016x}", value.to_bits()),
        None => outcome.result.error_code().expect("失败必须带错误身份").to_owned(),
        other => panic!("pow返回类型错误：{other:?}"),
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("冻结pow输入表");
    let source = std::fs::read_to_string(path).unwrap();
    let operator = program(true);
    let primitive = program(false);
    let mut checked = 0;
    for line in source.lines().filter(|s| !s.starts_with('#') && !s.is_empty()) {
        let columns: Vec<_> = line.split('\t').collect();
        assert_eq!(columns.len(), 7);
        let a = f64::from_bits(u64::from_str_radix(columns[1], 16).unwrap());
        let b = f64::from_bits(u64::from_str_radix(columns[2], 16).unwrap());
        let power = match RuntimeValue::Float(a).power(&RuntimeValue::Float(b)) {
            Ok(RuntimeValue::Float(value)) => format!("{:016x}", value.to_bits()),
            Err(e) => e.code().to_owned(),
            other => panic!("Runtime pow返回类型错误：{other:?}"),
        };
        let mut results = vec![power, result(xiao_runtime::cmath_pow(a, b))];
        for carrier in 0..3 {
            results.push(vm(&operator, a, b, carrier));
            results.push(vm(&primitive, a, b, carrier));
        }
        assert!(results.iter().all(|r| r == columns[4]), "{}: {results:?}", columns[0]);
        println!("B4-5-POW\t{}\t{}\t{}\t{}", columns[0], columns[1], columns[2], results.join("\t"));
        checked += 1;
    }
    assert_eq!(checked, 1075);
    println!("B4-5-POW-SUMMARY rows=1075 runtime_paths=2 vm_paths=6 decoded_tac=true result=passed");
}
