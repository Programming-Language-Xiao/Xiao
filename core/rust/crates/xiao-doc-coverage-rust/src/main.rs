//! TypeScript 文档覆盖率编排器使用的 JSON 行协议入口。

use std::io::{self, Read};
use xiao_doc_coverage_rust::{ScanRequest, scan};

/// 读取一份 JSON 请求并输出一份 JSON 响应。
fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("A0-PARSER-003: unable to read request");
        std::process::exit(2);
    }
    let request: ScanRequest = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("A0-PARSER-004: {error}");
            std::process::exit(2);
        }
    };
    let response = scan(&request);
    println!(
        "{}",
        serde_json::to_string(&response).expect("serialize response")
    );
    if !response.errors.is_empty() {
        std::process::exit(1);
    }
}
