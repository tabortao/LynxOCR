# Plan: Markdown 渲染展示 + TXT/MD 双格式导出

## 摘要

为 OCR 文本识别结果新增 Markdown 渲染展示能力，并将导出功能从仅 TXT 扩展为 TXT + Markdown 双格式。

## 现状分析

### 当前结果展示

OCR 结果在 `src/app/ocr/page.tsx` 中展开详情区域（line 710-734）以纯文本 `<p>` 标签渲染每个 `textBlock.text`，附置信度 `Badge`。不支持任何富文本/格式化展示。

### 当前导出

* 工具栏仅一个「导出 TXT」按钮（line 525-530），调用 `handleExportAllTxt`

* `handleExportAllTxt`（line 441-465）将所有已完成 item 的文本用 `\n\n---\n\n` 拼接，写入 `{原文件名}_ocr.txt`，然后调用系统默认程序打开

* Rust 端提供通用 `write_text_file` + `open_file_with_system` 命令，无导出专属逻辑

### 关键发现

* **无 markdown 库**：`package.json` 中未安装任何 markdown 渲染/解析库

* **死 i18n key**：`ocr.batchExportDone` 已定义但从未使用

## 方案设计

### 1. 依赖安装

```bash
bun add react-markdown remark-gfm
```

| 包                | 用途                                       | 大小          |
| ---------------- | ---------------------------------------- | ----------- |
| `react-markdown` | React 组件，将 markdown 字符串渲染为 React 元素      | \~40KB gzip |
| `remark-gfm`     | GitHub Flavored Markdown 插件（表格/删除线/任务列表） | \~10KB gzip |

### 2. 结果展示模式切换

在展开详情区域（`CardContent` 内 OCR 结果栏）顶部新增一个 `ToggleGroup` 或两个 icon button 切换视图模式：

| 模式          | 渲染方式                 | 说明                     |
| ----------- | -------------------- | ---------------------- |
| 纯文本         | 当前 `<p>` 标签          | 默认，与现有一致               |
| Markdown 预览 | `<ReactMarkdown>` 组件 | 渲染 `**粗体**`、`# 标题`、表格等 |

**状态**：每个 item 独立的 `viewMode` 状态，存储在 `BatchOcrItem` 中或使用 `Map<number, "plain" | "markdown">`。

**markdown 内容构造**：将所有 `textBlocks` 的文本用 `\n\n` 连接，作为一段连续的 markdown 文本传入 `ReactMarkdown`。

### 3. 导出按钮改为下拉菜单

将工具栏中的单个「导出 TXT」按钮替换为 `DropdownMenu`：

```
[导出 ▼]
  ├── 导出 TXT (.txt)  —— 纯文本，当前行为
  └── 导出 Markdown (.md) —— 新增，带元数据头
```

**Markdown 导出格式**：

```markdown
# OCR 识别结果

> 识别时间: 2026-06-17 14:30:00
> 识别模型: PaddleOCR V6

## 文件1.png

(文本块内容，块间空行分隔)

---

## 文件2.pdf (第1页)

(文本块内容)
```

### 4. i18n 新增 key

| key                | zh          | en               |
| ------------------ | ----------- | ---------------- |
| `ocr.viewPlain`    | 纯文本         | Plain Text       |
| `ocr.viewMarkdown` | Markdown 预览 | Markdown Preview |
| `ocr.exportMd`     | 导出 Markdown | Export MD        |
| `ocr.exportAs`     | 导出          | Export           |

### 5. 文件变更清单

| 文件                                    | 变更类型 | 说明                                  |
| ------------------------------------- | ---- | ----------------------------------- |
| `package.json`                        | 修改   | 新增 `react-markdown`、`remark-gfm` 依赖 |
| `src/lib/app-context.tsx`             | 修改   | 新增 4 个 i18n key                     |
| `src/app/ocr/page.tsx`                | 修改   | 新增视图切换、导出下拉菜单、MD 导出逻辑               |
| `src/components/ui/dropdown-menu.tsx` | 已有   | 复用已有 shadcn 组件，无需修改                 |

### 6. 实现要点

**Markdown 渲染区域**：

* 使用 `prose` class（Tailwind Typography 风格）或自定义样式

* `max-h-[250px] overflow-y-auto` 保持与现有纯文本区域一致的滚动行为

* `remarkGfm` 插件启用 GFM 扩展语法

**导出文件命名**：

* TXT：`{原文件名}_ocr.txt`（不变）

* MD：`{原文件名}_ocr.md`（新增）

**图标选择**：

* 纯文本模式：`AlignLeftIcon`（lucide-react）

* Markdown 模式：`EyeIcon` 或 `FileCodeIcon`（lucide-react）

* 导出下拉：`ChevronDownIcon`（lucide-react）

## 验证步骤

1. `bun install` 安装新依赖
2. `bun run tauri build` 编译
3. 手动测试：拖入图片 → OCR → 展开结果 → 切换纯文本/Markdown 视图 → 确认渲染正常
4. 手动测试：点击导出下拉 → 导出 TXT → 打开确认内容正确
5. 手动测试：点击导出下拉 → 导出 MD → 打开确认格式正确（含元数据头）

