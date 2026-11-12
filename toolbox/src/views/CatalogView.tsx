import { useState } from "react";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  CardFooter,
} from "../components/ui/card";
import { Input } from "../components/ui/input";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import {
  Empty,
  EmptyHeader,
  EmptyTitle,
  EmptyDescription,
} from "../components/ui/empty";
import { StartAction } from "../components/StartAction";
import { useWorkspace } from "../lib/workspace";
import { useT, phaseText } from "../lib/i18n";
export function CatalogView({
  onService,
}: {
  onService: (id: string) => void;
}) {
  const { state } = useWorkspace();
  const t = useT();
  const [search, setSearch] = useState("");
  const services =
    state.catalog?.services.filter((s) =>
      `${s.id} ${s.name} ${s.category}`
        .toLowerCase()
        .includes(search.toLowerCase()),
    ) ?? [];
  return (
    <div className="flex flex-col gap-5">
      <div>
        <h1>{t("全部服务")}</h1>
        <p className="muted">
          {t("服务是否可用由真实连接验证决定；外部服务仅观察。")}
        </p>
      </div>
      <Input
        aria-label="搜索服务"
        placeholder="搜索服务或分类…"
        value={search}
        onChange={(e) => setSearch(e.target.value)}
      />
      <div className="card-grid">
        {services.map((service) => {
          const snapshot = state.snapshot?.services.find(
            (s) => s.serviceId === service.id,
          );
          return (
            <Card key={service.id}>
              <CardHeader>
                <div className="flex items-center justify-between gap-2">
                  <CardTitle>{service.name}</CardTitle>
                  <Badge variant="secondary">
                    {t(phaseText[snapshot?.phase ?? "Uninitialized"])}
                  </Badge>
                </div>
                <CardDescription>{service.category}</CardDescription>
              </CardHeader>
              <CardContent>
                <p className="muted">
                  依赖：{service.dependencies.join("、") || "无"}
                </p>
                <p className="muted">{snapshot?.messageCode}</p>
                {snapshot?.phase === "Ready" && (
                  <p className="muted">
                    {t("基础设施就绪 · 业务客户端待接入")}
                  </p>
                )}
              </CardContent>
              <CardFooter>
                <Button
                  variant="outline"
                  onClick={() => onService(service.id)}
                  aria-label={`查看 ${service.name}`}
                >
                  {t("查看")}
                </Button>
                {snapshot?.ownership !== "External" &&
                  snapshot?.phase !== "ExternalObserve" && (
                    <StartAction serviceIds={[service.id]} />
                  )}
              </CardFooter>
            </Card>
          );
        })}
      </div>
      {!services.length && (
        <Empty>
          <EmptyHeader>
            <EmptyTitle>{t("没有匹配的服务")}</EmptyTitle>
            <EmptyDescription>{t("换一个服务名或分类。")}</EmptyDescription>
          </EmptyHeader>
        </Empty>
      )}
    </div>
  );
}
