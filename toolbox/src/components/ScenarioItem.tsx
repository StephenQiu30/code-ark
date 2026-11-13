import { ArrowUpRight, Layers } from "lucide-react";
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from "./ui/item";
import type { SceneSpec, ServiceSpec } from "../types/toolbox";

export function ScenarioItem({
  scene,
  services,
  onSelect,
}: {
  scene: SceneSpec;
  services: ServiceSpec[];
  onSelect: () => void;
}) {
  return (
    <Item asChild variant="scenario">
      <button
        type="button"
        onClick={onSelect}
        aria-label={`查看 ${scene.name}`}
      >
        <ItemMedia>
          <Layers className="size-6" />
        </ItemMedia>
        <ItemContent className="min-w-0 gap-2">
          <ItemTitle>{scene.name}</ItemTitle>
          <ItemDescription>
            {scene.services
              .map((id) => services.find((s) => s.id === id)?.name ?? id)
              .join(" / ")}
          </ItemDescription>
          <span className="text-xs text-muted-foreground">
            {scene.services.length} 项服务 · 优先复用已有依赖
          </span>
        </ItemContent>
        <ItemActions>
          <ArrowUpRight className="size-4" aria-hidden="true" />
        </ItemActions>
      </button>
    </Item>
  );
}
