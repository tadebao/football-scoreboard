<!-- 项目徽章：状态/版本/平台/技术栈一行展示 -->
<p align="center">
  <a href="https://github.com/tadebao/football-scoreboard/blob/main/LICENSE"><img src="https://img.shields.io/github/license/tadebao/football-scoreboard?style=flat-square" alt="License"></a>
  &nbsp;
  <a href="https://github.com/tadebao/football-scoreboard/releases/latest"><img src="https://img.shields.io/github/v/release/tadebao/football-scoreboard?style=flat-square&color=brightgreen" alt="Latest Release"></a>
  &nbsp;
  <a href="https://github.com/tadebao/football-scoreboard/releases"><img src="https://img.shields.io/github/downloads/tadebao/football-scoreboard/total?style=flat-square" alt="Downloads"></a>
  &nbsp;
  <img src="https://img.shields.io/badge/platform-Windows%2010%2F11-0078d4?style=flat-square" alt="Platform">
  &nbsp;
  <img src="https://img.shields.io/badge/rust-stable-orange?style=flat-square&logo=rust" alt="Rust">
  &nbsp;
  <img src="https://img.shields.io/badge/UI-egui%200.29-ff69b4?style=flat-square" alt="egui">
  &nbsp;
  <img src="https://img.shields.io/badge/no%20telemetry-100%25%20local-success?style=flat-square" alt="Local only">
  &nbsp;
  <a href="https://github.com/tadebao/football-scoreboard/releases/latest"><img src="https://img.shields.io/badge/%E2%AC%87%20Download-v0.2.0-brightgreen?style=flat-square" alt="Download"></a>
</p>

<h1 align="center">⚽ 肖恩足球计分板</h1>

<p align="center">
  <b>肖恩足球计分板（Football Scoreboard）— 为业余足球赛事打造的原生桌面比分投屏工具</b><br>
  一个窗口控制，一个窗口投屏——把实时比分、比赛时间和球队信息一键投到副屏 / 投影仪，<br>
  现场零依赖、即开即用
</p>

<p align="center">
  <a href="#-下载使用">📥 下载</a> &nbsp;·&nbsp;
  <a href="#-界面预览">📸 截图</a> &nbsp;·&nbsp;
  <a href="#-核心功能">✨ 功能</a> &nbsp;·&nbsp;
  <a href="#-从源码构建">🛠️ 构建</a> &nbsp;·&nbsp;
  <a href="#-开源协议">📜 协议</a>
</p>

---

## 📸 界面预览

<p align="center">
  <b>大屏投屏效果</b><br>
  <img src="docs/screenshots/display.png?v=2" alt="肖恩足球计分板 · 大屏投屏" width="92%">
</p>

<br>

<p align="center">
  <b>控制台</b><br>
  <img src="docs/screenshots/console.png?v=2" alt="肖恩足球计分板 · 控制台" width="80%">
</p>

> 大屏以**宽屏大字体**展示比赛时间、双方队名与比分；控制台负责**比赛设置、比分管理、自动暂停等**全部控制操作。

---

## ✨ 核心功能

### 🎮 双窗口协作
- **控制台**：比分、计时、球队与赛事设置、素材上传的主控台
- **大屏**：纯展示快照，可拖到任意副屏 / 投影仪，按 `F` 一键全屏
- 两窗口通过内存快照单向同步，零竞争、零延迟

### ⏱️ 精准计时
- 基于**墙钟时间戳**（`SystemTime`）的稳定计时模型：暂停 / 继续 / 跨重启都不丢时间
- 可开关 **45 / 90 分钟自动暂停**：跨过整点自动停表，避免计时跑过
- **系统时间跳变检测**：检测到系统时间被异常修改时，顶部以**红色呼吸药丸**醒目标示，避免计时静默错乱

### ⚽ 比分与赛事管理
- 双方比分一键 `+1`；**比分归零**只清比分、**不打断计时**（用于比赛中改分）
- **整场重开**带 3 秒二次确认：停表 + 清比分 + 清阶段标志（用于换场）
- 自定义队名、赛事名、球衣色、队徽、背景图

### 🖼️ 大屏布局可视化
- 元素可**拖动 / 缩放 / 多选**调整位置
- 布局按窗口宽高**比例**存储，换分辨率 / 全屏后相对位置不变形
- 双击元素复位、`Ctrl + -/=` 缩放、`F` 全屏、`Esc` 关闭

### 🧹 整洁的界面
- 高级调试功能（手动设置任意比分、时间控制）**默认隐藏**
- 连点左上角 ⚽ **5 次** 即可解锁，调试与日常互不干扰

### 🔒 隐私与离线
- **无任何遥测 / 联网行为**，所有数据仅保存到本地 `data/` 目录
- 单 EXE 自包含，无需安装；无注册表 / 无后台服务

---

## 📥 下载使用

> ⚠️ 本项目**不提供 `dist/` 源码目录打包**，请直接下载已构建好的安装包。

- 🟢 **最新发布版**：[Release v0.2.0](https://github.com/tadebao/football-scoreboard/releases/tag/v0.2.0)
- ⬇️ **直接下载安装包**：[football-scoreboard-v0.2.0.zip](https://github.com/tadebao/football-scoreboard/releases/download/v0.2.0/football-scoreboard-v0.2.0.zip)（约 2.9 MB）

**安装步骤：**
1. 下载 `football-scoreboard-v0.2.0.zip`
2. **解压到任意目录**（建议非系统盘）
3. 双击 `football-scoreboard.exe` 启动（无控制台黑窗）
4. 在控制台填写两队名称、赛事名称，选好球衣色
5. 点「开赛」开始计时
6. 把「大屏」窗口拖到投影副屏，按 `F` 进入全屏

> 整个文件夹即用、无需安装、无需管理员权限、可拷贝给任何人。

---

## ⌨️ 大屏快捷键

| 操作 | 效果 |
|------|------|
| `F` | 全屏 / 退出全屏 |
| `Esc` | 关闭大屏窗口 |
| 单击 | 选中元素并拖动 |
| `Ctrl + 点击` | 多选元素 |
| `Ctrl + - / =` | 缩放选中元素 |
| 双击 | 复位元素到默认位置 |

---

## 🎮 控制台按钮

| 按钮 | 作用 | 计时 |
|------|------|------|
| **开赛 / 暂停 / 继续** | 控制比赛计时走停 | — |
| **`+1 进球`** | 主队或客队比分 +1 | 保持 |
| **比分归零** | 仅清比分，**不打断计时** | 保持 |
| **整场重开** | 停表 + 清比分 + 清阶段，**3 秒二次确认** | 停止 |
| **45 / 90 自动暂停** | 到节点自动提示，可临时关闭 | — |
| **保存设置** | 保存队名、颜色、素材 | — |

> 调试隐藏的「手动设置（任意比分）」「时间控制（任意时间）」：控制台左上角 ⚽ 连点 5 次解锁。

---

## 🛠️ 从源码构建

需要 **Rust 1.75+** 和 **Windows + MSVC**（egui 0.29 桌面绑定需要 Windows API）：

```bash
git clone https://github.com/tadebao/football-scoreboard.git
cd football-scoreboard
cargo build --release
```

构建产物：`target/release/football-scoreboard.exe`（约 6 MB，已剥离调试符号）。

> 想跑起来看效果？直接下载 [Release zip](#-下载使用) 更方便。

---

## 🧰 技术栈

- **语言**：[Rust](https://www.rust-lang.org/) stable
- **UI**：[egui](https://github.com/emilk/egui) 0.29 / eframe（原生即时模式 GUI）
- **窗口**：[winit](https://github.com/rust-windowing/winit) 双视口
- **图像**：`image` 0.25（PNG / JPEG / WebP / GIF）
- **持久化**：`serde` + `serde_json`（`.tmp` 写盘再 rename，防断电损坏）
- **字体**：Windows 系统黑体 + 自带 DIN Condensed Bold

---

## 📁 项目结构

```
football-scoreboard/
├─ src/
│  ├─ main.rs          # 双窗口 UI / 渲染 / 布局 / 视口（~1815 行）
│  ├─ state.rs         # 比赛状态 + 计时内核
│  └─ store.rs         # 持久化（.tmp + rename）
├─ assets/             # 图标（嵌入 exe 资源）
├─ fonts/              # 大屏数字字体
├─ Cargo.toml
├─ Cargo.lock
├─ build.rs            # Windows 资源嵌入
├─ app.rc              # 图标 / 清单
├─ LICENSE             # MIT 全文
└─ README.md           # 你正在看的
```

---

## 🤝 贡献

欢迎 Issue 与 PR：

- 🐛 **Bug 报告**：请附上操作系统版本、复现步骤与必要截图
- 💡 **功能建议**：先开 Issue 讨论设计，再开 PR
- 🌐 **翻译 / 文档**：欢迎补充

> 注：本项目按「现状」提供，作者不对因使用本软件产生的任何直接或间接损失承担责任。

---

## 📜 开源协议

本项目以 **[MIT License](LICENSE)** 开源——可自由用于**个人与商业用途**（使用、修改、分发、商用），**仅需保留版权与许可声明**。

```
MIT License

Copyright (c) 2026 BH6AEP
```

---

<p align="center">
  <i>让每一场业余比赛，都有专业级的大屏呈现。</i><br><br>
  <img src="docs/screenshots/display.png?v=2" alt="肖恩足球计分板" width="240">
</p>
