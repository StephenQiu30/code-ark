import { ToggleGroup, ToggleGroupItem } from "../components/ui/toggle-group";
import { Field, FieldLabel } from "../components/ui/field";
import { useState } from "react";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  CardFooter,
} from "../components/ui/card";
import { Button } from "../components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "../components/ui/dialog";
import { StartAction } from "../components/StartAction";
import { useWorkspace } from "../lib/workspace";
import { useT } from "../lib/i18n";
import type { SceneSpec } from "../types/toolbox";
const boundaries: Record<string, string> = {
  "backend-mysql": "应用 API 由您提供；教学范围为 SQL 查询与 S3 对象读写。",
  "backend-pgsql": "应用 API、Agent 与向量扩展由应用提供。",
  "java-microservices": "真实微服务与客户端集成由应用提供。",
  "java-distributed": "业务 undo_log、业务代码与 XXL 执行器由应用提供。",
  "workflow-temporal": "基础设施不包含业务 Worker；教程另提供测试 Worker。",
  "cdc-messaging":
    "分别验证订阅与消息收发；Canal → 队列桥接为选做示例，不会自动部署。",
  "search-observability": "真实业务采集器及采集目标由应用提供。",
};
export function LaunchpadView() {
  const { state, api, run } = useWorkspace();
  const [stopBusy, setStopBusy] = useState(false);
  const t = useT();
  const [selected, setSelected] = useState<SceneSpec | null>(null);
  const [queue, setQueue] = useState("kafka");
  const [search, setSearch] = useState("elastic");
  const catalog = state.catalog;
  const selectedIds =
    selected?.id === "cdc-messaging"
      ? [
          ...selected.services.filter(
            (id) => !["kafka", "rabbitmq", "rocketmq"].includes(id),
          ),
          queue,
        ]
      : (selected?.services.filter(
          (id) =>
            selected.id !== "search-observability" ||
            search === "both" ||
            (search === "monitoring"
              ? id === "monitoring"
              : id !== "monitoring"),
        ) ?? []);
  return (
    <section className="flex flex-col gap-4">
      <div className="section-heading">
        <h2>{t("选择需要的基础设施")}</h2>
        <span className="muted">
          {catalog?.scenes.length ?? 0} 个场景 · {catalog?.services.length ?? 0}{" "}
          个服务
        </span>
      </div>
      <div className="card-grid">
        {catalog?.scenes.map((scene) => (
          <Card key={scene.id}>
            <CardHeader>
              <CardTitle>{scene.name}</CardTitle>
              <CardDescription>
                {scene.services
                  .map(
                    (id) =>
                      catalog.services.find((s) => s.id === id)?.name ?? id,
                  )
                  .join(" / ")}
              </CardDescription>
            </CardHeader>
            <CardContent>
              <p className="muted">{t(boundaries[scene.id])}</p>
            </CardContent>
            <CardFooter>
              <Button
                variant="outline"
                aria-label={`查看 ${scene.name}`}
                onClick={() => setSelected(scene)}
              >
                {t("查看")}
              </Button>
            </CardFooter>
          </Card>
        ))}
      </div>
      <Dialog
        open={!!selected}
        onOpenChange={(open) => {
          if (!open) setSelected(null);
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{selected?.name}</DialogTitle>
            <DialogDescription>
              {selected && t(boundaries[selected.id])}
            </DialogDescription>
          </DialogHeader>
          {selected?.id === "cdc-messaging" && (
            <Field>
              <FieldLabel id="queue-label">
                {t("消息系统（只启动所选队列）")}
              </FieldLabel>
              <ToggleGroup
                aria-labelledby="queue-label"
                value={[queue]}
                onValueChange={(value) => setQueue(value[0] ?? queue)}
                variant="outline"
              >
                <ToggleGroupItem value="kafka">Kafka</ToggleGroupItem>
                <ToggleGroupItem value="rabbitmq">RabbitMQ</ToggleGroupItem>
                <ToggleGroupItem value="rocketmq">RocketMQ</ToggleGroupItem>
              </ToggleGroup>
            </Field>
          )}
          {selected?.id === "search-observability" && (
            <Field>
              <FieldLabel id="search-label">{t("选择范围")}</FieldLabel>
              <ToggleGroup
                aria-labelledby="search-label"
                value={[search]}
                onValueChange={(value) => setSearch(value[0] ?? search)}
                variant="outline"
              >
                <ToggleGroupItem value="elastic">Elastic</ToggleGroupItem>
                <ToggleGroupItem value="monitoring">Monitoring</ToggleGroupItem>
                <ToggleGroupItem value="both">{t("同时启动")}</ToggleGroupItem>
              </ToggleGroup>
            </Field>
          )}
          <ul>
            {selectedIds.map((id) => {
              const service = catalog?.services.find((s) => s.id === id);
              return (
                <li key={id}>
                  <strong>{service?.name ?? id}</strong> · 内存最低{" "}
                  {((service?.minMemoryBytes ?? 0) / 1024 ** 3).toFixed(2)} GiB
                  <p className="muted">
                    依赖复用：{service?.dependencies.join("、") || "无"} ·
                    数据：{service?.dataPaths.join("、") || "以已应用配置为准"}
                  </p>
                </li>
              );
            })}
          </ul>
          <StartAction
            serviceIds={selectedIds}
            sceneId={selected?.id ?? null}
          />
          <p className="muted">
            {t(
              "停止场景会保留数据；其他场景共用的依赖由后端保留，具体范围见运行任务。",
            )}
          </p>
          <Button
            variant="outline"
            disabled={
              stopBusy ||
              !state.snapshot?.target ||
              !state.snapshot.services.some(
                (service) =>
                  selectedIds.includes(service.serviceId) &&
                  service.instanceId &&
                  service.ownership.toLowerCase() === "managed",
              )
            }
            onClick={async () => {
              const snapshot = state.snapshot;
              if (!snapshot?.target || !selected) return;
              setStopBusy(true);
              try {
                await run(() =>
                  api.stopServices({
                    workspaceId: snapshot.workspace.workspaceId,
                    targetId: snapshot.target!.targetId,
                    instanceIds: snapshot.services
                      .filter(
                        (service) =>
                          selectedIds.includes(service.serviceId) &&
                          service.instanceId &&
                          service.ownership.toLowerCase() === "managed",
                      )
                      .map((service) => service.instanceId!),
                    sceneId: selected.id,
                  }),
                );
              } finally {
                setStopBusy(false);
              }
            }}
          >
            {t("停止场景（保留数据）")}
          </Button>
        </DialogContent>
      </Dialog>
    </section>
  );
}
