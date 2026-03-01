#!/bin/sh
# 监控面板启动脚本
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "${SCRIPT_DIR}"

echo "---------------------------------------"
echo "  分布式监控面板启动中..."
echo "---------------------------------------"
echo ""
echo "服务列表："
echo "  - Prometheus:  http://localhost:19090"
echo "  - Grafana:     http://localhost:13000 (admin/admin)"
echo ""
echo "等待服务启动..."
docker compose up -d

echo ""
echo "---------------------------------------"
echo "  监控面板启动完成！"
echo "---------------------------------------"
echo ""
echo "查看日志: docker compose logs -f"
echo "停止服务: ./stop.sh"
