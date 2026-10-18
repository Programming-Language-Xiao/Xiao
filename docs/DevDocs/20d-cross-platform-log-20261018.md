# 20D Ubuntu x86_64 裸机执行日志（2026-10-18）

对应 [结果文档](20d-cross-platform-results-20261018.md)。本页收录环境、命令、退出码与关键原始输出；**完整 stdout/stderr 未删行，保存在 [原始证据 ZIP](assets/20d-linux-20261018/raw-evidence.zip)**，含 CI 原始日志。

归档校验：

```text
d279d862805fea8005526bbbbcde696a3b926799c1c801cdbfa8e421843a5389  raw-evidence.zip
```

下载解压后可在 `logs/` 目录执行 `sha256sum -c SHA256SUMS` 核对每个原始文件。仓库中的 [SHA256SUMS](assets/20d-linux-20261018/SHA256SUMS) 可核对 ZIP 本身。

## 跑前环境原始输出

```text
2026-10-18T15:15:48+08:00
commit acfcd4e965e04ab4e4927cafc9205c2048ef4642
Author:     xiaoCZX <xiaochengzhixue@outlook.com>
AuthorDate: Sun Oct 18 15:05:05 2026 +0800
Commit:     xiaoCZX <xiaochengzhixue@outlook.com>
CommitDate: Sun Oct 18 15:05:05 2026 +0800

    docs(20d): 跨平台裸机验证交接（Ubuntu AMD x64）

    目标提交 8a22c22；三平台 CI 已绿，交接的是 CI 覆盖不到的裸机维度：
    环境清单、门禁/全量/受控原生/专门夹具/复现脚本执行序列、20D-PROCESS
    逐字对账表、失败协议与 10 系列旧债执行点。明确不替代 Windows/macOS 证据。
Linux xiaoCZX 7.0.0-38-generic #38-Ubuntu SMP PREEMPT_DYNAMIC Fri Sep  4 09:10:14 UTC 2026 x86_64 GNU/Linux
Distributor ID:	Ubuntu
Description:	Ubuntu 26.04.1 LTS
Release:	26.04
Codename:	resolute
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
Architecture:                            x86_64
CPU op-mode(s):                          32-bit, 64-bit
Address sizes:                           39 bits physical, 48 bits virtual
Byte Order:                              Little Endian
CPU(s):                                  12
On-line CPU(s) list:                     0-11
Vendor ID:                               GenuineIntel
Model name:                              12th Gen Intel(R) Core(TM) i5-12450H
CPU family:                              6
Model:                                   154
Thread(s) per core:                      2
Core(s) per socket:                      8
Socket(s):                               1
Stepping:                                3
CPU(s) scaling MHz:                      81%
CPU max MHz:                             4400.0000
CPU min MHz:                             400.0000
BogoMIPS:                                4992.00
Flags:                                   fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush dts acpi mmx fxsr sse sse2 ss ht tm pbe syscall nx pdpe1gb rdtscp lm constant_tsc art arch_perfmon pebs bts rep_good nopl xtopology nonstop_tsc cpuid aperfmperf tsc_known_freq pni pclmulqdq dtes64 monitor ds_cpl vmx smx est tm2 ssse3 sdbg fma cx16 xtpr pdcm pcid sse4_1 sse4_2 x2apic movbe popcnt tsc_deadline_timer aes xsave avx f16c rdrand lahf_lm abm 3dnowprefetch cpuid_fault epb ssbd ibrs ibpb stibp ibrs_enhanced tpr_shadow flexpriority ept vpid ept_ad fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb intel_pt sha_ni xsaveopt xsavec xgetbv1 xsaves split_lock_detect user_shstk avx_vnni dtherm ida arat pln pts hwp hwp_notify hwp_act_window hwp_epp hwp_pkg_req hfi vnmi umip pku ospke waitpkg gfni vaes vpclmulqdq rdpid movdiri movdir64b fsrm md_clear serialize arch_lbr ibt flush_l1d arch_capabilities
Virtualization:                          VT-x
L1d cache:                               320 KiB (8 instances)
L1i cache:                               384 KiB (8 instances)
L2 cache:                                7 MiB (5 instances)
L3 cache:                                12 MiB (1 instance)
NUMA node(s):                            1
NUMA node0 CPU(s):                       0-11
Vulnerability Gather data sampling:      Not affected
Vulnerability Ghostwrite:                Not affected
Vulnerability Indirect target selection: Not affected
Vulnerability Itlb multihit:             Not affected
Vulnerability L1tf:                      Not affected
Vulnerability Mds:                       Not affected
Vulnerability Meltdown:                  Not affected
Vulnerability Mmio stale data:           Not affected
Vulnerability Old microcode:             Not affected
Vulnerability Reg file data sampling:    Mitigation; Clear Register File
Vulnerability Retbleed:                  Not affected
Vulnerability Spec rstack overflow:      Not affected
Vulnerability Spec store bypass:         Mitigation; Speculative Store Bypass disabled via prctl
Vulnerability Spectre v1:                Mitigation; usercopy/swapgs barriers and __user pointer sanitization
Vulnerability Spectre v2:                Mitigation; Enhanced / Automatic IBRS; IBPB conditional; PBRSB-eIBRS SW sequence; BHI BHI_DIS_S
Vulnerability Srbds:                     Not affected
Vulnerability Tsa:                       Not affected
Vulnerability Tsx async abort:           Not affected
Vulnerability Vmscape:                   Mitigation; IBPB before exit to userspace
               total        used        free      shared  buff/cache   available
Mem:            14Gi       5.9Gi       4.4Gi       2.3Gi       6.7Gi       8.9Gi
Swap:          4.0Gi          0B       4.0Gi
none
virtualization_probe_exit=1
rustc 1.96.0 (ac68faa20 2026-05-25)
binary: rustc
commit-hash: ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96
commit-date: 2026-05-25
host: x86_64-unknown-linux-gnu
release: 1.96.0
LLVM version: 22.1.2
cargo 1.96.0 (30a34c682 2026-05-25)
1.4.0
Ubuntu clang version 21.1.8 (6ubuntu1)
Target: x86_64-pc-linux-gnu
Thread model: posix
InstalledDir: /usr/lib/llvm-21/bin
Ubuntu LLVM version 21.1.8
  Optimized build.
Ubuntu LLVM version 21.1.8
  Optimized build.
  Default target: x86_64-pc-linux-gnu
  Host CPU: alderlake

DISPLAY=:0
WAYLAND_DISPLAY=wayland-0
XDG_SESSION_TYPE=wayland
/usr/bin/clang
/usr/bin/llvm-as
/usr/bin/llc
/usr/bin/llvm-strip
/usr/bin/xterm
/usr/bin/lxterm
Filesystem      Size  Used Avail Use% Mounted on
/dev/nvme0n1p2  468G  110G  334G  25% /
tmpfs           7.5G   18M  7.5G   1% /tmp
```

## 受控原生环境

```bash
export RUSTUP_TOOLCHAIN=1.96.0
export XIAO_CLANG=/usr/bin/clang
export XIAO_LLVM_AS=/usr/bin/llvm-as
export XIAO_LLC=/usr/bin/llc
export XIAO_STRIP=/usr/bin/llvm-strip
export XIAO_TARGET_TRIPLE=x86_64-unknown-linux-gnu
export XIAO_RUNTIME_LIBRARY=/home/xiaocz/Projsct/Xiao/Xiao/core/rust/target/release/libxiao_runtime.a
export XIAO_DIAGNOSTICS_PATH=/home/xiaocz/Projsct/Xiao/Xiao/core/rust/target/debug/xiao-diagnostics
export XIAO_DIAGNOSTICS_HOLD_MS=0
export XIAO_USE_XVFB=0
```

## 实际执行命令与退出码

日志由证据包中的 `capture.sh` 将命令 stdout/stderr 直接重定向到独立文件，再采集退出码；不存在 `tee` 掩盖失败的问题。

### 01-repository-gates

```text
started_at=2026-10-18T15:15:48+08:00
commit=acfcd4e965e04ab4e4927cafc9205c2048ef4642
cwd=/home/xiaocz/Projsct/Xiao/Xiao
RUSTUP_TOOLCHAIN=1.96.0
XIAO_CLANG=<unset>
XIAO_LLVM_AS=<unset>
XIAO_LLC=<unset>
XIAO_STRIP=<unset>
XIAO_TARGET_TRIPLE=<unset>
XIAO_RUNTIME_LIBRARY=<unset>
XIAO_DIAGNOSTICS_PATH=<unset>
XIAO_DIAGNOSTICS_HOLD_MS=<unset>
XIAO_USE_XVFB=<unset>
command=bash tools/gates/run.sh
finished_at=2026-10-18T15:17:41+08:00
elapsed_seconds=113
exit_code=0
```

### 02-workspace-default

```text
started_at=2026-10-18T15:18:30+08:00
commit=acfcd4e965e04ab4e4927cafc9205c2048ef4642
cwd=/home/xiaocz/Projsct/Xiao/Xiao
RUSTUP_TOOLCHAIN=1.96.0
XIAO_CLANG=<unset>
XIAO_LLVM_AS=<unset>
XIAO_LLC=<unset>
XIAO_STRIP=<unset>
XIAO_TARGET_TRIPLE=<unset>
XIAO_RUNTIME_LIBRARY=<unset>
XIAO_DIAGNOSTICS_PATH=<unset>
XIAO_DIAGNOSTICS_HOLD_MS=<unset>
XIAO_USE_XVFB=<unset>
command=cargo test --manifest-path core/rust/Cargo.toml --workspace --no-fail-fast -j 2
finished_at=2026-10-18T15:20:05+08:00
elapsed_seconds=95
exit_code=0
```

### 03a-runtime-build

```text
started_at=2026-10-18T15:23:31+08:00
commit=acfcd4e965e04ab4e4927cafc9205c2048ef4642
cwd=/home/xiaocz/Projsct/Xiao/Xiao
RUSTUP_TOOLCHAIN=1.96.0
XIAO_CLANG=/usr/bin/clang
XIAO_LLVM_AS=/usr/bin/llvm-as
XIAO_LLC=/usr/bin/llc
XIAO_STRIP=/usr/bin/llvm-strip
XIAO_TARGET_TRIPLE=x86_64-unknown-linux-gnu
XIAO_RUNTIME_LIBRARY=/home/xiaocz/Projsct/Xiao/Xiao/core/rust/target/release/libxiao_runtime.a
XIAO_DIAGNOSTICS_PATH=/home/xiaocz/Projsct/Xiao/Xiao/core/rust/target/debug/xiao-diagnostics
XIAO_DIAGNOSTICS_HOLD_MS=0
XIAO_USE_XVFB=0
command=cargo build --manifest-path core/rust/Cargo.toml --release -p xiao-runtime
finished_at=2026-10-18T15:23:42+08:00
elapsed_seconds=11
exit_code=0
```

### 03b-diagnostics-build

```text
started_at=2026-10-18T15:24:51+08:00
commit=acfcd4e965e04ab4e4927cafc9205c2048ef4642
cwd=/home/xiaocz/Projsct/Xiao/Xiao
RUSTUP_TOOLCHAIN=1.96.0
XIAO_CLANG=/usr/bin/clang
XIAO_LLVM_AS=/usr/bin/llvm-as
XIAO_LLC=/usr/bin/llc
XIAO_STRIP=/usr/bin/llvm-strip
XIAO_TARGET_TRIPLE=x86_64-unknown-linux-gnu
XIAO_RUNTIME_LIBRARY=/home/xiaocz/Projsct/Xiao/Xiao/core/rust/target/release/libxiao_runtime.a
XIAO_DIAGNOSTICS_PATH=/home/xiaocz/Projsct/Xiao/Xiao/core/rust/target/debug/xiao-diagnostics
XIAO_DIAGNOSTICS_HOLD_MS=0
XIAO_USE_XVFB=0
command=cargo build --manifest-path core/rust/Cargo.toml -p xiao-diagnostics
finished_at=2026-10-18T15:24:52+08:00
elapsed_seconds=1
exit_code=0
```

### 03c-workspace-native

```text
started_at=2026-10-18T15:27:12+08:00
commit=acfcd4e965e04ab4e4927cafc9205c2048ef4642
cwd=/home/xiaocz/Projsct/Xiao/Xiao
RUSTUP_TOOLCHAIN=1.96.0
XIAO_CLANG=/usr/bin/clang
XIAO_LLVM_AS=/usr/bin/llvm-as
XIAO_LLC=/usr/bin/llc
XIAO_STRIP=/usr/bin/llvm-strip
XIAO_TARGET_TRIPLE=x86_64-unknown-linux-gnu
XIAO_RUNTIME_LIBRARY=/home/xiaocz/Projsct/Xiao/Xiao/core/rust/target/release/libxiao_runtime.a
XIAO_DIAGNOSTICS_PATH=/home/xiaocz/Projsct/Xiao/Xiao/core/rust/target/debug/xiao-diagnostics
XIAO_DIAGNOSTICS_HOLD_MS=0
XIAO_USE_XVFB=0
command=cargo test --manifest-path core/rust/Cargo.toml --workspace --no-fail-fast -- --ignored --nocapture --skip dns_resolution_failure_is_unavailable --skip platform_failure_child
finished_at=2026-10-18T15:28:34+08:00
elapsed_seconds=82
exit_code=0
```

### 04a-first-api

```text
started_at=2026-10-18T15:29:50+08:00
commit=acfcd4e965e04ab4e4927cafc9205c2048ef4642
cwd=/home/xiaocz/Projsct/Xiao/Xiao
RUSTUP_TOOLCHAIN=1.96.0
XIAO_CLANG=/usr/bin/clang
XIAO_LLVM_AS=/usr/bin/llvm-as
XIAO_LLC=/usr/bin/llc
XIAO_STRIP=/usr/bin/llvm-strip
XIAO_TARGET_TRIPLE=x86_64-unknown-linux-gnu
XIAO_RUNTIME_LIBRARY=/home/xiaocz/Projsct/Xiao/Xiao/core/rust/target/release/libxiao_runtime.a
XIAO_DIAGNOSTICS_PATH=/home/xiaocz/Projsct/Xiao/Xiao/core/rust/target/debug/xiao-diagnostics
XIAO_DIAGNOSTICS_HOLD_MS=0
XIAO_USE_XVFB=0
command=cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20d_first_api
finished_at=2026-10-18T15:29:50+08:00
elapsed_seconds=0
exit_code=0
```

### 04b-process-api

```text
started_at=2026-10-18T15:29:50+08:00
commit=acfcd4e965e04ab4e4927cafc9205c2048ef4642
cwd=/home/xiaocz/Projsct/Xiao/Xiao
RUSTUP_TOOLCHAIN=1.96.0
XIAO_CLANG=/usr/bin/clang
XIAO_LLVM_AS=/usr/bin/llvm-as
XIAO_LLC=/usr/bin/llc
XIAO_STRIP=/usr/bin/llvm-strip
XIAO_TARGET_TRIPLE=x86_64-unknown-linux-gnu
XIAO_RUNTIME_LIBRARY=/home/xiaocz/Projsct/Xiao/Xiao/core/rust/target/release/libxiao_runtime.a
XIAO_DIAGNOSTICS_PATH=/home/xiaocz/Projsct/Xiao/Xiao/core/rust/target/debug/xiao-diagnostics
XIAO_DIAGNOSTICS_HOLD_MS=0
XIAO_USE_XVFB=0
command=cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20d_process_api
finished_at=2026-10-18T15:29:50+08:00
elapsed_seconds=0
exit_code=0
```

### 05-platform-reproduction

```text
started_at=2026-10-18T15:30:14+08:00
commit=acfcd4e965e04ab4e4927cafc9205c2048ef4642
cwd=/home/xiaocz/Projsct/Xiao/Xiao
RUSTUP_TOOLCHAIN=1.96.0
XIAO_CLANG=/usr/bin/clang
XIAO_LLVM_AS=/usr/bin/llvm-as
XIAO_LLC=/usr/bin/llc
XIAO_STRIP=/usr/bin/llvm-strip
XIAO_TARGET_TRIPLE=x86_64-unknown-linux-gnu
XIAO_RUNTIME_LIBRARY=/home/xiaocz/Projsct/Xiao/Xiao/core/rust/target/release/libxiao_runtime.a
XIAO_DIAGNOSTICS_PATH=/home/xiaocz/Projsct/Xiao/Xiao/core/rust/target/debug/xiao-diagnostics
XIAO_DIAGNOSTICS_HOLD_MS=0
XIAO_USE_XVFB=0
command=bash tools/platform-reproduction/reproduce.sh native
finished_at=2026-10-18T15:31:56+08:00
elapsed_seconds=102
exit_code=0
```

## 必跑测试关键原始输出

```text

01-repository-gates.log

 294 pass

 6 skip

 0 fail

Ran 300 tests across 38 files. [2.71s]



02-workspace-default.log

合计：1205 passed / 0 failed / 41 ignored（各测试二进制原始统计见完整日志）。



03c-workspace-native.log

test diagnostics::tests::real_terminal_session_is_environment_gated ... ok

test native_process_spawn_rejects_implicit_shell ... ok

test native_process_lifecycle_and_capability_have_no_hidden_host_effects ... ok

test native_process_api_matches_vm ... ok

test native_dynamic_resource_boundaries_match_vm ... ok

test native_table_methods_debug_execution_o0_o3 ... ok



04a-first-api.log

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/20d_first_api.rs (core/rust/target/debug/deps/20d_first_api-2f4954728ebd30ea)

running 17 tests
test filesystem_capability_bit_is_the_frozen_bit_zero ... ok
test native_os_module_matches_vm_output ... ignored, 需要受控 LLVM 工具链、Runtime staticlib 和目标三元组；准备方式见 10D §4
test resource_const_is_rejected_with_non_constant_diagnostic ... ok
test directory_cannot_be_passed_to_read_text_at_type_check ... ok
test vm_relative_path_child ... ignored, 由 vm_relative_paths_use_process_cwd 在隔离 cwd 中调用
test resource_declarations_reject_ignored_path_constraints ... ok
test table_resource_constants_use_the_non_constant_diagnostic ... ok
test resource_set_annotations_are_rejected_by_type_check ... ok
test imported_resource_interfaces_respect_scope_and_reassignment ... ok
test resource_api_rejects_keyword_and_expanded_arguments_before_execution ... ok
test imported_resource_signatures_preserve_type_boundaries ... ok
test vm_relative_paths_use_process_cwd ... ok
test imported_resource_signatures_reject_invalid_argument_shapes ... ok
test os_module_opens_reads_and_closes_file_in_vm ... ok
test os_module_requires_filesystem_capability_before_open ... ok
test os_module_runtime_error_identities_are_stable_in_vm ... ok
test imported_resource_calls_execute_in_vm_and_keep_native_boundary ... ok

test result: ok. 15 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.34s




04b-process-api.log

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/20d_process_api.rs (core/rust/target/debug/deps/20d_process_api-5d2eb9373394284a)

running 18 tests
test native_dynamic_resource_boundaries_match_vm ... ignored, 需要受控 LLVM 工具链、Runtime staticlib 和目标三元组
test native_process_api_matches_vm ... ignored, 需要受控 LLVM 工具链、Runtime staticlib 和目标三元组
test native_process_lifecycle_and_capability_have_no_hidden_host_effects ... ignored, 需要受控 LLVM 工具链、Runtime staticlib 和目标三元组
test native_process_spawn_rejects_implicit_shell ... ignored, 需要受控 LLVM 工具链、Runtime staticlib 和目标三元组
test process_vm_child ... ignored, 仅由父测试在隔离进程与 cwd 下调用
test process_container_and_constant_boundaries_are_executable ... ok
test dynamic_resource_module_imports_keep_valid_processes ... ok
test process_vm_maps_child_signal_or_exception_exit ... ok
test process_vm_wait_caches_exit_code_and_preserves_kind ... ok
test process_table_fields_preserve_resource_kind ... ok
test process_types_and_call_shapes_are_rejected_before_execution ... ok
test process_spawn_inherits_path_environment_cwd_and_standard_streams ... ok
test process_missing_capability_does_not_start_a_child ... ok
test process_close_and_drop_detach_without_waiting_or_killing ... ok
test resource_variadic_parameters_validate_each_value ... ok
test dynamic_resource_boundaries_validate_values_in_vm ... ok
test process_vm_error_identities_and_closed_alias_are_stable ... ok
test process_spawn_rejects_implicit_shell_in_vm ... ok

test result: ok. 13 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 0.23s




05-platform-reproduction.log

test diagnostics::tests::real_terminal_session_is_environment_gated ... ok

test native_table_methods_debug_execution_o0_o3 ... ok

test dns_resolution_failure_is_unavailable ... ok

test platform_failure_child ... ok

 294 pass

 6 skip

 0 fail

Ran 300 tests across 38 files. [2.57s]

调试原生产物运行结果：exit=0

/tmp/xiao-platform-reproduction.94zreF/native:       ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV), dynamically linked, interpreter /lib64/ld-linux-x86-64.so.2, for GNU/Linux 3.2.0, with debug_info, not stripped

/tmp/xiao-platform-reproduction.94zreF/package/xiao: ELF 64-bit LSB executable, x86-64, version 1 (SYSV), dynamically linked, interpreter /lib64/ld-linux-x86-64.so.2, for GNU/Linux 3.2.0, BuildID[sha1]=2c2e51f3b6fae6722da4f7b69751a2e9467ab063, not stripped

Platform reproduction completed: x86_64-unknown-linux-gnu

平台复现脚本完成：x86_64-unknown-linux-gnu



```

## 20D 原生输出（原样）

```text
20D-NATIVE relative: stdout="relative-ok\n" code=None
20D-NATIVE directory: stdout="resource<directory>\n" code=None
20D-NATIVE missing: stdout="" code=Some("X06-RUNTIME-012")
20D-NATIVE utf8: stdout="" code=Some("X06-RUNTIME-012")
20D-NATIVE closed: stdout="" code=Some("X06-RUNTIME-025")
20D-PROCESS-REVIEW declaration: stdout="" code=Some("X06-RUNTIME-002")
20D-PROCESS cached: stdout="resource<process>\n7\n7\n" code=None
20D-PROCESS-REVIEW assignment: stdout="" code=Some("X06-RUNTIME-002")
20D-PROCESS closed: stdout="" code=Some("X06-RUNTIME-025")
20D-PROCESS-REVIEW parameter: stdout="" code=Some("X06-RUNTIME-002")
20D-PROCESS missing: stdout="" code=Some("X06-RUNTIME-012")
20D-PROCESS-REVIEW return: stdout="" code=Some("X06-RUNTIME-002")
20D-PROCESS denied: stdout="" code=Some("X06-RUNTIME-026")
20D-PROCESS-REVIEW inferred_parameter: stdout="" code=Some("X06-RUNTIME-002")
20D-PROCESS signal: stdout="143\n" code=None
20D-PROCESS-REVIEW filesystem_parent: stdout="" code=Some("X06-RUNTIME-002")
20D-PROCESS-REVIEW valid: stdout="7\n" code=None
20D-PROCESS-REVIEW valid_file_parent: stdout="ok\n" code=None
20D-PROCESS-REVIEW valid_directory_parent: stdout="ok\n" code=None
```

## CI Linux 对照输出（仅去除 CI 时间/步骤前缀）

```text
20D-NATIVE relative: stdout="relative-ok\n" code=None
20D-NATIVE directory: stdout="resource<directory>\n" code=None
20D-NATIVE missing: stdout="" code=Some("X06-RUNTIME-012")
20D-NATIVE utf8: stdout="" code=Some("X06-RUNTIME-012")
20D-NATIVE closed: stdout="" code=Some("X06-RUNTIME-025")
20D-PROCESS-REVIEW declaration: stdout="" code=Some("X06-RUNTIME-002")
20D-PROCESS cached: stdout="resource<process>\n7\n7\n" code=None
20D-PROCESS-REVIEW assignment: stdout="" code=Some("X06-RUNTIME-002")
20D-PROCESS closed: stdout="" code=Some("X06-RUNTIME-025")
20D-PROCESS-REVIEW parameter: stdout="" code=Some("X06-RUNTIME-002")
20D-PROCESS missing: stdout="" code=Some("X06-RUNTIME-012")
20D-PROCESS-REVIEW return: stdout="" code=Some("X06-RUNTIME-002")
20D-PROCESS denied: stdout="" code=Some("X06-RUNTIME-026")
20D-PROCESS-REVIEW inferred_parameter: stdout="" code=Some("X06-RUNTIME-002")
20D-PROCESS signal: stdout="143\n" code=None
20D-PROCESS-REVIEW filesystem_parent: stdout="" code=Some("X06-RUNTIME-002")
20D-PROCESS-REVIEW valid: stdout="7\n" code=None
20D-PROCESS-REVIEW valid_file_parent: stdout="ok\n" code=None
20D-PROCESS-REVIEW valid_directory_parent: stdout="ok\n" code=None
```

```text
CI Linux 39104020329 / job 117605942603
local commit acfcd4e965e04ab4e4927cafc9205c2048ef4642
local_count=19
ci_count=19
line_multiset_equal=True
only_local=[]
only_ci=[]
```

## 进程表快照

原生测试前：

```text
    PID    PPID STAT COMMAND
```

原生测试后：

```text
    PID    PPID STAT COMMAND
```

## 跑后环境与产物摘要

```text
2026-10-18T15:34:06+08:00
acfcd4e965e04ab4e4927cafc9205c2048ef4642
    PID    PPID STAT COMMAND
core/rust/target/release/libxiao_runtime.a: current ar archive
core/rust/target/debug/xiao-diagnostics:    ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV), dynamically linked, interpreter /lib64/ld-linux-x86-64.so.2, for GNU/Linux 3.2.0, BuildID[sha1]=43b630371f5ac7ec19ea1ace75fc6023c2b2d8a2, with debug_info, not stripped
e91b825a034d5bd2eef8974f1dfa408cf1dcbe09f7bd94b058a3b567e352c921  core/rust/target/release/libxiao_runtime.a
23f08a9780413934dbfe860063109ca06b2d534151dbbbd135189b61418b7998  core/rust/target/debug/xiao-diagnostics
```

## 自审核：框架统计与测试执行边界

本次自审核重新读取 ZIP 中的 stdout/stderr 统计，而非使用结果摘要作为输入。
默认测试的 1205 条 `ok`、41 条 `ignored` 与各测试二进制汇总一致；CI 记录也从完整 CI 日志重新抽取，19 条仍与裸机逐字一致。
ZIP 和其中全部 27 个原始文件的 SHA-256 再次校验通过，原始归档未改写。

### Bun 原始跳过列表

```text
6 tests skipped:
(skip) 主机工具链发现 > 环境门控：使用真实 clang 完成目标探测
(skip) zsh 在安装时复用白名单并可取消激活
(skip) fish 在安装时复用白名单并可取消激活
(skip) fish 激活切换后恢复用户原函数，未激活取消无副作用
(skip) PowerShell 钩子失败时保留非零进程码，成功时不混入数字输出
(skip) 可选 MSYS Bash 在 Windows 同样按白名单激活和拒绝注入

 294 pass
 6 skip
 0 fail
 1229 expect() calls
Ran 300 tests across 38 files. [2.71s]
```

源码条件复核：`toolchain.test.ts` 的 clang 探测要求 `XIAO_TOOLCHAIN_SMOKE=1`；
`environments/activation.test.ts` 的 PowerShell 测试使用 `test.skipIf(!windows)`，MSYS 用例也仅限 Windows；
其余三项要求可用的 zsh/fish。因此不能把 294/6 全部归因于 clang 探测开关。

### 隔离子入口

以下是 §3.3 的原始框架输出；这些入口缺少父测试设置时直接返回，不能记成两条额外原生检查：

```text
test vm_relative_path_child ... ok
test process_vm_child ... ok
```

§3.5 同样包含以下直接返回的隔离入口；此处 `ok` 本身不代表触发了真实平台异常：

```text
test platform_failure_child ... ok
```

对应行为由 §3.2 的父测试在正确子进程环境下执行并断言，原始输出如下：

```text
test vm_relative_paths_use_process_cwd ... ok
test process_spawn_inherits_path_environment_cwd_and_standard_streams ... ok
test process_missing_capability_does_not_start_a_child ... ok
test process_close_and_drop_detach_without_waiting_or_killing ... ok
test platform_failure_report_is_machine_readable ... ok
```

因此 §3.3 是框架统计 39 条通过（37 条检查 + 2 个直接返回入口），§3.5 的 ignored 阶段是
41 条通过（38 条检查 + 3 个直接返回入口）。通过数和失败数不变，结果文档已补充该统计边界。
