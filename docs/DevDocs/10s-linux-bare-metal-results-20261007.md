# 10S Linux 裸机证据回传（2026-10-07）

检出提交：0ae48501bc4e1fe99635113c33fd86d94109fa91
发行版：Ubuntu 26.04.1 LTS
架构 / glibc：x86_64 / 2.43
虚拟化检测：systemd-detect-virt 返回 none（退出码 1，表示未检测到虚拟化）
rustc / Bun / clang / llvm-as：1.96.0 / 1.4.0 / 21.1.8 / 21.1.8
完整复现命令：
```bash
RUSTUP_TOOLCHAIN=1.96.0 bash tools/platform-reproduction/reproduce.sh native > /tmp/xiao-10s-linux-evidence/native.log 2>&1
```
退出码：101（直接采集脚本退出码，未被 tee 掩盖）
XIAO_USE_XVFB：未设置
会话：Wayland，DISPLAY=:0，WAYLAND_DISPLAY=wayland-0
real_terminal_session_is_environment_gated：ok，实际执行，非 ignored
窗口肉眼可见：否；操作者反馈「没见有窗口跳出来」
截图：无法提供（未看到窗口）
打包产物 file 输出：未产生；脚本在门控测试阶段失败，未到打包阶段

19.14 C 档结论：尚未验证通过。测试返回 ok 只证明测试断言通过，不能替代肉眼可见和截图证据；窗口未出现的原因尚未定位。

交接要求见 [10S Linux 裸机交接](10s-linux-bare-metal-handoff.md)。完整原始日志见 [本次复现日志](10s-linux-bare-metal-log-20261007.md)。

## 失败结果

`native_side_matches_the_vm_sides_on_every_case` 失败，d19a_differential.rs:921。
以下 13 个用例均因产物包含 IR 未登记的 weak 组件而构建失败：

- 用例 function-heap-return-overridden：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-function-heap-return-overridden-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
- 用例 function-scope-isolation：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-function-scope-isolation-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
- 用例 function-return-heap：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-function-return-heap-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
- 用例 function-finally-overrides-return：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-function-finally-overrides-return-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
- 用例 function-finally-overrides-error：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-function-finally-overrides-error-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
- 用例 function-empty-return：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-function-empty-return-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
- 用例 overflow：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-overflow-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
- 用例 nested-finally-drops：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-nested-finally-drops-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
- 用例 function-dynamic-condition：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-function-dynamic-condition-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
- 用例 selector-range-downstream：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-selector-range-downstream-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
- 用例 selector-random-downstream：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-selector-random-downstream-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
- 用例 selector-all-downstream：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-selector-all-downstream-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })
- 用例 selector-open-range-downstream：原生构建失败：Backend(ArtifactVerification { path: "/tmp/xiao-19a-native-selector-open-range-downstream-569086/program", message: "产物观察到未由 IR 登记的 Runtime 组件：weak" })

其他异常：held-table-instance 输出既有未覆盖记录，VM 五次 strong_release、原生三次，均一次 destroy。
未修改源码，未跳过失败测试，未使用 Xvfb。完整平台复现未通过，不能据此宣称 Linux 完整验收通过、三平台一致或性能达标。

## 环境原始输出

```text
0ae48501bc4e1fe99635113c33fd86d94109fa91
PRETTY_NAME="Ubuntu 26.04.1 LTS"
NAME="Ubuntu"
VERSION_ID="26.04"
VERSION="26.04.1 LTS (Resolute Raccoon)"
VERSION_CODENAME=resolute
ID=ubuntu
ID_LIKE=debian
HOME_URL="https://www.ubuntu.com/"
SUPPORT_URL="https://help.ubuntu.com/"
BUG_REPORT_URL="https://bugs.launchpad.net/ubuntu/"
PRIVACY_POLICY_URL="https://www.ubuntu.com/legal/terms-and-policies/privacy-policy"
UBUNTU_CODENAME=resolute
LOGO=ubuntu-logo
x86_64
none
ldd (Ubuntu GLIBC 2.43-2ubuntu2.4) 2.43
rustc 1.96.0 (ac68faa20 2026-05-25)
binary: rustc
commit-hash: ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96
commit-date: 2026-05-25
host: x86_64-unknown-linux-gnu
release: 1.96.0
LLVM version: 22.1.2
1.4.0
Ubuntu clang version 21.1.8 (6ubuntu1)
Ubuntu LLVM version 21.1.8
/usr/bin/clang
/usr/bin/llvm-as
/usr/bin/llc
/usr/bin/llvm-strip
/usr/bin/xterm
:0
wayland-0
wayland
```

## 日志尾部（60 行）

```text
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
