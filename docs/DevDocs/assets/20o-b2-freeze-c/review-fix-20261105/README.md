# F-C收口后的五项文档复核

入口与被复核提交：38d6120d。职责：同步20O总纲状态、补全104条历史绑定清单、登记收口CI、明确UseDocs覆盖并给F-B3历史句补“当时”。
本轮只改文档和来源记录；30/0、F-D未签字、ABI1.18/生成器20、cmath planned及产品未开工保持原状态。
此处记录0a93f957复核退出时点。evidence.json的current_entry_evidence指向后续前置复核更新后的当前F-C清单，historical_entry_evidence另存38d6120d的历史Git身份；历史104条始终以coverage-38d6120d.json为准。

[历史绑定清单](coverage-38d6120d.json)来自Git提交38d6120d的六份清单，构成为11、10、17、17、24、25，合计104个不同路径。
F-C单目录是25条，六份清单自身不计入104；本目录新增文件另由本次evidence.json记录，不追加到这个历史数字。
7个源码摘要、6个归档、11份许可、4个入口API不是额外加数，其中许可与API已经包含在上述清单中。

[收口CI](ci-38d6120d.json)绑定38d6120d，保存两个真实运行、七个实际作业和四份原始API响应。
[UseDocs范围](usedocs-scope.json)证明原收口相对8df只修改cmath教程，且既有usedocs入口扫描整个docs/UseDocs；本轮无差异命令也显式覆盖该目录。
既有F-C[检查记录](../entry-20261105/checks.json)保留原始命令，并补充独立的文档覆盖说明，避免把后来扩大范围的命令冒充原执行命令。

## 历史104条的只读复跑

在仓库根执行以下命令。它从Git38d6120d读取清单和内容，既校验完整条目集合，也逐字节核对SHA-256；当前目录的文件变化不影响历史复跑。
所有Git调用仍经cmd，不创建、覆盖或删除仓库文件。

```powershell
@'
import hashlib, json, pathlib, posixpath, subprocess

def git(*args):
    return subprocess.check_output(["cmd", "/c", "git", *args])

path = pathlib.Path("docs/DevDocs/assets/20o-b2-freeze-c/review-fix-20261105/coverage-38d6120d.json")
inventory = json.loads(path.read_text(encoding="utf-8"))
commit = "38d6120d01958221d48a92deff020e45397e1560"
assert inventory["source_commit"] == commit
expected = []
for manifest in inventory["manifests"]:
    raw = git("show", commit + ":" + manifest["path"])
    assert hashlib.sha256(raw).hexdigest() == manifest["sha256"], manifest["path"]
    evidence = json.loads(raw)
    assert len(evidence["files"]) == manifest["declared_files"]
    for item in evidence["files"]:
        target = posixpath.normpath(posixpath.dirname(manifest["path"]) + "/" + item["file"])
        expected.append({"manifest": manifest["path"], "path": target, "sha256": item["sha256"]})
assert expected == inventory["bindings"]
assert len(expected) == len({item["path"] for item in expected}) == 104
assert [item["declared_files"] for item in inventory["manifests"]] == [11, 10, 17, 17, 24, 25]
for item in expected:
    raw = git("show", commit + ":" + item["path"])
    assert hashlib.sha256(raw).hexdigest() == item["sha256"], item["path"]
print(json.dumps({"passed": True, "historical_bindings": len(expected), "source_commit": commit}))
'@ | python -
```

## 本次检查的边界

文档门禁为check:docs、check:usedocs和check:layout；后两项分别检查教程登记/状态与目录布局，不执行新增数值产品。
产品/配置无差异检查以38d6120d为基线，显式包含docs/UseDocs；原收口改过的cmath教程则用8df→38d6120d的变更允许清单单独核对。
检查命令、退出码、日志、历史复跑结果和摘要校验见本目录checks.json与evidence.json。
收口提交的CI成功不核销b846/59a容量取消及19/11/22历史债务，也不作为本次复核提交已获CI结论的声明。
