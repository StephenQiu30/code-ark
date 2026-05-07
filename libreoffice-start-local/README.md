# LibreOffice 本地文档转换环境

> [返回项目首页](../README.md) | [Back to root](../README.md)

## English Summary

This directory builds a real LibreOffice container for local document conversion.
It runs LibreOffice in headless mode through the `soffice` command.
The current setup does not expose an HTTP service and does not require a `.env` file.

## 服务简介

该目录提供真实 LibreOffice 本体的 Docker 环境，适合：

- Word、Excel、PowerPoint 转 PDF
- OpenDocument、RTF、TXT 等格式转 PDF
- 本地验证 LibreOffice headless 转换效果
- 直接使用 LibreOffice 本体执行文档转换

当前配置采用一次性命令容器，不常驻运行，也不占用宿主机端口。

## 端口与访问入口

| 组件 | 端口 | 说明 |
| --- | --- | --- |
| LibreOffice | 无 | 通过 `docker compose run --rm` 执行 `soffice` 转换命令 |

## 前置条件

- 已安装 Docker 和 Docker Compose
- 将待转换文件放入 `./data`

## 快速启动

```bash
cd libreoffice-start-local
mkdir -p data fonts
docker compose build
docker compose run --rm libreoffice --version
```

## 配置说明

当前镜像基于 `debian:bookworm-slim` 构建，并安装：

- `libreoffice`
- `libreoffice-l10n-zh-cn`
- `fonts-noto-cjk`
- `fonts-liberation`
- `fonts-dejavu`

目录说明：

- `./data`：放置待转换文件和输出文件
- `./fonts`：可选自定义字体目录，只读挂载到容器内 `/usr/local/share/fonts/custom`

如需提高中文、Office 模板或品牌字体的版式一致性，可以把字体文件放入 `./fonts` 后重新运行转换命令。

## 使用示例

将 Word 文档转换为 PDF：

```bash
cp /path/to/document.docx ./data/document.docx
docker compose run --rm libreoffice \
  --convert-to pdf \
  --outdir /data \
  /data/document.docx
```

将 Excel 文档转换为 PDF：

```bash
cp /path/to/workbook.xlsx ./data/workbook.xlsx
docker compose run --rm libreoffice \
  --convert-to pdf \
  --outdir /data \
  /data/workbook.xlsx
```

常见可用输入包括：

- `doc`、`docx`、`odt`、`rtf`、`txt`
- `xls`、`xlsx`、`ods`、`csv`
- `ppt`、`pptx`、`odp`

复杂版式、特殊字体或 Office 专有效果可能和 Microsoft Office 渲染结果存在差异。转换结果请以真实样例验证为准。

## 数据持久化与清理

- 数据目录：`./data`
- 自定义字体目录：`./fonts`

```bash
docker compose down
rm -rf ./data
rm -rf ./fonts
```

## 常用命令

```bash
docker compose build
docker compose run --rm libreoffice --version
docker compose run --rm libreoffice --convert-to pdf --outdir /data /data/document.docx
docker compose down
```

## 返回导航

- 项目首页：[`../README.md`](../README.md)
