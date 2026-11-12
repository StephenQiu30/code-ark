import {
  createContext,
  useContext,
  useEffect,
  useReducer,
  useCallback,
  useRef,
} from "react";
import type { ReactNode } from "react";
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
  state: State;
  refresh: () => Promise<void>;
  run: <R>(task: () => Promise<R>) => Promise<R | null>;
} | null>(null);
export function WorkspaceProvider({
  api,
  children,
}: {
  api: ToolboxApi;
  children: ReactNode;
}) {
  const [state, dispatch] = useReducer(workspaceReducer, {
    catalog: null,
    snapshot: null,
    error: null,
    disconnectedAt: null,
  });
  const inFlight = useRef<Promise<void> | null>(null);
  const catalogLoaded = useRef(false);
  const eventSeqs = useRef(new Map<string, number>());
  const activeWorkspace = useRef<string | null>(null);
  const refresh = useCallback(() => {
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
        if (activeWorkspace.current !== value.workspace.workspaceId)
          eventSeqs.current.clear();
        activeWorkspace.current = value.workspace.workspaceId;
        for (const operation of value.operations)
          eventSeqs.current.set(
            operation.operationId,
            Math.max(
              operation.seq,
              eventSeqs.current.get(operation.operationId) ?? -1,
            ),
          );
        dispatch({ type: "snapshot", value });
      })
      .catch((e) => {
        dispatch({ type: "error", code: errorCode(e) });
      })
      .finally(() => {
        inFlight.current = null;
      });
    inFlight.current = pending;
    return pending;
  }, [api]);
  const run = useCallback(
    async <R,>(task: () => Promise<R>): Promise<R | null> => {
      try {
        const result = await task();
        await refresh();
        return result;
      } catch (e) {
        dispatch({ type: "error", code: errorCode(e) });
        return null;
      }
    },
    [refresh],
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
      .catch(() => {});
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
  }, [api, refresh]);
  return (
    <Context.Provider value={{ api, state, refresh, run }}>
      {children}
    </Context.Provider>
  );
}
export function useWorkspace() {
  const value = useContext(Context);
  if (!value) throw Error("WorkspaceProvider required");
  return value;
}
