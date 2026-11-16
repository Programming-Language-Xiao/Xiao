# B5 重建基线与版本矩阵实施

星崽于2026-11-16明确批准方案2：从固定B1源码106b99c7和固定工具链重建1.18产物，标为重建基线。历史二进制原件未归档的缺口保留；本目录不把重建解释为找回原件，原B5-0预检目录、B4-5/FIX.1证据均保持只读。

产品入口cfa79381，ABI1.18/生成器21/接口1；目标ABI1.19、生成器仍21、接口1。先建立B1的1.18/20重建基线和B4的1.18/21内部基线，再执行新版本、双向拒绝、两组缓存重生成与C调用方兼容。所有产物只作内部验证，公共cmath/UseDocs/registry保持planned；B6、三平台最终发布、性能和721后置。

工具固定Rust1.96.0，Cargo使用原锁及--locked。每个来源独立任务根、source/target/Home/TEMP/缓存，受控环境清除XIAO_/RUST_BIGDECIMAL_。回执绑定命令、PID、时间、源码/锁、Runtime及工具摘要；所有原始字节以SHA-256单对象存储，用途/媒体类型单独记录，同内容的日志与ZIP不重复落盘。最终文档定稿后再单次采集docs门禁。

本次基线字段固定为baseline_kind=reconstructed_from_B1_source、historical_originals_available=false；旧源码产生后再冻结的xiaoc/xar/原生元数据不手改头或放宽版本。签署冻结正文和历史成绩不改写。
