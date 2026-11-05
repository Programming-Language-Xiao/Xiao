# B2-A0 Windows本地基线

本目录记录`d0e4a5fb9043aa6c93dce7892dbe1ef333d3b276`的实际基线，不是B2-A数值产品证据。源码Git树为`593da1c97780120fc9c8aa980e6f7e803168d1c1`；A0入口主工作区干净，测试在独立detached worktree执行。

**A0本地基线成立，下一入口为另批A1；A1尚未开始。** Windows功能与门禁已实测，Linux/macOS本地未验证。没有运行B1旧721条F10，没有添加F-7依赖或改写产品、Cargo/锁、ABI、intrinsic、诊断、测试、工作流及UseDocs/registry。

## 实际结果

| 项 | 本轮结果与原始记录 |
| --- | --- |
| B1契约 | [9/9](logs/20o_b1_imp_contract.log)：七库20入口、双路径、真实归档、导出/移除、普通包隔离与教程 |
| B1资源 | [17/17](logs/20o_b1_imp_resources.log)：最大输入/输出、预算首错、UTF-8/Base64/CSV边界 |
| VM精确点 | [唯一1 passed、0 failed、0 ignored](logs/vm-exact.log)；实际参数含`--exact --nocapture` |
| Windows原生精确点 | [唯一1 passed、0 failed、0 ignored](logs/windows-native-exact.log)；七库O0/O3、别名/字符串辅助样例、FIX-B清理与原有探测保留 |
| workspace | [Clippy](logs/workspace-clippy.log)、[workspace check](logs/workspace-check.log)、[benchmark check](logs/benchmarks-check.log)、[锁差异](logs/lock-diff.log)均退出0，所有Cargo操作带`--locked` |
| docs/usedocs/layout/覆盖率 | [实体依赖后的完整all门禁](repo-all-local-copy.json)退出0；原有覆盖率warning保留 |
| B1守卫 | [28/28](b1-guards.json)；四条路径的模拟cargo输出守卫，不能算三平台产品证据 |
| 源码与签字 | [身份](identity.json)、[六条批准](approval-audit.json)、[最终边界](boundary.json)；ABI1.18/生成器20/活动intrinsic最大32、100墓碑、cmath planned |
| 当前收口 | [closeout.json](closeout.json)汇总有效门禁；[workspace-doc-gate.json](workspace-doc-gate.json)单列本次文档改动后的门禁 |

初轮[commands.json](commands.json)含22条命令的实际argv、cwd、PID、起止时间、退出码和完整日志摘要。[environment.json](environment.json)只记录显式覆盖与MSVC白名单；[继承构建项](inherited-build-environment.json)补记构建相关变量，不保存凭据或完整宿主环境。

`bun run check`按等价子项执行：`repo-check all`、workspace/all-targets Clippy、workspace check、benchmark check，再以`cmd /c git diff --exit-code`检查两份锁文件。这样所有Git调用仍经cmd，且每条Cargo命令额外使用`--locked`。

## 隔离、工具与产物

唯一任务根为`D:\project\Xiao\.a0-baseline\20261105-2bc5a0f4c6c0`；`source/`、`cargo-target/`、`xiao-home/`和`run-temp/`分别保存源码、新构建、Xiao缓存和临时输出。Cargo下载缓存可复用，任何Rust构建产物和Xiao原生缓存均从空目录开始。

`XIAO_STDLIB_ROOT`固定到该工作树的`stdlib`，Runtime从同一源码全新release构建。[built-tools.json](built-tools.json)绑定Runtime静态库、诊断程序及文档AST适配器的绝对路径、大小和SHA-256；Runtime SHA-256为`1b11cc7b05e4c60709a2f40c79865ff20e66aa9890b980c0e063b094958cd45d`。

Rust/Cargo1.96.0、Bun1.4.0、PowerShell7.6.6、LLVM22.1.2及MSVC链接器的实际版本输出均在logs内；目标为`x86_64-pc-windows-msvc`。仓库既有jobs=4、关闭增量和RUST_TEST_THREADS=4策略未改。

[native-artifacts.json](native-artifacts.json)旁观记录原生测试目录中58个稳定文件的摘要，其中52个为可执行文件；目录名包含测试进程PID。原测试结束后自动清理二进制，本目录未归档它们。这份摘要不是逐进程执行追踪；实际执行与输出核对以同一次原生精确测试的成功标记和唯一汇总为证。

签字按既定UTF-8/LF口径复算；Windows检出的CRLF原始摘要另列，不修改草案。正文仍为`8bca48018be3bf2cf38f615d7cf35d55a0ff9745f73e3c196161abd854d286ff`，台账投影仍为`999fbfa7988e03d67444f1d51aa1cec86c2cd9f1095ef1674c9a9ba9943f76a7`。

## 失败保留与解决

最初执行器[result.json](result.json)保留`passed=false`，原因是初轮文档工具环境未齐备，不能把它重写为全绿。其余功能/构建/守卫均通过。最终有效结论另见closeout.json。

1. [初轮repo-all](logs/repo-all.log)退出2：Bun不能沿局部junction解析TypeScript的相对链接。
2. [第一次重试](repo-all-retry.json)补根node_modules后恢复解析，退出1；[原始错误](logs/repo-all-retry.log)为`A0-LAYOUT-002`，目录门禁正确拒绝工作树外的依赖链接。
3. 删除这两处任务自建junction，只实体复制同一TypeScript5.9.3；[132个文件及逐字一致清单](javascript-dependency-local-copy.json)保留来源与目标。之后相同源码、相同argv和Rust适配器的[all门禁](logs/repo-all-local-copy.log)退出0；没有修改门禁实现、阈值或产品。

[driver-invocation.json](driver-invocation.json)中的实际执行器摘要对应[run-baseline-observed.py](run-baseline-observed.py)，首次重试代码快照为[run-doc-gate-observed.py](run-doc-gate-observed.py)。[run-baseline.py](run-baseline.py)仅将新任务的JS准备改为工作树内实体复制；产品测试与门禁命令集合未变。修正后的JS准备由本次all重跑验证，没有把该修正写成再次完整运行产品。

[preflight-01](preflight-01/README.md)另保留构建前的工具发现记录：签字首次误用CRLF原始摘要，以及`link.exe /?`返回1100后停止；后者原始输出和退出码仍在。正式轮读取同一链接器文件版本信息。两者均发生在任何Cargo构建或产品测试之前。

## 只读核对与完整复跑

只核对归档，不读取当前HEAD的registry，也不把历史证据当当前产品验证：

```powershell
python -X utf8 docs/DevDocs/assets/20o-b2-a0/entry-20261105/verify-evidence.py
```

[evidence.json](evidence.json)列出本目录全部归档文件的实际字节数和SHA-256（不包含清单自身）。本目录的.gitattributes禁止换行转换，原始日志字节保持不变。只读核对还检查日志中真实的9/17/1/1汇总、原生七库O0/O3标记、28/28守卫和两次失败的具体错误文本。

完整复跑使用本机记录的Windows工具路径和TypeScript5.9.3，创建全新任务目录，明确回到被测`d0e4a5fb`源码；结果写入新任务目录，不能覆盖本归档。它不是“在未来当前HEAD上复跑”：

```powershell
$a0Repo = (Get-Location).Path
$a0Task = Join-Path (Split-Path -Parent $a0Repo) ('.a0-baseline/replay-' + [guid]::NewGuid().ToString('N'))
$a0Source = Join-Path $a0Task 'source'
New-Item -ItemType Directory -Path $a0Task | Out-Null
cmd /c git worktree add --detach $a0Source d0e4a5fb9043aa6c93dce7892dbe1ef333d3b276 2>&1 |
    Tee-Object -FilePath (Join-Path $a0Task 'worktree-add.log')
if ($LASTEXITCODE -ne 0) { throw '无法创建A0历史源码工作树' }
foreach ($a0Name in @('cargo-target', 'run-temp', 'xiao-home', 'logs')) {
    New-Item -ItemType Directory -Path (Join-Path $a0Task $a0Name) | Out-Null
}
[ordered]@{
    created_utc = [DateTime]::UtcNow.ToString('o')
    repository_root = $a0Repo
    task_root = $a0Task
    source_root = $a0Source
    head = (cmd /c git -C $a0Source rev-parse HEAD)
    tree = (cmd /c git -C $a0Source show -s --format=%T HEAD)
    branch = (cmd /c git branch --show-current)
    status_porcelain = (@(cmd /c git -C $a0Source status --porcelain=v1) -join "`n")
} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $a0Task 'bootstrap.json') -Encoding utf8NoBOM
python -X utf8 -u docs/DevDocs/assets/20o-b2-a0/entry-20261105/run-baseline.py `
    --task-root $a0Task --evidence-root (Join-Path $a0Task 'evidence')
if ($LASTEXITCODE -ne 0) { throw 'A0复跑存在失败，保留新目录中的全部原始输出' }
```

当前工作区的全路径边界检查、未跟踪文件检查及签字对象防漂移见[boundary.json](boundary.json)，不只枚举部分代码目录。Linux/macOS仍须真实平台执行；A7退出和B2-C新快照721条F10的要求没有由这份Windows基线代替，22D/22C及19/11/22历史债务未关闭。
