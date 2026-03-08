#!/bin/sh
# Sentinel 启动脚本
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "${SCRIPT_DIR}"

echo "---------------------------------------"
echo "  Sentinel 启动中..."
echo "---------------------------------------"
echo ""
echo "服务列表："
echo "  - Dashboard:   http://localhost:8858"
echo "  - API Port:    8719"
echo ""
echo "默认账号密码: sentinel / sentinel"
echo ""
echo "等待服务启动..."
docker compose up -d

echo ""
echo "---------------------------------------"
echo "  Sentinel 启动完成！"
echo "---------------------------------------"
echo ""
echo "查看日志: docker compose logs -f"
echo "停止服务: docker compose down"
echo ""