# B3-0F 联合冻结交接

B3-0A～E 五批只读审计在固定入口 `277e74c8` 上复跑，共核对 129 项前置证据。四批台账保留 12、9、6、5 项原答复，B3-0D4 条件修订另有实批。冻结表指向六模块的接口、能力、错误、取消、预算、性能和教程原件。[前置复跑](preflight.json) · [冻结表](freeze-table.json)

产品取证基线为 `e79fab3c`，ABI 1.19、生成器 22、标准库接口 1。两个 Cargo.lock 的字节在基线和当前入口一致。产品路径对账从已独立合入推送门禁修复的 `9a8f4285` 起算，净差异位于 DevDocs。三平台宿主记录和 B1 28 项守卫沿用 B3-0A 封存，其中 4 项实测、24 项派生。[三平台观察](../entry-20261122-b3-0/README.md)

交接的联合目标为 ABI 1.20、生成器 23、标准库接口 1。time 接收 ID 49～52、四个 C ABI 入口和固定时钟教程。B3-1 的编号激活与验收裁定在实施规划 §3。教程候选为 `planned`，产品级 Windows COFF 为 `pending`。B3-1 须用实际产物验证允许的 COFF 路径与产物身份绑定。[版本矩阵](../entry-20261124-b3-0d/version-matrix.json) · [time 夹具和入口](../entry-20261124-b3-0e/tutorials.json) · [B3-1 实施规划](../../../20o-b3-1-time-implementation-plan.md)

## 只读复跑

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261124-b3-0f/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261124-b3-0f/self_negative.py
```

审计从固定 Git 对象核对前置回执。自反例在临时副本核对 1 条原样通过和 5 条预期拒绝，逐项比较退出码、拒绝文案及前后文件摘要。三项文档门禁绑定受检提交的 DevDocs 和 UseDocs 树。回执分别存于 `closeout.json`、`doc-gates.json`、`self-negative-result.json`，文件摘要见 `evidence.json`。
