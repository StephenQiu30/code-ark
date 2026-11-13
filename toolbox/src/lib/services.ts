import {
  Activity,
  Container,
  Database,
  HardDrive,
  Layers,
  Network,
  Server,
  Workflow,
  Zap,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { ServiceSpec } from "../types/toolbox";

const icons: Record<string, LucideIcon> = {
  redis: Zap,
  pgsql: Database,
  mysql: Database,
  mongodb: Database,
  minio: HardDrive,
  temporal: Workflow,
  kafka: Network,
  rabbitmq: Network,
  rocketmq: Network,
  nacos: Server,
  elastic: Activity,
  monitoring: Activity,
  canal: Layers,
};
export const serviceIcon = (id: string) => icons[id] ?? Container;
export const categoryLabels: Record<string, string> = {
  database: "数据库",
  cache: "缓存",
  messaging: "消息队列",
  workflow: "工作流",
  search: "搜索与监控",
  monitoring: "搜索与监控",
  observability: "搜索与监控",
  content: "内容工具",
  storage: "存储",
  governance: "服务治理",
  coordination: "服务治理",
  transaction: "事务",
  scheduler: "任务调度",
  cdc: "数据同步",
};
export const categoryLabel = (category: string, serviceId?: string) =>
  serviceId === "redis" ? "缓存" : (categoryLabels[category] ?? category);
export function defaultEndpoint(service: ServiceSpec) {
  const port = service.ports[0];
  return port ? `默认 ${port.hostIp}:${port.hostPort}` : "尚未创建";
}
