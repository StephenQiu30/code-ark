import type { ComponentProps } from "react";
import { RefreshCw } from "lucide-react";
import { Button } from "./ui/button";
import { useWorkspace } from "../lib/workspace";
import { useT } from "../lib/i18n";
import { cn } from "../lib/utils";

export function EnvironmentConnectButton({
  label = "检测连接",
  disabled,
  variant = "ghost",
  ...props
}: Omit<ComponentProps<typeof Button>, "children" | "onClick"> & {
  label?: string;
}) {
  const { detecting, detectEnvironment } = useWorkspace();
  const t = useT();
  return (
    <Button
      {...props}
      variant={variant}
      disabled={disabled || detecting}
      onClick={() => void detectEnvironment()}
    >
      <RefreshCw
        data-icon="inline-start"
        className={cn({ "animate-spin": detecting })}
      />
      {t(detecting ? "正在检测…" : label)}
    </Button>
  );
}
