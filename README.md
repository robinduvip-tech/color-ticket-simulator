# 彩票物理开奖模拟台

基于 Rust + Vue 3 + Tauri 的福利彩票双色球、体育彩票大乐透模拟开奖项目。

## 功能

- 左侧展示近期开奖号码、一等奖注数、开奖站点和走势分布。
- 根据历史样本计算冷热号、三区分布、奇偶分布，并映射到每个开奖球的权重。
- Rust 后端模拟球的质量、摩擦系数、表面不规则度、冷热偏置和换球参数。
- Three.js 前端模拟开奖机转动、球体碰撞、落球和滚出轨道。
- 支持一键换球，改变整组球的物理参数。
- 支持自我测试，用当前球组参数对历史期号做回测，输出完全复现次数与平均命中。

## 运行

```powershell
npm.cmd install
npm.cmd run dev
```

打开 [http://127.0.0.1:1420](http://127.0.0.1:1420) 可预览前端。

运行 Tauri 桌面应用：

```powershell
npm.cmd run tauri dev
```

项目已通过 `rust-toolchain.toml` 固定到 `stable-x86_64-pc-windows-msvc`，避免 Windows GNU 工具链缺少 `dlltool.exe` 时构建失败。

## 验证

```powershell
npm.cmd run build
cargo check
```

说明：彩票开奖结果本质随机，本项目用于模拟开奖过程和校准样本分布，不用于预测真实开奖结果。
