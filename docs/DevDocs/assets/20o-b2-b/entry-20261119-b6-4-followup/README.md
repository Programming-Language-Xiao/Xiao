# B6-4 原生组件修复与 Windows 22D 复跑

**B6-4 受限交接，正式结果 `b6_4_passed=false`。** 原归档为 `original_passed=false`。修复快照的功能及 Linux 工程门禁复验通过，`followup_passed=true`。Windows COFF 导出完整性未验证，`windows_coff_verified=false`。22D 间歇性释放差分未关闭，`22d_open=true`。B6-5 与 B7 沿用这组状态。[交接状态](status.json) · [原始 B6-4 证据](../entry-20261119-b6-4/README.md)

源码修复为 `9d75ab1a`。CI 在 `97d5f288` 的 Runner 分支上应用同一补丁，执行提交为 `e1191445`。两份提交的受保护源码、测试、工具和锁文件一致。

## Runtime 组件

基线 `58176ff5` 在 Linux 本机复现了 `original-pow-eq-static-o0` 的 `rc` 拒绝。LLVM 调用 `xiao_runtime_cmath_pow`。该入口通过 `write_value` 替换拥有输出槽，链接后的 ELF 保留了 `xiao_runtime_release`。121 个单入口 ELF 链接样本中，15 个 cmath 入口和 `lfloat_divide` 均保留强引用释放，表和并发入口还存在跨组件依赖。`9d75ab1a` 从 LLVM 可达的 ABI 符号出发，递归展开手工维护的 `runtime_link_dependencies` 表。产物校验独立读取最终可执行文件的符号表。复现用例修复前后的 LLVM 字节及 SHA-256 `e8efb02c1f6b14968f71fc6f25e306806fd7c40cf68c62ca21eea2d1625d6be8` 一致。

后续 Linux 原生门禁从 release Runtime 静态库枚举导出，逐符号链接，对账组件声明与实际 ELF 组件集合。新增的导出会进入同一检查。2026-11-20 用固定 `780398e5` 在 Linux x86_64、Rust 1.96.0、LLVM 21.1.8 重建 release Runtime，164 个导出通过对账。补验保存了锁定命令、环境覆盖、Runtime 和 LLVM 工具 SHA-256、构建前后固定源码身份与原始 stdout/stderr。[对账入口](../../../20o-b2-b4-3-fix2-runtime-components-plan.md#九linux-release-静态库对账) · [补验归档](local-checks.zip)

Linux 本机以独立任务根重建 release Runtime，随后完成 62 个 VM 用例、248 次 `llvm-as` 和 248 次原生观测。两种 cmath 导入、显式除法、B1 精确原生入口、已登记回调与动态字符串回归均通过。两条首跑因找不到 `rustdoc` 或 `rustc` 退出，显式引入 Rust 1.96.0 工具路径后复跑通过。WSL 工具链缺少 Clippy 组件。[本机回执与原始日志](followup.zip)

修复快照的三个原生维护作业各完成 248 次浮点比较原生观测、两种 cmath 导入 O0/O3 和既有 B1 精确入口。Linux ELF 与 macOS Mach-O 在静态幂等于及不等于的 O0/O3 产物均观察到 `rc` 和 `value`，校验等级为 `Complete`。Windows 功能通过，COFF 内部符号完整性为 `UnverifiedCoffExports`。[Linux 作业](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44541850714/job/135621969980) · [macOS 作业](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44541850714/job/135621970069) · [Windows 作业](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44541850714/job/135621969868)

同一修复快照的 Linux 默认 workspace、B1 VM 精确用例与仓库门禁通过。仓库门禁包括 fmt、workspace Clippy、`check:lock` 和文档检查。[维护运行](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44541850714) · [仓库门禁](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44543362261)

`780398e5` 提交说明中的 Windows Clippy 指本机的 `xiao-codegen-llvm --all-targets`。2026-11-20 在固定 `780398e5`、Windows x64 和 Rust 1.96.0 上重跑，退出码 0。Linux workspace Clippy 绿灯对应 `e1191445` 的 Linux 仓库门禁作业。[Windows 定向回执](local-checks.zip) · [Linux 仓库门禁](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44543362261/job/135627307737)

## Windows 22D

原 Windows Runner 三次执行均绑定 `97d5f288`，使用 `windows-2025`、Rust 1.96.0、LLVM 23.1.5。Runtime、Clang 摘要一致。第 1 次原生轨迹投影少一组 `shared_destroy`／`weak_release`，后两次完整原生作业均通过。两次重跑的 Runner 镜像由 `20261103.298.1` 换为 `20261116.315.1`，原始失败作业只保留断言投影。[尝试 1](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44464422497/job/135358661620) · [尝试 2](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44464422497/job/135613903433) · [尝试 3](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44464422497/job/135623035222)

本机 Windows 在固定 `58176ff5`、Rust 1.96.0、LLVM 22.1.2 上执行原入口 5 轮。第 1、3、5 轮通过。第 2、4 轮分别在 `xcon-bounded-string-success` 和 `spawn-join-str-multiargument` 的释放断言提前退出。另用只选择 `native-lock-wait-027` 的临时测试入口运行 25 轮，沿用源码夹具、黄金与断言。25 轮均通过，每轮 VM 和原生各保留两组共享对象销毁与弱引用释放事件。[进程与轨迹归档](followup.zip)

22D 的完整释放差分已有三个失败点。本机单夹具的 25 轮只覆盖 `native-lock-wait-027`，修复快照留有一轮三平台原生维护绿灯。B6-5 与 B7 按间歇性释放差分未关闭交接。后续复现保存失败轮的 VM 和原生完整轨迹。[22D 记录](../../../22d-llvm-concurrency-differential.md#十一后续维护回归)

## 只读复核

`followup.zip` 的 173 个路径引用 127 个 SHA-256 对象，绑定 `9d75ab1a` 和 `e1191445`。`local-checks.zip` 的 13 个路径引用 11 个对象，绑定 `780398e5`。两个归档分别核对固定 Git 对象、源码与 Runtime 摘要及原始输出，CI 回执另核对作业身份。`docs-gate.zip` 保存 `b73d975b` 的定稿门禁日志及当时的文档摘要。

`b6_4_passed` 要求原批通过、后续复验通过且 `22d_open=false`。`followup_passed` 要求 Linux 工程门禁、三平台原生功能通过，并核对 Linux ELF 和 macOS Mach-O 的完整符号观测。按开工单 §2.1 的平台口径，Windows 功能复验可接受 `UnverifiedCoffExports`，COFF 完整性由 `windows_coff_verified` 单列。审计从原作业和本机完整差分的实际失败记录计算本次 `22d_open`。[阶段验收条款](../../../20o-b2-b6-4-ci-gates-plan.md#五退出条件)

`self_negative.py` 枚举原批、工程门禁、22D、Windows 功能及三平台符号验证等级的 128 组布尔组合，核对审计派生的状态并输出 `status_matrix`。该检查使用构造的输入状态。

自反例在临时副本上删除日志对象、改写 macOS 验证等级，或伪造阶段通过和 Windows COFF 已验证。补验归档反例把导出数由 164 改成 163，并重算进程输出回执和内容寻址清单，审计以 `Linux 导出对账输出不符` 拒绝。各项核对退出码、拒绝原因与输入摘要。新检出可获取修复 Runner 对象后复跑。[状态组合与反例回执](negative-cases.json)

```powershell
cmd /c git fetch origin codex/b6-4-runtime-components-runner
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261119-b6-4-followup/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261119-b6-4-followup/self_negative.py
```

Windows 上可直接按归档内 `evidence.json` 的路径映射读取原始测试输出和 Windows Clippy 日志。

```powershell
$archive = 'docs/DevDocs/assets/20o-b2-b/entry-20261119-b6-4-followup/local-checks.zip'
$readLog = @'
import json
import sys
from zipfile import ZipFile

with ZipFile(sys.argv[1]) as archive:
    manifest = json.loads(archive.read("evidence.json"))
    for name in sys.argv[2:]:
        print(archive.read("objects/" + manifest[name]["sha256"]).decode("utf-8"), end="")
'@
python -B -X utf8 -c $readLog $archive linux/exports.stdout windows/clippy.stderr
```

Linux 原生复验入口如下。运行前设置 Rust 1.96.0、LLVM、任务 release Runtime 及全新 `XIAO_FLOAT_EQUALITY_OUTPUT`。归档中的 `linux/fixed/environment.json` 和各进程回执保存实际路径、命令、PID 与摘要。

```powershell
cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_static_float_equality -- --ignored --exact native_static_dynamic_comparisons_match_vm_o0_o3 --nocapture
cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b2_b4_cmath_library -- --ignored --exact cmath_library_matches_native_o0_o3 --nocapture
```

Windows 复验命令沿用 B6-4 原入口，完整进程与单夹具进程分别有回执。变更夹具选择的临时差异在 `windows/single-fixture.diff`。

```powershell
cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 22d_llvm_concurrency -- --ignored --exact native_concurrency_vm_native_differential_matches_from_same_ir --nocapture
```
