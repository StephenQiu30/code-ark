import { Component, useEffect, type ReactNode } from "react";
import { Toaster } from "./ui/sonner";
import { Button } from "./ui/button";
import { Skeleton } from "./ui/skeleton";
import { useFeedback, type Feedback } from "../lib/feedback";
import { useT } from "../lib/i18n";
import { consumeApiFailure } from "../lib/notifying-api";

export function GlobalFeedback() {
  const t = useT();
  const feedback = useFeedback();
  useEffect(() => {
    const unexpected = (event: ErrorEvent) => {
      // Runtime exception messages and stacks can contain credentials.
      event.preventDefault();
      feedback.error({ code: "FrontendUnexpectedError" });
    };
    const rejection = (event: PromiseRejectionEvent) => {
      event.preventDefault();
      if (!consumeApiFailure(event.reason))
        feedback.error({ code: "FrontendUnexpectedError" });
    };
    window.addEventListener("error", unexpected);
    window.addEventListener("unhandledrejection", rejection);
    return () => {
      window.removeEventListener("error", unexpected);
      window.removeEventListener("unhandledrejection", rejection);
    };
  }, [feedback]);
  return (
    <Toaster
      position="bottom-right"
      closeButton
      visibleToasts={3}
      containerAriaLabel={t("全局消息")}
      hotkey={["altKey", "KeyT"]}
      toastOptions={{
        closeButtonAriaLabel: t("关闭消息"),
        actionButtonStyle: {
          background: "var(--primary)",
          color: "var(--primary-foreground)",
        },
      }}
    />
  );
}

class Boundary extends Component<
  { children: ReactNode; feedback: Feedback },
  { failed: boolean }
> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  componentDidCatch() {
    this.props.feedback.error(
      { code: "FrontendRenderFailed" },
      {
        persistent: true,
        action: {
          label: "重新加载页面",
          onClick: () => this.retry(),
        },
      },
    );
  }
  retry = () => {
    window.location.reload();
  };
  render() {
    if (!this.state.failed) return this.props.children;
    return (
      <div className="workspace-page">
        <Skeleton className="h-12 w-40" />
        <Skeleton className="h-48" />
        <div>
          <Button onClick={this.retry}>重新加载页面</Button>
        </div>
      </div>
    );
  }
}

export function AppErrorBoundary({ children }: { children: ReactNode }) {
  const feedback = useFeedback();
  return <Boundary feedback={feedback}>{children}</Boundary>;
}
