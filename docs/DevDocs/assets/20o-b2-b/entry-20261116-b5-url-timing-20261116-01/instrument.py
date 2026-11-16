"""Deterministically add diagnostic timing to the original resource test."""
import argparse
import hashlib
import json
from pathlib import Path

TEST_PATH = "core/rust/crates/xiao-driver/tests/20o_b1_imp_resources.rs"
TEST_NAME = "url_decoding_reaches_the_unicode_result_budget"
ORIGINAL_SHA256 = "facb94018bcbf434d7588da58147407bf80c531b20e5dcdfd7b3ee05e97753e9"
BEGIN_MARKER = "B5_URL_MEASURE_BEGIN"
END_MARKER = "B5_URL_MEASURE_END"


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def test_names(text):
    pending, names = False, []
    for line in text.splitlines():
        if line.strip() == "#[test]":
            pending = True
        elif pending and line.startswith("fn "):
            names.append(line[3:].split("(", 1)[0])
            pending = False
    return names


def instrument(original):
    if sha(original) != ORIGINAL_SHA256:
        raise ValueError("资源测试不是登记的原始 Git 字节")
    text = original.decode("utf-8")
    before = "    let result = FrontendVmDriver::new().run("
    after = "    let actual: String = result"
    if text.count(before) != 1 or text.count(after) != 1:
        raise ValueError("计时插入点不唯一")
    start = """    let __b5_timing = std::thread::current()
        .name()
        .is_some_and(|name| name == "url_decoding_reaches_the_unicode_result_budget");
    if __b5_timing {
        eprintln!("B5_URL_MEASURE_BEGIN pid={}", std::process::id());
    }
    let __b5_start_unix_ns = __b5_timing.then(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    });
    let __b5_started = __b5_timing.then(std::time::Instant::now);
"""
    finish = """    if let Some(started) = __b5_started {
        let elapsed_ns = started.elapsed().as_nanos();
        let end_unix_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        eprintln!(
            "B5_URL_MEASURE_END elapsed_ns={} start_unix_ns={} end_unix_ns={} pid={} success={} code={:?}",
            elapsed_ns,
            __b5_start_unix_ns.unwrap_or_default(),
            end_unix_ns,
            std::process::id(),
            result.is_success(),
            result.code()
        );
    }
"""
    patched = text.replace(before, start + before, 1).replace(after, finish + after, 1)
    if test_names(text) != test_names(patched) or len(test_names(patched)) != 17:
        raise ValueError("诊断补丁改变了17项测试集合")
    timeout = ".with_timeout(Duration::from_secs(60))"
    if text.count(timeout) != 1 or patched.count(timeout) != 1:
        raise ValueError("诊断补丁改变了60秒控制窗口")
    return patched.encode("utf-8")


def identity(original):
    patched = instrument(original)
    return {
        "path": TEST_PATH,
        "original_sha256": sha(original),
        "diagnostic_sha256": sha(patched),
        "tests": test_names(patched.decode("utf-8")),
        "timeout_seconds": 60,
        "measurement_scope": "driver invocation including request construction; not a standalone URL function",
        "begin_marker": BEGIN_MARKER,
        "end_marker": END_MARKER,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("original_test", type=Path)
    args = parser.parse_args()
    print(json.dumps(identity(args.original_test.read_bytes()), ensure_ascii=False))
