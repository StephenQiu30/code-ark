import { Badge } from "./ui/badge";
import { phaseText, useT } from "../lib/i18n";
import type { ServicePhase } from "../types/toolbox";

export function StatusBadge({ phase }: { phase: ServicePhase }) {
  const t = useT();
  const variant =
    phase === "Ready"
      ? "status-ready"
      : phase === "Failed"
        ? "status-error"
        : phase === "AwaitingAction" || phase === "Degraded"
          ? "status-warning"
          : "status";
  return <Badge variant={variant}>{t(phaseText[phase])}</Badge>;
}
