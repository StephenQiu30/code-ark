import { useState } from "react";
import { Search, SlidersHorizontal } from "lucide-react";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
} from "../components/ui/input-group";
import { Button } from "../components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "../components/ui/tabs";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "../components/ui/dropdown-menu";
import {
  Empty,
  EmptyHeader,
  EmptyTitle,
  EmptyDescription,
} from "../components/ui/empty";
import { EnvironmentConnectButton } from "../components/EnvironmentConnectButton";
import { PageHeader } from "../components/PageHeader";
import { ResourceList } from "../components/ResourceList";
import { ServiceRow } from "../components/ServiceRow";
import { categoryLabel } from "../lib/services";
import { useWorkspace } from "../lib/workspace";
import { useT } from "../lib/i18n";

export function CatalogView({
  onService,
}: {
  onService: (id: string) => void;
}) {
  const { state } = useWorkspace();
  const t = useT();
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("all");
  const [status, setStatus] = useState("all");
  const categories = [
    ...new Set(
      state.catalog?.services.map((s) => categoryLabel(s.category, s.id)) ?? [],
    ),
  ];
  const services =
    state.catalog?.services.filter((service) => {
      const phase =
        state.snapshot?.services.find((s) => s.serviceId === service.id)
          ?.phase ?? "Uninitialized";
      return (
        `${service.id} ${service.name} ${service.category} ${categoryLabel(service.category, service.id)}`
          .toLowerCase()
          .includes(search.toLowerCase()) &&
        (category === "all" ||
          categoryLabel(service.category, service.id) === category) &&
        (status === "all" ||
          (status === "ready"
            ? phase === "Ready"
            : status === "action"
              ? ["Failed", "Degraded", "AwaitingAction"].includes(phase)
              : ["Stopped", "Uninitialized"].includes(phase)))
      );
    }) ?? [];
  const statuses: Record<string, string> = {
    all: "全部状态",
    ready: "已就绪",
    stopped: "未启动 / 已停止",
    action: "需要处理",
  };
  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title={t("全部服务")}
        description={`${state.catalog?.services.length ?? 0} 套环境，按需启动。`}
        actions={
          <EnvironmentConnectButton
            variant="outline"
            label={state.snapshot?.target ? "检测连接" : "连接 Docker"}
          />
        }
      />
      <div className="flex items-center justify-between gap-4">
        <InputGroup className="max-w-90">
          <InputGroupAddon>
            <Search />
          </InputGroupAddon>
          <InputGroupInput
            aria-label="搜索服务"
            placeholder={t("搜索服务…")}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </InputGroup>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="outline">
              <SlidersHorizontal data-icon="inline-start" />
              {t(statuses[status])}
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuGroup>
              {Object.entries(statuses).map(([key, label]) => (
                <DropdownMenuItem key={key} onSelect={() => setStatus(key)}>
                  {t(label)}
                </DropdownMenuItem>
              ))}
            </DropdownMenuGroup>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
      <Tabs value={category} onValueChange={setCategory}>
        <TabsList
          variant="line"
          className="h-auto max-w-full flex-wrap justify-start"
        >
          <TabsTrigger value="all">{t("全部")}</TabsTrigger>
          {categories.map((c) => (
            <TabsTrigger key={c} value={c}>
              {t(c)}
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>
      <ResourceList items={services} getKey={(s) => s.id}>
        {(service) => (
          <ServiceRow
            launch
            service={service}
            snapshot={state.snapshot?.services.find(
              (s) => s.serviceId === service.id,
            )}
            onSelect={onService}
          />
        )}
      </ResourceList>
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
