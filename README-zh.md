# LynxOCR

> **离线 OCR 文字识别工具**

LynxOCR 是一款极速、跨平台的离线 OCR 文字识别桌面应用。基于 PaddleOCR（PP-OCR V4/V5/V6）和 ONNX Runtime — 所有处理完全在本地完成。**无需联网，数据隐私无忧。**

## 功能特性

### 文字识别（OCR）
- **PaddleOCR 模型** — 支持 PP-OCR V4、V5、V6 ONNX 模型，一键下载。
- **图片 OCR** — 拖放或文件选择器加载图片（PNG、JPG、BMP、WEBP、TIFF）进行文字识别。
- **PDF OCR** — 渲染并识别 PDF 文档中的文字。
- **截图 OCR** — 按全局快捷键（默认 `Ctrl+Shift+O`）截取任意屏幕区域，自动识别文字并复制到剪贴板。支持多显示器。

### 应用特性
- **系统托盘** — 关闭窗口最小化到系统托盘，左键恢复，右键退出。
- **单实例运行** — 只允许运行一个实例，再次启动时激活已有窗口。
- **全局快捷键** — 截图 OCR 快捷键在应用最小化或托盘状态下也可使用。
- **多语言界面** — 支持中文和英文界面切换。
- **模型管理** — 一键下载模型，带进度显示；随时切换活跃模型。

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | [Tauri v2](https://v2.tauri.app)（Rust 后端） |
| 前端 | React 19 + TypeScript + [shadcn/ui](https://ui.shadcn.com) |
| OCR 引擎 | [PaddleOCR](https://github.com/PaddlePaddle/PaddleOCR) via [paddle-ocr-rs](https://github.com/mg-chao/paddle-ocr-rs) |
| OCR 模型 | PP-OCR V4/V5/V6 ONNX（每个约 25MB） |
| 截图捕获 | [xcap](https://github.com/nicepkg/xcap)（多显示器支持） |
| PDF 渲染 | [pdfium-render](https://github.com/ajrcarey/pdfium-render) |
| 构建工具 | [Bun](https://bun.sh) + Vite |

## 快速开始

### 环境要求

- [Bun](https://bun.sh)（包管理器）
- [Rust](https://rustup.rs)（用于编译 Tauri 后端）

### 开发

```bash
# 安装依赖
bun install

# 开发模式运行
bun run tauri dev

# 构建生产版本
bun run tauri build
```

### 模型下载

OCR 模型可在应用内通过 **设置 → 模型管理 → 下载** 获取。

| 模型 | 大小 | 说明 |
|------|------|------|
| PP-OCR V4 | 约 25MB | 轻量中文文字识别 |
| PP-OCR V5 | 约 25MB | 更高精度文本检测 |
| PP-OCR V6 | 约 25MB | 最新版本，识别精度最高 |

模型存储在可配置的本地目录中，默认路径为 `{应用数据目录}/models/`。

### 模型存放路径
下载的模型会存放在应用数据目录下的 `models/` 文件夹中：

| 系统 | 模型存放路径 |
|------|-------------|
| Windows | `%APPDATA%\LynxOCR\models` |
| macOS | `~/Library/Application Support/LynxOCR/models` |
| Linux | `~/.local/share/LynxOCR/models` |

**手动下载地址**
- Gitcode:[https://gitcode.com/tabortao/LynxOCR/releases/model](https://gitcode.com/tabortao/LynxOCR/releases/model)
- Gitee:[https://gitee.com/tabortao/LynxOCR/releases/model](https://gitee.com/tabortao/LynxOCR/releases/model)
- 魔塔社区：[https://www.modelscope.cn/models/tabortao/sherpa-onnx-asr-int8/tree/master/PaddleOCR-onnx](https://www.modelscope.cn/models/tabortao/sherpa-onnx-asr-int8/tree/master/PaddleOCR-onnx)
- 蓝奏云：[https://wwbtm.lanzouu.com/b01d70renc](https://wwbtm.lanzouu.com/b01d70renc)  
密码：`fwoq`

下载后解压到上述对应系统的模型目录即可。

## 贡献

LynxOCR 正在积极开发中，欢迎提交 Issue 和 Pull Request。

## 许可证

MIT

## 鸣谢

LynxOCR 的构建得益于以下优秀开源项目：

- [PaddleOCR](https://github.com/PaddlePaddle/PaddleOCR) — 出色的多语言 OCR 工具
- [OnnxOCR](https://github.com/jingsongliujing/OnnxOCR) — 高性能 PaddleOCR ONNX 推理引擎
- [paddle-ocr-rs](https://github.com/mg-chao/paddle-ocr-rs) — PaddleOCR ONNX 推理的 Rust 绑定
- [xcap](https://github.com/nicepkg/xcap) — 跨平台屏幕捕获库
- [pdfium-render](https://github.com/ajrcarey/pdfium-render) — Rust PDF 渲染库
- [Tauri](https://tauri.app/) — 跨平台桌面应用框架
- [React](https://react.dev/) — 前端 UI 库
- [shadcn/ui](https://ui.shadcn.com/) — 精美设计的 UI 组件