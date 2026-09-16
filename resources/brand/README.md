# `resources/brand`

## 目录职责

存放 Xiao 的静态品牌资源，不包含可执行代码。

## 文件清单

- `XiaoLogo-Light.png` / `XiaoLogo-Light.svg`：浅色背景使用的位图和矢量资源。
- `XiaoLogo-Dark.png` / `XiaoLogo-Dark.svg`：深色背景使用的位图和矢量资源。
- `XiaoLogo-Light.ico` / `XiaoLogo-Dark.ico`：对应主题的 Windows 图标资源。

平台接线阶段应根据宿主背景选择 `Light` 或 `Dark` 变体，并在生成最终应用时
导出约定的 `XiaoLogo.ico` 文件名；本目录不负责文件关联注册。

## 使用阶段

原生应用图标接线由 10、18、19 阶段负责；本目录只保证资源路径稳定，不实现文件关联、
安装器注册或平台特定图标生成。
