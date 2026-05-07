# PaddleOCR 本地识别服务

> [返回项目首页](../README.md) | [Back to root](../README.md)

## English Summary

This directory starts a local PaddleOCR HTTP service for image text recognition.
It uses PaddleOCR as the OCR engine and exposes a small FastAPI wrapper for development integration.
The current setup does not require a `.env` file.

## 服务简介

该目录提供基于百度开源 PaddleOCR 的本地 OCR 服务，适合：

- 图片文字检测与识别
- 中英文混合文本识别验证
- 本地后端服务联调 OCR 能力
- 后续扩展版面分析、表格识别等 PaddleOCR 能力

当前服务默认使用 CPU 运行，首次构建和首次识别会下载 Python 依赖与 PaddleOCR 模型，耗时会比较久。

## 端口与访问入口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| PaddleOCR API | `18868` | 宿主机访问端口，容器内端口为 `8868` |

- 健康检查：`http://localhost:18868/health`
- OCR 接口：`http://localhost:18868/ocr`

## 前置条件

- 已安装 Docker 和 Docker Compose

## 快速启动

```bash
cd ocr-start-local
mkdir -p data models
docker compose up -d --build
```

启动后可以检查服务状态：

```bash
curl http://localhost:18868/health
```

## 配置说明

当前配置直接写在 `docker-compose.yml` 和 `Dockerfile` 中：

- OCR 引擎：`paddleocr>=3.0.0,<4.0.0`
- 推理框架：`paddlepaddle>=3.0.0,<4.0.0`
- HTTP 服务：FastAPI + Uvicorn
- 宿主机端口：`18868`
- 输入输出目录：`./data`
- 模型缓存目录：`./models`
- 默认语言：`ch`

目录说明：

- `./data`：放置本地测试图片或临时样例
- `./models`：缓存 PaddleOCR 下载的模型文件，避免每次重建后重复下载
- `./app/server.py`：本地 HTTP 包装层，底层调用 PaddleOCR Python API

该服务是本地开发依赖，不建议直接暴露到公网。

## 使用示例

识别一张图片：

```bash
curl \
  --request POST http://localhost:18868/ocr \
  --form file=@./data/input.png
```

指定语言：

```bash
curl \
  --request POST http://localhost:18868/ocr \
  --form file=@./data/input.png \
  --form lang=ch
```

返回结果包含：

- `text`：按行拼接后的识别文本
- `lines`：识别文本数组
- `results`：PaddleOCR 原始 JSON 结果，包含文本框、置信度等信息

常见语言参数：

- `ch`：中文与英文混合识别
- `en`：英文识别
- `chinese_cht`：繁体中文识别
- `japan`：日文识别

更多语言与模型能力请参考 PaddleOCR 官方文档。

## PDF 处理说明

PaddleOCR 当前接口优先处理图片。如果需要处理 PDF，推荐先通过本仓库的 LibreOffice / Gotenberg 服务或其他工具把页面渲染为图片，再调用 `/ocr` 识别。

如果你的目标只是把扫描版 PDF 转为可搜索 PDF，OCRmyPDF 仍然是更直接的专用工具；但本目录默认选择 PaddleOCR，是为了提供更通用的 OCR 能力底座。

## 数据持久化与清理

- 数据目录：`./data`
- 模型缓存目录：`./models`

```bash
docker compose down
rm -rf ./data
rm -rf ./models
```

## 常用命令

```bash
docker compose up -d --build
docker compose ps
docker compose logs -f
docker compose stop
docker compose down
```

## 返回导航

- 项目首页：[`../README.md`](../README.md)
