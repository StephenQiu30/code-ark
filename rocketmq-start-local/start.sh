#!/bin/sh
# RocketMQ 启动脚本
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "${SCRIPT_DIR}"

echo "---------------------------------------"
echo "  RocketMQ 启动中..."
echo "---------------------------------------"
echo ""
echo "服务列表："
echo "  - NameServer:  http://localhost:9876"
echo "  - Broker:      http://localhost:10911"
echo "  - Console:     http://localhost:8180"
echo ""
echo "等待服务启动..."
docker compose up -d

echo ""
echo "---------------------------------------"
echo "  RocketMQ 启动完成！"
echo "---------------------------------------"
echo ""
echo "查看日志: docker compose logs -f"
echo "停止服务: ./stop.sh"
