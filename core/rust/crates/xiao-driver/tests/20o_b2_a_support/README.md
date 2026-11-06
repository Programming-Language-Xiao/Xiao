# B2-A语言差分支持

工程期：20O-B2-A5。

职责：从同一完整语言fixture构建原生O0/O3，并在有界子进程核对独立stdout黄金。VM和维护精确入口复用同一语料，保留原B1和A1执行点。

边界：不写另一套数值算法，不新增ABI/intrinsic，不把本批功能样例计为A6性能证据。

replay.rs经正式.xiaoc编码及真实.xar构建后删除应用源码重放；resources.rs逐字节比对错误位置。链接期C观察器只读既有snapshot，另一个shim旁观真实cast/release或定向返回一次既有011，不新增生产ABI，不take/clear挂起槽。三种临时值路径与分配失败各跑O0/O3，原始产物独立留存。
