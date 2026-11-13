import { cn } from "../lib/utils";

export function LogoMark() {
  return <span aria-hidden="true" className="brand-mark" />;
}

export function Brand({ compact = false }: { compact?: boolean }) {
  return (
    <span
      className="flex shrink-0 items-center gap-3"
      aria-label="代码方舟 · Code Ark"
    >
      <LogoMark />
      <span
        className={cn(
          "brand-name",
          compact && "sr-only",
          "group-data-[collapsible=icon]:sr-only",
        )}
      >
        代码方舟
      </span>
    </span>
  );
}
