#!/bin/sh
# Sentinel 启动脚本
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "${SCRIPT_DIR}"

if [ -f .env ]; then
  set -a
  . ./.env
  set +a
fi

SENTINEL_DASHBOARD_PORT="${SENTINEL_DASHBOARD_PORT:-8858}"
SENTINEL_API_PORT="${SENTINEL_API_PORT:-8719}"

echo "---------------------------------------"
echo "  Sentinel 启动中..."
echo "---------------------------------------"
echo ""
echo "服务列表："
echo "  - Dashboard:   http://localhost:${SENTINEL_DASHBOARD_PORT}"
echo "  - API Port:    ${SENTINEL_API_PORT}"
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
