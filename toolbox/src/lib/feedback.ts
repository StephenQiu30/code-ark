import { useMemo, useRef } from "react";
import { toast } from "sonner";
import { publicError } from "./errors";
import { useT } from "./i18n";
import type { PreflightReport } from "../types/toolbox";

type Options = {
  id?: string;
  persistent?: boolean;
  action?: { label: string; onClick: () => void };
};

export function createFeedback(t: (text: string) => string) {
  const error = (value: unknown, options: Options = {}) => {
    const issue = publicError(value);
    return toast.error(t(issue.title), {
      id:
        options.id ??
        (issue.code === "BackendUnavailable"
          ? "backend-connection"
          : `error:${issue.code}`),
      description: `${t(issue.description)} · ${issue.code}`,
      duration: options.persistent ? Infinity : 7000,
      action: options.action && {
        label: t(options.action.label),
        onClick: options.action.onClick,
      },
    });
  };
  const warning = (value: unknown) => {
    const issue = publicError(value);
    return toast.warning(t(issue.title), {
      id: `warning:${issue.code}`,
      description: `${t(issue.description)} · ${issue.code}`,
    });
  };
  const success = (message: string, options: Options = {}) =>
    toast.success(t(message), {
      id: options.id,
      duration: 7000,
      description: undefined,
      action: undefined,
    });
  const loading = (message: string, options: Options = {}) =>
    toast.loading(t(message), {
      id: options.id,
      duration: Infinity,
      description: undefined,
      action: undefined,
    });
  const info = (message: string, options: Options = {}) =>
    toast.info(t(message), {
      id: options.id ?? `info:${message}`,
      duration: options.persistent ? Infinity : 5000,
      action: options.action && {
        label: t(options.action.label),
        onClick: options.action.onClick,
      },
    });
  return {
    error,
    success,
    loading,
    warning,
    info,
    dismiss: (id: string) => toast.dismiss(id),
    async copy(value: string) {
      try {
        await navigator.clipboard.writeText(value);
        success("已复制");
      } catch {
        error({ code: "ClipboardUnavailable" });
      }
    },
    preflight(report: PreflightReport, announceSuccess = false) {
      const issue = report.issues.find(
        (i) => i.severity.toLowerCase() === "error",
      );
      if (issue) error({ code: issue.code });
      else if (report.issues[0]) warning({ code: report.issues[0].code });
      else if (report.requiresCapacityConsent)
        info("可用磁盘容量未知，继续操作前需要单独确认。");
      else if (announceSuccess) success("所选服务预检通过");
    },
  };
}

export type Feedback = ReturnType<typeof createFeedback>;

export function useFeedback() {
  const t = useT();
  const translate = useRef(t);
  translate.current = t;
  return useMemo(() => createFeedback((text) => translate.current(text)), []);
}
