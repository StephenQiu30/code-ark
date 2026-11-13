import { useEffect, useState } from "react";
import { ArrowUpRight } from "lucide-react";
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from "./ui/item";
import { StartAction } from "./StartAction";
import { StatusBadge } from "./StatusBadge";
import { useWorkspace } from "../lib/workspace";
import { categoryLabel, defaultEndpoint, serviceIcon } from "../lib/services";
import type { ServiceSnapshot, ServiceSpec } from "../types/toolbox";

export function ServiceRow({
  service,
  snapshot,
  onSelect,
  launch = false,
}: {
  service: ServiceSpec;
  snapshot?: ServiceSnapshot;
  onSelect: (id: string) => void;
  launch?: boolean;
}) {
  const { api } = useWorkspace();
  const [endpoint, setEndpoint] = useState<string | null>(null);
  const instanceId = snapshot?.instanceId;
  const external =
    snapshot?.phase === "ExternalObserve" ||
    snapshot?.ownership.toLowerCase().startsWith("external");
  const canReadConnection = !!snapshot?.appliedGeneration || external;
  useEffect(() => {
    let alive = true;
    setEndpoint(null);
    if (instanceId && canReadConnection)
      void api
        .connectionView(instanceId)
        .then((view) => {
          const first = view.hostEndpoints[0];
          if (alive && first) setEndpoint(`${first.host}:${first.port}`);
        })
        .catch(() => {});
    return () => {
      alive = false;
    };
  }, [api, instanceId, snapshot?.appliedGeneration, canReadConnection]);
  const Icon = serviceIcon(service.id);
  return (
    <Item variant="service">
      <button
        className="flex min-w-0 flex-1 items-center gap-4 text-left outline-none focus-visible:ring-2 focus-visible:ring-ring rounded-md"
        type="button"
        data-service-id={service.id}
        aria-label={`查看 ${service.name}`}
        onClick={() => onSelect(service.id)}
      >
        <ItemMedia>
          <Icon className="size-6" />
        </ItemMedia>
        <ItemContent className="min-w-0">
          <ItemTitle>{service.name}</ItemTitle>
          <ItemDescription>
            {categoryLabel(service.category, service.id)}
          </ItemDescription>
        </ItemContent>
        <span className="hidden w-44 shrink-0 truncate text-left font-mono text-xs text-muted-foreground lg:block">
          {canReadConnection
            ? (endpoint ?? "连接信息待读取")
            : service.ports.length || !instanceId
              ? defaultEndpoint(service)
              : "配置预设"}
        </span>
        <ItemActions className="ml-auto gap-5">
          <StatusBadge phase={snapshot?.phase ?? "Uninitialized"} />
          <ArrowUpRight className="size-4 shrink-0" aria-hidden="true" />
        </ItemActions>
      </button>
      {launch && snapshot?.phase !== "Ready" && !external && (
        <StartAction serviceIds={[service.id]} label={`启动 ${service.name}`} />
      )}
    </Item>
  );
}
