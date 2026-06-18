# LynxOCR API 使用教程

LynxOCR 内置了 RESTful HTTP API 服务，允许其他应用程序通过网络调用 OCR 文字识别功能。本教程将详细介绍 API 的配置、认证、请求格式和各类使用场景。

## 目录

- [1. 启动 API 服务](#1-启动-api-服务)
- [2. API 端点概览](#2-api-端点概览)
- [3. 认证方式](#3-认证方式)
- [4. OCR 识别 — 三种输入方式](#4-ocr-识别--三种输入方式)
  - [4.1 本地图片上传 (multipart/form-data)](#41-本地图片上传-multipartform-data)
  - [4.2 Base64 编码图片 (JSON)](#42-base64-编码图片-json)
  - [4.3 图床/在线图片链接 (JSON)](#43-图床在线图片链接-json)
- [5. 模型切换](#5-模型切换)
- [6. 健康检查与信息查询](#6-健康检查与信息查询)
- [7. 错误处理](#7-错误处理)
- [8. 编程语言集成示例](#8-编程语言集成示例)
- [9. 常见问题](#9-常见问题)

---

## 1. 启动 API 服务

### 方式一：通过应用界面启动

1. 打开 LynxOCR 应用
2. 在左侧导航栏点击「API 服务」
3. 配置端口号（默认 9720）和 API 密钥（可选）
4. 点击「启动服务」按钮

### 方式二：开机自动启动

在 API 服务设置页面中，打开「开机自动启动服务」开关，保存配置后，每次启动 LynxOCR 时 API 服务会自动运行。

### 方式三：通过配置文件

编辑 `%APPDATA%\LynxOCR\config.json`（Windows）或 `~/.config/LynxOCR/config.json`（Linux/macOS）：

```json
{
  "apiServerPort": 9720,
  "apiKey": "your-secret-key",
  "apiServerAutoStart": true,
  "maxFileSizeMb": 20
}
```

### 验证服务是否启动

```bash
curl http://localhost:9720/api/v1/health
```

返回 `{"status":"ok","model_loaded":true,...}` 表示服务正常运行。

---

## 2. API 端点概览

| 方法   | 路径              | 说明               | 需要认证 |
|--------|-------------------|--------------------|----------|
| `POST` | `/api/v1/ocr`     | 图片 OCR 文字识别  | 可选     |
| `GET`  | `/api/v1/health`  | 健康检查 / 服务状态 | 否       |
| `GET`  | `/api/v1/info`    | 服务信息           | 可选     |

---

## 3. 认证方式

如果配置了 API 密钥，除 `/api/v1/health` 外的所有接口都需要在请求头中携带 `Authorization: Bearer <密钥>`。

### 不带认证（密钥为空时）

```bash
curl -X POST http://localhost:9720/api/v1/ocr \
  -F "image=@/path/to/image.png"
```

### 带认证（密钥为 `my-secret-key` 时）

```bash
curl -X POST http://localhost:9720/api/v1/ocr \
  -H "Authorization: Bearer my-secret-key" \
  -F "image=@/path/to/image.png"
```

认证失败时返回：

```json
{
  "success": false,
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Invalid or missing API key"
  }
}
```

---

## 4. OCR 识别 — 三种输入方式

LynxOCR API 支持三种图片输入方式，可根据场景灵活选择。

### 4.1 本地图片上传 (multipart/form-data)

适用于直接上传本地图片文件。支持 PNG、JPEG、BMP、WebP、TIFF 等常见格式。

**请求示例：**

```bash
curl -X POST http://localhost:9720/api/v1/ocr \
  -F "image=@/path/to/screenshot.png"

curl -X POST http://localhost:9720/api/v1/ocr \
  -F "image=@D:\LynxOCR\docs\demo.png"
```

**带认证的请求：**

```bash
curl -X POST http://localhost:9720/api/v1/ocr \
  -H "Authorization: Bearer my-secret-key" \
  -F "image=@/path/to/screenshot.png"
```

**成功响应：**

```json
{
  "success": true,
  "data": {
    "text": "这是识别到的全部文字内容\n第二行文字",
    "regions": [
      {
        "text": "这是识别到的全部文字内容",
        "confidence": 0.987,
        "bbox": [[10, 20], [200, 20], [200, 45], [10, 45]]
      },
      {
        "text": "第二行文字",
        "confidence": 0.965,
        "bbox": [[10, 50], [150, 50], [150, 75], [10, 75]]
      }
    ],
    "total_time_ms": 156
  },
  "model": "ppocr-v6"
}
```

**响应字段说明：**

| 字段                         | 类型     | 说明                                      |
|------------------------------|----------|-------------------------------------------|
| `success`                    | boolean  | 请求是否成功                              |
| `data.text`                  | string   | 所有识别区域的文字拼接结果                |
| `data.regions`               | array    | 每个识别区域的详细信息                    |
| `data.regions[].text`        | string   | 该区域的文字内容                          |
| `data.regions[].confidence`  | number   | 识别置信度 (0.0 ~ 1.0)                    |
| `data.regions[].bbox`        | array    | 四点坐标 [[x1,y1],[x2,y2],[x3,y3],[x4,y4]] |
| `data.total_time_ms`         | number   | 识别总耗时（毫秒）                        |
| `model`                      | string   | 使用的模型版本                            |

### 4.2 Base64 编码图片 (JSON)

适用于无法直接上传文件或需要将图片嵌入 JSON 请求体的场景。注意：Base64 数据不应包含 `data:image/...;base64,` 前缀。

**请求示例：**

```bash
# Linux/macOS
curl -X POST http://localhost:9720/api/v1/ocr \
  -H "Content-Type: application/json" \
  -d '{"image": "'$(base64 -w 0 /path/to/image.png)'"}'

# Windows PowerShell
$base64 = [Convert]::ToBase64String([IO.File]::ReadAllBytes("C:\path\to\image.png"))
$body = @{image=$base64} | ConvertTo-Json
Invoke-RestMethod -Uri "http://localhost:9720/api/v1/ocr" -Method POST -Body $body -ContentType "application/json"
```

**最简单的手动编码方式：**

```bash
# 生成 base64 字符串
base64 -w 0 image.png > image_base64.txt

# 发送请求
curl -X POST http://localhost:9720/api/v1/ocr \
  -H "Content-Type: application/json" \
  -d "{\"image\": \"$(cat image_base64.txt)\"}"
```

**响应格式与 multipart 方式完全相同。**

### 4.3 图床/在线图片链接 (JSON)

适用于需要识别网络上的图片（如 CDN、图床、对象存储中的图片）的场景。只需提供图片的 HTTP/HTTPS URL，LynxOCR 会自动下载并识别。

**注意：** `url` 和 `image` 字段互斥，每次请求只能使用其中一个。

**请求示例：**

```bash
curl -X POST http://localhost:9720/api/v1/ocr \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com/image.png"}'
```

**支持图床示例：**

```bash
# GitHub 图片
curl -X POST http://localhost:9720/api/v1/ocr \
  -H "Content-Type: application/json" \
  -d '{"url": "https://raw.githubusercontent.com/user/repo/main/screenshot.png"}'

# 自建图床（如 Chevereto、Lsky Pro 等）
curl -X POST http://localhost:9720/api/v1/ocr \
  -H "Content-Type: application/json" \
  -d '{"url": "https://img.example.com/2024/06/screenshot.png"}'

# 阿里云 OSS / 腾讯云 COS
curl -X POST http://localhost:9720/api/v1/ocr \
  -H "Content-Type: application/json" \
  -d '{"url": "https://bucket.oss-cn-hangzhou.aliyuncs.com/image.png"}'

# 带认证的图床链接
curl -X POST http://localhost:9720/api/v1/ocr \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer my-secret-key" \
  -d '{"url": "https://private-cdn.example.com/screenshot.png"}'
```

**限制说明：**

- 下载超时时间为 30 秒
- 下载的图片大小受 `maxFileSizeMb` 配置限制（默认 20MB）
- 服务器会检查响应的 `Content-Type` 是否为 `image/*` 类型
- 某些图床可能要求设置 Referer 或 User-Agent（LynxOCR 使用 `LynxOCR/1.1` 作为 User-Agent）

---

## 5. 模型切换

LynxOCR 支持三种 PaddleOCR 模型版本，可在请求中指定模型。需要先在「模型管理」页面下载对应模型。

### multipart 方式指定模型

```bash
curl -X POST http://localhost:9720/api/v1/ocr \
  -F "image=@/path/to/image.png" \
  -F "model=ppocr-v5"
```

### JSON 方式指定模型

```bash
curl -X POST http://localhost:9720/api/v1/ocr \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com/image.png", "model": "ppocr-v4"}'
```

**可用模型版本：**

| 模型名称   | 说明                                    |
|------------|-----------------------------------------|
| `ppocr-v6` | 最新版，多语言高精度（默认）            |
| `ppocr-v5` | 改进版，中英文识别精度提升              |
| `ppocr-v4` | 经典版，中英文文字检测与识别            |

---

## 6. 健康检查与信息查询

### 健康检查

```bash
curl http://localhost:9720/api/v1/health
```

响应：

```json
{
  "status": "ok",
  "model_loaded": true,
  "active_model": "ppocr-v6",
  "version": "1.1.0"
}
```

### 服务信息

```bash
curl http://localhost:9720/api/v1/info
```

响应：

```json
{
  "name": "LynxOCR API",
  "version": "1.1.0",
  "engine": "PaddleOCR ONNX",
  "available_models": ["ppocr-v4", "ppocr-v5", "ppocr-v6"],
  "active_model": "ppocr-v6",
  "max_file_size_mb": 20
}
```

---

## 7. 错误处理

所有错误响应遵循统一格式：

```json
{
  "success": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "人类可读的错误描述"
  }
}
```

### 错误码一览

| HTTP 状态码 | 错误码              | 说明                         | 常见原因                     |
|-------------|---------------------|------------------------------|------------------------------|
| 400         | `INVALID_IMAGE`     | 图片解码失败                 | 格式不支持、图片损坏、Base64 格式错误 |
| 400         | `NO_IMAGE`          | 未提供图片                   | 缺少 `image` 字段和 `url` 字段 |
| 400         | `INVALID_MODEL`     | 未知的模型版本               | 模型名称拼写错误             |
| 400         | `MODEL_NOT_INSTALLED`| 模型未下载                   | 请先在模型管理页面下载       |
| 401         | `UNAUTHORIZED`      | 认证失败                     | API 密钥错误或未提供         |
| 413         | `FILE_TOO_LARGE`    | 文件超过大小限制             | 调整 `maxFileSizeMb` 配置    |
| 500         | `OCR_ERROR`         | OCR 引擎错误                 | 模型加载失败、图片解码异常   |
| 500         | `INTERNAL_ERROR`    | 内部服务器错误               | 意外异常，请查看日志         |

---

## 8. 编程语言集成示例

### Python

```python
import requests
import base64
import json

API_BASE = "http://localhost:9720"
API_KEY = "your-secret-key"  # 为空则不需要认证

headers = {}
if API_KEY:
    headers["Authorization"] = f"Bearer {API_KEY}"

# 方式 1：上传本地图片
def ocr_local_file(image_path):
    with open(image_path, "rb") as f:
        files = {"image": f}
        resp = requests.post(f"{API_BASE}/api/v1/ocr", files=files, headers=headers)
    return resp.json()

# 方式 2：Base64 编码
def ocr_base64(image_path):
    with open(image_path, "rb") as f:
        b64 = base64.b64encode(f.read()).decode("utf-8")
    resp = requests.post(
        f"{API_BASE}/api/v1/ocr",
        json={"image": b64},
        headers=headers
    )
    return resp.json()

# 方式 3：图床 URL
def ocr_url(image_url):
    resp = requests.post(
        f"{API_BASE}/api/v1/ocr",
        json={"url": image_url},
        headers=headers
    )
    return resp.json()

# 使用示例
result = ocr_local_file("screenshot.png")
if result["success"]:
    print(f"识别文字: {result['data']['text']}")
    print(f"耗时: {result['data']['total_time_ms']}ms")
else:
    print(f"错误: {result['error']['message']}")
```

### JavaScript / Node.js

```javascript
const API_BASE = "http://localhost:9720";
const API_KEY = "your-secret-key";

const headers = {
  ...(API_KEY && { Authorization: `Bearer ${API_KEY}` }),
};

// 方式 1：上传本地图片（Node.js 18+）
async function ocrLocalFile(imagePath) {
  const { readFile } = require("node:fs/promises");
  const imageData = await readFile(imagePath);
  const blob = new Blob([imageData]);
  const formData = new FormData();
  formData.append("image", blob, "image.png");

  const resp = await fetch(`${API_BASE}/api/v1/ocr`, {
    method: "POST",
    headers,
    body: formData,
  });
  return resp.json();
}

// 方式 2：Base64 编码
async function ocrBase64(imagePath) {
  const { readFile } = require("node:fs/promises");
  const imageData = await readFile(imagePath);
  const b64 = imageData.toString("base64");

  const resp = await fetch(`${API_BASE}/api/v1/ocr`, {
    method: "POST",
    headers: { ...headers, "Content-Type": "application/json" },
    body: JSON.stringify({ image: b64 }),
  });
  return resp.json();
}

// 方式 3：图床 URL
async function ocrUrl(imageUrl) {
  const resp = await fetch(`${API_BASE}/api/v1/ocr`, {
    method: "POST",
    headers: { ...headers, "Content-Type": "application/json" },
    body: JSON.stringify({ url: imageUrl }),
  });
  return resp.json();
}
```

### Rust

```rust
use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Debug)]
struct OcrResponse {
    success: bool,
    data: Option<OcrData>,
    error: Option<ErrorDetail>,
}

#[derive(Deserialize, Debug)]
struct OcrData {
    text: String,
    total_time_ms: u64,
}

#[derive(Deserialize, Debug)]
struct ErrorDetail {
    code: String,
    message: String,
}

fn ocr_local_file(path: &str) -> Result<OcrResponse, Box<dyn std::error::Error>> {
    let form = ureq::multipart::Form::new()
        .file("image", path)?;
    let resp: OcrResponse = ureq::post("http://localhost:9720/api/v1/ocr")
        .send_multipart(form)?
        .into_json()?;
    Ok(resp)
}

fn ocr_url(image_url: &str) -> Result<OcrResponse, Box<dyn std::error::Error>> {
    let resp: OcrResponse = ureq::post("http://localhost:9720/api/v1/ocr")
        .set("Content-Type", "application/json")
        .send_json(ureq::json!({"url": image_url}))?
        .into_json()?;
    Ok(resp)
}
```

---

## 9. 常见问题

### Q: 支持哪些图片格式？

A: 支持 PNG、JPEG、BMP、WebP、TIFF、GIF 等常见格式。如果遇到无法识别的格式，请先将图片转换为 PNG 或 JPEG。

### Q: 图床链接下载失败怎么办？

A: 检查以下几点：
1. 图床链接是否可以直接在浏览器中访问
2. 服务器是否能访问外网（内网环境可能无法下载公网图片）
3. 某些图床可能有防盗链，需要设置 Referer（当前版本暂不支持自定义 Referer）

### Q: 如何提高识别速度？

A: 在 API 设置页面确保服务已启动，模型已预加载。首次请求会初始化模型引擎（约 1-2 秒），后续请求速度更快。

### Q: 可以同时处理多个请求吗？

A: 可以。API 服务器使用异步 I/O，可以同时处理多个并发请求。但 OCR 引擎使用互斥锁，实际识别会排队执行。

### Q: 如何修改默认端口？

A: 在 API 服务设置页面修改端口号，保存后重启服务即可。也可直接编辑 `config.json` 中的 `apiServerPort` 字段。

### Q: 如何关闭 API 服务？

A: 在 API 服务设置页面点击「停止服务」按钮，或直接关闭 LynxOCR 应用。

### Q: 数据安全吗？

A: LynxOCR 是完全离线的桌面应用，OCR 识别在本地进行，图片数据不会上传到任何云端服务器。API 服务仅在本地网络监听（默认 `127.0.0.1`），外部网络无法访问。