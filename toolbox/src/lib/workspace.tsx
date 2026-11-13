import {
  createContext,
  useContext,
  useEffect,
  useReducer,
  useCallback,
  useRef,
  useMemo,
  useState,
} from "react";
import type { ReactNode } from "react";
import { useFeedback, type Feedback } from "./feedback";
import { notifyingApi, consumeApiFailure } from "./notifying-api";
import { errorCode } from "../api/tauri";
import type { ToolboxApi } from "../api/tauri";
import type {
  DashboardSnapshot,
  ServiceCatalog,
  StepProgressEvent,
} from "../types/toolbox";
type State = {
  catalog: ServiceCatalog | null;
  snapshot: DashboardSnapshot | null;
  error: string | null;
  disconnectedAt: number | null;
};
type Action =
  | { type: "catalog"; value: ServiceCatalog }
  | { type: "snapshot"; value: DashboardSnapshot }
  | { type: "error"; code: string }
  | { type: "event"; value: StepProgressEvent };
export function workspaceReducer(state: State, action: Action): State {
  if (action.type === "catalog") return { ...state, catalog: action.value };
  if (action.type === "error")
    return {
      ...state,
      error: action.code,
      disconnectedAt: state.disconnectedAt ?? Date.now(),
    };
  if (action.type === "snapshot")
    return {
      ...state,
      snapshot: action.value,
      error: null,
      disconnectedAt: null,
    };
  const current = state.snapshot?.operations.find(
    (o) => o.operationId === action.value.operationId,
  );
  if (!current || action.value.seq <= current.seq) return state;
  // Events never manufacture service readiness. Refreshing snapshots is authoritative.
  return state;
}
const Context = createContext<{
  api: ToolboxApi;
  feedback: Feedback;
  state: State;
  refresh: () => Promise<void>;
  detecting: boolean;
  detectEnvironment: () => Promise<DashboardSnapshot | null>;
  run: <R>(task: () => Promise<R>) => Promise<R | null>;
} | null>(null);
export function WorkspaceProvider({
  api: rawApi,
  children,
}: {
  api: ToolboxApi;
  children: ReactNode;
}) {
  const feedback = useFeedback();
  const api = useMemo(() => notifyingApi(rawApi, feedback), [rawApi, feedback]);
  const connectionIssue = useRef<string | null>(null);
  const environmentIssue = useRef<string | null>(null);
  const observedOperations = useRef(new Map<string, string>());
  const observedServices = useRef(new Map<string, string>());
  const [state, dispatch] = useReducer(workspaceReducer, {
    catalog: null,
    snapshot: null,
    error: null,
    disconnectedAt: null,
  });
  const [detecting, setDetecting] = useState(false);
  const pendingDetection = useRef<Promise<DashboardSnapshot | null> | null>(
    null,
  );
  const inFlight = useRef<Promise<void> | null>(null);
  const catalogLoaded = useRef(false);
  const eventSeqs = useRef(new Map<string, number>());
  const activeWorkspace = useRef<string | null>(null);
  const refresh: () => Promise<void> = useCallback(() => {
    if (inFlight.current) return inFlight.current;
    const pending = Promise.all([
      api.dashboardSnapshot(),
      catalogLoaded.current
        ? Promise.resolve()
        : api.serviceCatalog().then((value) => {
            catalogLoaded.current = true;
            dispatch({ type: "catalog", value });
          }),
    ])
      .then(([value]) => {
        if (activeWorkspace.current !== value.workspace.workspaceId) {
          eventSeqs.current.clear();
          observedOperations.current.clear();
          observedServices.current.clear();
        }
        activeWorkspace.current = value.workspace.workspaceId;
        for (const operation of value.operations)
          eventSeqs.current.set(
            operation.operationId,
            Math.max(
              operation.seq,
              eventSeqs.current.get(operation.operationId) ?? -1,
            ),
          );
        if (connectionIssue.current) {
          connectionIssue.current = null;
          feedback.dismiss("backend-connection");
          feedback.success("已重新连接工作箱后端");
        }
        const environmentKey = value.environmentError
          ? `${value.workspace.workspaceId}:${value.target?.targetId}:${value.environmentError}`
          : null;
        if (environmentKey !== environmentIssue.current) {
          environmentIssue.current = environmentKey;
          if (value.environmentError)
            feedback.error(
              { code: value.environmentError },
              {
                id: "runtime-environment",
                persistent: true,
                action: {
                  label: "Docker 官方安装指引",
                  onClick: () => {
                    void api.openOfficial("docker-install").catch(() => {});
                  },
                },
              },
            );
          else feedback.dismiss("runtime-environment");
        }
        dispatch({ type: "snapshot", value });
      })
      .catch((e) => {
        const code = errorCode(e);
        if (connectionIssue.current !== code) {
          connectionIssue.current = code;
          feedback.error(e, {
            id: "backend-connection",
            persistent: true,
            action: {
              label: "重新检测",
              onClick: () => {
                void refresh();
              },
            },
          });
        }
        dispatch({ type: "error", code });
      })
      .finally(() => {
        inFlight.current = null;
      });
    inFlight.current = pending;
    return pending;
  }, [api, feedback]);
  const detectEnvironment = useCallback(() => {
    if (pendingDetection.current) return pendingDetection.current;
    setDetecting(true);
    const pending = api
      .detectEnvironment()
      .then(async (value) => {
        environmentIssue.current = value.environmentError
          ? `${value.workspace.workspaceId}:${value.target?.targetId}:${value.environmentError}`
          : null;
        feedback.dismiss("runtime-environment");
        dispatch({ type: "snapshot", value });
        await refresh();
        return value;
      })
      .catch((error) => {
        if (!consumeApiFailure(error)) feedback.error(error);
        return null;
      })
      .finally(() => {
        pendingDetection.current = null;
        setDetecting(false);
      });
    pendingDetection.current = pending;
    return pending;
  }, [api, feedback, refresh]);
  const run = useCallback(
    async <R,>(task: () => Promise<R>): Promise<R | null> => {
      try {
        const result = await task();
        await refresh();
        return result;
      } catch (e) {
        if (!consumeApiFailure(e)) feedback.error(e);
        if (errorCode(e) === "BackendUnavailable") await refresh();
        return null;
      }
    },
    [refresh, feedback],
  );
  useEffect(() => {
    let alive = true;
    let unlisten: (() => void) | undefined;
    const seqs = eventSeqs.current;
    void refresh();
    void api
      .subscribeProgress((event) => {
        if (!alive || event.seq <= (seqs.get(event.operationId) ?? -1)) return;
        seqs.set(event.operationId, event.seq);
        dispatch({ type: "event", value: event });
        void refresh();
      })
      .then((stop) => {
        if (!alive) stop();
        else unlisten = stop;
      })
      .catch((e) => {
        if (alive && errorCode(e) !== "BackendUnavailable") feedback.error(e);
      });
    const interval = setInterval(() => {
      if (alive) void refresh();
    }, 5000);
    const focus = () => {
      void refresh();
    };
    window.addEventListener("focus", focus);
    window.addEventListener("online", focus);
    return () => {
      alive = false;
      clearInterval(interval);
      unlisten?.();
      window.removeEventListener("focus", focus);
      window.removeEventListener("online", focus);
    };
  }, [api, refresh, feedback]);
  useEffect(() => {
    if (!state.snapshot) return;
    const nextOperations = new Map<string, string>();
    for (const operation of state.snapshot.operations) {
      const key = `${operation.phase}:${operation.errorCode ?? ""}`;
      const previous = observedOperations.current.get(operation.operationId);
      nextOperations.set(operation.operationId, key);
      if (previous === key) continue;
      if (
        operation.errorCode &&
        ["Failed", "Degraded", "AwaitingAction"].includes(operation.phase)
      )
        feedback.error({ code: operation.errorCode });
      else if (
        previous &&
        ["Completed", "Succeeded", "Ready"].includes(operation.phase)
      )
        feedback.success("服务操作已完成");
    }
    observedOperations.current = nextOperations;
    const nextServices = new Map<string, string>();
    for (const service of state.snapshot.services) {
      if (
        !service.messageCode ||
        !["Failed", "Degraded", "AwaitingAction"].includes(service.phase)
      )
        continue;
      const key = `${service.instanceId}:${service.messageCode}`;
      nextServices.set(service.serviceId, key);
      if (observedServices.current.get(service.serviceId) !== key)
        feedback.error({ code: service.messageCode });
    }
    observedServices.current = nextServices;
  }, [state.snapshot, feedback]);
  return (
    <Context.Provider
      value={{
        api,
        feedback,
        state,
        refresh,
        run,
        detecting,
        detectEnvironment,
      }}
    >
      {children}
    </Context.Provider>
  );
}
export function useWorkspace() {
  const value = useContext(Context);
  if (!value) throw Error("WorkspaceProvider required");
  return value;
}
