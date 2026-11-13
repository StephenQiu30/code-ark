import { EnvironmentConnectButton } from "../components/EnvironmentConnectButton";
import { Separator } from "../components/ui/separator";
import {
  Empty,
  EmptyHeader,
  EmptyTitle,
  EmptyDescription,
} from "../components/ui/empty";
import { PageHeader } from "../components/PageHeader";
import { ResourceList } from "../components/ResourceList";
import { ServiceRow } from "../components/ServiceRow";
import { StartAction } from "../components/StartAction";
import { useWorkspace } from "../lib/workspace";
import { useT } from "../lib/i18n";
import { LaunchpadView } from "./LaunchpadView";

export function WelcomeView({
  onService,
}: {
  onService: (id: string) => void;
}) {
  const { state } = useWorkspace();
  const t = useT();
  const recent =
    state.snapshot?.services
      .filter((s) => s.instanceId)
      .slice(0, 3)
      .flatMap((snapshot) => {
        const service = state.catalog?.services.find(
          (s) => s.id === snapshot.serviceId,
        );
        return service ? [{ service, snapshot }] : [];
      }) ?? [];
  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title={t("概览")}
        description={t("启动服务，继续你的开发。")}
      />
      <section
        className="flex flex-wrap items-center justify-between gap-6"
        aria-label={t("从 Redis 开始")}
      >
        <div className="flex flex-col gap-2">
          <h2 className="section-title">{t("从 Redis 开始")}</h2>
          <p className="muted">{t("检查环境、启动服务并验证连接。")}</p>
        </div>
        <div className="flex flex-wrap gap-2">
          {!state.snapshot?.target && (
            <EnvironmentConnectButton variant="outline" label="检测 Docker 环境" />
          )}
          <StartAction serviceIds={["redis"]} label="启动 Redis" />
        </div>
      </section>
      <Separator />
      <section className="flex flex-col gap-6">
        <h2 className="section-title">{t("最近使用")}</h2>
        {recent.length ? (
          <ResourceList items={recent} getKey={(item) => item.service.id}>
            {({ service, snapshot }) => (
              <ServiceRow
                service={service}
                snapshot={snapshot}
                onSelect={onService}
              />
            )}
          </ResourceList>
        ) : (
          <Empty>
            <EmptyHeader>
              <EmptyTitle>{t("还没有启动过服务")}</EmptyTitle>
              <EmptyDescription>
                {t("从 Redis 开始，或为项目选择一个场景。")}
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
        )}
      </section>
      <LaunchpadView compact />
    </div>
  );
}
