# 10S Linux 原始复现日志（2026-10-07）

对应 [测试结果](10s-linux-bare-metal-results-20261007.md)，检出提交 `0ae48501bc4e1fe99635113c33fd86d94109fa91`，退出码 `101`。

```text
平台：Linux/x86_64
Rust：rustc 1.96.0 (ac68faa20 2026-05-25)
Bun：1.4.0
目标：x86_64-unknown-linux-gnu
clang：Ubuntu clang version 21.1.8 (6ubuntu1)
llvm-as：Ubuntu LLVM version 21.1.8
llc：Ubuntu LLVM version 21.1.8

== 准备 Rust 核心和 Runtime ==
    Updating crates.io index
   Compiling serde_core v1.0.229
   Compiling zmij v1.0.23
   Compiling memchr v2.8.3
   Compiling serde_json v1.0.151
   Compiling serde v1.0.229
   Compiling itoa v1.0.18
   Compiling xiao-runtime-abi v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-runtime-abi)
   Compiling xiao-i18n v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-i18n)
   Compiling xiao-diagnostics v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-diagnostics)
   Compiling xiao-syntax v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-syntax)
   Compiling xiao-intrinsics v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-intrinsics)
   Compiling xiao-types v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-types)
   Compiling xiao-lifetime v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-lifetime)
   Compiling xiao-runtime v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-runtime)
    Finished `release` profile [optimized] target(s) in 25.92s
 Downloading crates ...
  Downloaded adler2 v2.0.1
  Downloaded simd-adler32 v0.3.10
  Downloaded crc32fast v1.5.2
  Downloaded miniz_oxide v0.9.1
  Downloaded flate2 v1.1.10
   Compiling xiao-runtime-abi v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-runtime-abi)
   Compiling serde_json v1.0.151
   Compiling crc32fast v1.5.2
   Compiling adler2 v2.0.1
   Compiling simd-adler32 v0.3.10
   Compiling xiao-lock v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-lock)
   Compiling xiao-i18n v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-i18n)
   Compiling miniz_oxide v0.9.1
   Compiling xiao-diagnostics v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-diagnostics)
   Compiling xiao-syntax v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-syntax)
   Compiling flate2 v1.1.10
   Compiling xiao-modules v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-modules)
   Compiling xiao-config v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-config)
   Compiling xiao-intrinsics v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-intrinsics)
   Compiling xiao-types v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-types)
   Compiling xiao-lifetime v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-lifetime)
   Compiling xiao-ir v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-ir)
   Compiling xiao-runtime v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-runtime)
   Compiling xiao-optimizer v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-optimizer)
   Compiling xiao-bytecode v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-bytecode)
   Compiling xiao-codegen-llvm v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-codegen-llvm)
   Compiling xiao-package v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-package)
   Compiling xiao-artifacts v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-artifacts)
   Compiling xiao-vm v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-vm)
   Compiling xiao-xar v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-xar)
   Compiling xiao-driver v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-driver)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 30.99s

== 环境门控测试（显式 --ignored） ==
   Compiling xiao-runtime-abi v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-runtime-abi)
   Compiling xiao-i18n v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-i18n)
   Compiling xiao-intrinsics v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-intrinsics)
   Compiling xiao-lock v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-lock)
   Compiling xiao-diagnostics v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-diagnostics)
   Compiling xiao-syntax v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-syntax)
   Compiling xiao-types v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-types)
   Compiling xiao-modules v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-modules)
   Compiling xiao-config v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-config)
   Compiling xiao-lifetime v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-lifetime)
   Compiling xiao-ir v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-ir)
   Compiling xiao-runtime v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-runtime)
   Compiling xiao-optimizer v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-optimizer)
   Compiling xiao-codegen-llvm v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-codegen-llvm)
   Compiling xiao-bytecode v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-bytecode)
   Compiling xiao-artifacts v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-artifacts)
   Compiling xiao-vm v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-vm)
   Compiling xiao-package v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-package)
   Compiling xiao-xar v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-xar)
   Compiling xiao-driver v0.1.0 (/home/xiaocz/Projsct/Xiao/Xiao/core/rust/crates/xiao-driver)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 28.81s
     Running unittests src/lib.rs (core/rust/target/debug/deps/xiao_artifacts-63940d1f80571f28)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 21 filtered out; finished in 0.00s

     Running unittests src/lib.rs (core/rust/target/debug/deps/xiao_bytecode-978d49c4c952e091)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 26 filtered out; finished in 0.00s

     Running unittests src/bin/xiaoc-check.rs (core/rust/target/debug/deps/xiaoc_check-fcf2bb394ccaf793)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/b0_a_api.rs (core/rust/target/debug/deps/b0_a_api-ad33e87783a9a13d)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

     Running tests/fuzz_xiaoc.rs (core/rust/target/debug/deps/fuzz_xiaoc-13f04e211ef90add)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s

     Running tests/r2_tac.rs (core/rust/target/debug/deps/r2_tac-7439e910abf7a2b1)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 27 filtered out; finished in 0.00s

     Running unittests src/lib.rs (core/rust/target/debug/deps/xiao_codegen_llvm-55828139f953f4e4)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 44 filtered out; finished in 0.00s

     Running tests/15e_ci_gated.rs (core/rust/target/debug/deps/15e_ci_gated-fcf69a3b24e40730)

running 5 tests
test ci_performance_baseline_is_platform_scoped ... ok
test real_artifact_reproducibility_is_byte_comparable ... ok
test real_symbol_table_and_debug_path_evidence ... ok
test real_artifact_strip_modes_all_optimization_levels ... ok
test real_debug_activation_survives_all_optimization_levels ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.99s

     Running tests/n0_a.rs (core/rust/target/debug/deps/n0_a-e78cd36add1c523f)

running 3 tests
test optional_corrupt_llvm_is_rejected ... ok
test optional_entry_observation_tracks_runtime_branch ... ok
test optional_real_llvm_round_trip ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out; finished in 0.18s

     Running tests/n0_b_dynamic.rs (core/rust/target/debug/deps/n0_b_dynamic-42bc209213957413)

running 1 test
test optional_llvm_accepts_dynamic_table_module ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 22 filtered out; finished in 0.02s

     Running tests/n0_c_errors.rs (core/rust/target/debug/deps/n0_c_errors-4f9fe5dda3b5d5c2)

running 2 tests
test optional_stackrestore_runtime_probe ... ok
test optional_llvm_accepts_error_path_module ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.20s

     Running unittests src/lib.rs (core/rust/target/debug/deps/xiao_config-3a9c0e11baa61e53)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s

     Running tests/d05_config.rs (core/rust/target/debug/deps/d05_config-c21529ba534c9a9b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 13 filtered out; finished in 0.00s

     Running tests/spec_snapshots.rs (core/rust/target/debug/deps/spec_snapshots-809109ff008cb0a7)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

     Running unittests src/lib.rs (core/rust/target/debug/deps/xiao_diagnostics-aba55eb97060c710)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 17 filtered out; finished in 0.00s

     Running unittests src/bin/xiao-diagnostics.rs (core/rust/target/debug/deps/xiao_diagnostics-3c73d05b81b4677c)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.00s

     Running unittests src/lib.rs (core/rust/target/debug/deps/xiao_doc_coverage_rust-04b1d2d49db7e3a9)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.00s

     Running unittests src/main.rs (core/rust/target/debug/deps/xiao_doc_coverage_rust-b222cd1cf220bf2a)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (core/rust/target/debug/deps/xiao_driver-6fb3562e8d23802f)

running 1 test
test diagnostics::tests::real_terminal_session_is_environment_gated ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 58 filtered out; finished in 0.31s

     Running unittests src/protocol_main.rs (core/rust/target/debug/deps/xiao_core-acd9d27bca3c587d)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/artifact_size_regression.rs (core/rust/target/debug/deps/artifact_size_regression-d79783c3c2e0d8c0)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s

     Running tests/b0_c_driver.rs (core/rust/target/debug/deps/b0_c_driver-99562512c9a413d2)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s

     Running tests/b0_d_exit_codes.rs (core/rust/target/debug/deps/b0_d_exit_codes-7ae7afd5474910f7)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.00s

     Running tests/compatibility_matrix.rs (core/rust/target/debug/deps/compatibility_matrix-f78d2b89d96ad304)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

     Running tests/cross_platform_release.rs (core/rust/target/debug/deps/cross_platform_release-4e1da31f5583ef8e)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s

     Running tests/d17c_archive.rs (core/rust/target/debug/deps/d17c_archive-0452dcc383a29510)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.00s

     Running tests/d19a_differential.rs (core/rust/target/debug/deps/d19a_differential-e8e201846573ddd4)

running 1 test
test native_side_matches_the_vm_sides_on_every_case ... FAILED

failures:

---- native_side_matches_the_vm_sides_on_every_case stdout ----
10R-TRACE function-heap-return-error VM=["0:2:strong_release", "1:2:strong_release", "2:2:destroy", "3:1:strong_release", "4:1:destroy"] NATIVE=["0:2:strong_release", "1:2:strong_release", "2:2:destroy", "3:1:strong_release", "4:1:destroy"]
10R-TRACE function-finally-raises VM=["0:2:strong_release", "1:2:strong_release", "2:2:destroy", "3:1:strong_release", "4:1:destroy"] NATIVE=["0:2:strong_release", "1:2:strong_release", "2:2:destroy", "3:1:strong_release", "4:1:destroy"]
10R-TRACE held-tuple VM=["0:1:strong_release", "1:2:strong_release", "2:2:destroy", "3:1:strong_release", "4:1:destroy"] NATIVE=["0:1:strong_release", "1:2:strong_release", "2:2:destroy", "3:1:strong_release", "4:1:destroy"]
10R-TRACE held-dict-table VM=["0:1:strong_release", "1:2:strong_release", "2:2:destroy", "3:1:strong_release", "4:1:destroy"] NATIVE=["0:1:strong_release", "1:2:strong_release", "2:2:destroy", "3:1:strong_release", "4:1:destroy"]
10R-TRACE held-dict-column VM=["0:1:strong_release", "1:2:strong_release", "2:2:destroy", "3:1:strong_release", "4:1:destroy"] NATIVE=["0:1:strong_release", "1:2:strong_release", "2:2:destroy", "3:1:strong_release", "4:1:destroy"]
10R-TRACE held-set VM=["0:1:strong_release", "1:2:strong_release", "2:2:destroy", "3:1:strong_release", "4:1:destroy"] NATIVE=["0:1:strong_release", "1:2:strong_release", "2:2:destroy", "3:1:strong_release", "4:1:destroy"]
10R-TRACE held-table-instance VM=["0:1:strong_release", "1:1:strong_release", "2:1:strong_release", "3:1:strong_release", "4:1:strong_release", "5:1:destroy"] NATIVE=["0:1:strong_release", "1:1:strong_release", "2:1:strong_release", "3:1:destroy"]
[19A 差分] 原生未覆盖 held-table-instance：10R 账目后续：同一表实例 VM 五次 strong_release、原生三次，均仅一次 destroy；构造/字段初始化 ABI 临时引用差异待逐调用核对，本批不改释放逻辑；VM（源码）=["0:1:strong_release", "1:1:strong_release", "2:1:strong_release", "3:1:strong_release", "4:1:strong_release", "5:1:destroy"]；原生=["0:1:strong_release", "1:1:strong_release", "2:1:strong_release", "3:1:destroy"]
10R-TRACE finally-normal VM=["0:2:strong_release", "1:2:destroy", "2:1:strong_release", "3:1:destroy"] NATIVE=["0:2:strong_release", "1:2:destroy", "2:1:strong_release", "3:1:destroy"]
10R-TRACE held-string VM=["0:1:strong_release", "1:1:destroy"] NATIVE=["0:1:strong_release", "1:1:destroy"]
10R-TRACE held-array VM=["0:1:strong_release", "1:1:destroy"] NATIVE=["0:1:strong_release", "1:1:destroy"]
10R-TRACE print VM=["0:1:strong_release", "1:1:strong_release", "2:1:destroy"] NATIVE=["0:1:strong_release", "1:1:strong_release", "2:1:destroy"]
10R-TRACE caught VM=["0:2:strong_release", "1:2:strong_release", "2:2:destroy", "3:3:strong_release", "4:3:destroy", "5:4:strong_release", "6:4:destroy", "7:1:strong_release", "8:1:destroy"] NATIVE=["0:2:strong_release", "1:2:strong_release", "2:2:destroy", "3:3:strong_release", "4:3:destroy", "5:4:strong_release", "6:4:destroy", "7:1:strong_release", "8:1:destroy"]
10R-TRACE unmatched VM=["0:2:strong_release", "1:2:strong_release", "2:2:destroy", "3:1:strong_release", "4:1:destroy"] NATIVE=["0:2:strong_release", "1:2:strong_release", "2:2:destroy", "3:1:strong_release", "4:1:destroy"]
10R-TRACE function-dynamic-arithmetic VM=[] NATIVE=[]

thread 'native_side_matches_the_vm_sides_on_every_case' (569087) panicked at crates/xiao-driver/tests/d19a_differential.rs:921:5:

用例 function-heap-return-overridden：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-function-heap-return-overridden-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
用例 function-scope-isolation：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-function-scope-isolation-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
用例 function-return-heap：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-function-return-heap-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
用例 function-finally-overrides-return：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-function-finally-overrides-return-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
用例 function-finally-overrides-error：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-function-finally-overrides-error-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
用例 function-empty-return：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-function-empty-return-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
用例 overflow：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-overflow-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
用例 nested-finally-drops：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-nested-finally-drops-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
用例 function-dynamic-condition：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-function-dynamic-condition-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
用例 selector-range-downstream：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-selector-range-downstream-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
用例 selector-random-downstream：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-selector-random-downstream-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
用例 selector-all-downstream：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-selector-all-downstream-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
用例 selector-open-range-downstream：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-selector-open-range-downstream-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    native_side_matches_the_vm_sides_on_every_case

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 6 filtered out; finished in 6.27s

error: test failed, to rerun pass `-p xiao-driver --test d19a_differential`
```
