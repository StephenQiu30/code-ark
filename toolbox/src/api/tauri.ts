import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type * as T from "../types/toolbox";
const call = <R>(command: string, args?: Record<string, unknown>) =>
  invoke<R>(command, args);
// Production has one transport: trusted Tauri commands. Tests inject this boundary only.
export const tauriApi = {
  serviceCatalog: () => call<T.ServiceCatalog>("service_catalog"),
  dashboardSnapshot: () => call<T.DashboardSnapshot>("dashboard_snapshot"),
  detectEnvironment: () => call<T.DashboardSnapshot>("detect_environment"),
  preflightService: (serviceId: string) =>
    call<T.PreflightReport>("preflight_service", { serviceId }),
  startServices: (request: T.StartRequest) =>
    call<T.OperationRecord>("start_services", { request }),
  stopServices: (request: T.StopRequest) =>
    call<T.OperationRecord>("stop_services", { request }),
  retryOperation: (operationId: string) =>
    call<T.OperationRecord>("retry_operation", { operationId }),
  cancelOperation: (operationId: string) =>
    call<void>("cancel_operation", { operationId }),
  connectionView: (instanceId: string) =>
    call<T.ConnectionView>("connection_view", { instanceId }),
  revealSecret: (instanceId: string, handle: string) =>
    call<string>("reveal_secret", { instanceId, handle }),
  configDraft: (instanceId: string) =>
    call<T.ConfigDraftView>("config_draft", { instanceId }),
  validateDraft: (request: T.ConfigDraft) =>
    call<T.ConfigDraftView>("validate_draft", { request }),
  applyDraft: (draftId: string, acceptUnknownCapacity = false) =>
    call<T.OperationRecord>("apply_draft", { draftId, acceptUnknownCapacity }),
  planReset: (instanceId: string) =>
    call<T.ResetPlan>("plan_reset", { instanceId }),
  executeReset: (request: T.ResetConfirmation) =>
    call<T.OperationRecord>("execute_reset", { request }),
  previewDiagnostics: () => call<T.DiagnosticsPreview>("preview_diagnostics"),
  exportDiagnostics: (confirmation: T.DiagnosticsConfirmation) =>
    call<T.ExportResult>("export_diagnostics", { confirmation }),
  exportGuideSources: (guideId: string) =>
    call<T.ExportResult>("export_guide_sources", { guideId }),
  listGuides: () => call<T.GuideSpec[]>("list_guides"),
  readGuide: (guideId: string) => call<string>("read_guide", { guideId }),
  openOfficial: (
    linkId: "docker-install" | "docker-compose" | "tauri-prerequisites",
  ) => call<void>("open_official", { linkId }),
  openServiceConsole: (instanceId: string, endpointLabel: string) =>
    call<void>("open_service_console", { instanceId, endpointLabel }),
  chooseWorkspace: () => call<T.DashboardSnapshot>("choose_workspace"),
  databaseCandidates: () => call<T.DatabaseCandidate[]>("database_candidates"),
  verifyDatabaseBinding: (request: T.VerifyBindingRequest) =>
    call<T.DatabaseBinding>("verify_database_binding", { request }),
  bindingGrantPreview: (bindingId: string, serviceId: string) =>
    call<T.BindingGrantPreview>("binding_grant_preview", {
      bindingId,
      serviceId,
    }),
  authorizeDatabaseBinding: (previewId: string) =>
    call<T.DatabaseBinding>("authorize_database_binding", { previewId }),
  startLogStream: (
    request: T.LogStreamRequest,
    onBatch: (batch: T.LogBatch) => void,
  ) => {
    const channel = new Channel<T.LogBatch>();
    channel.onmessage = onBatch;
    return call<T.LogStreamHandle>("start_log_stream", { request, channel });
  },
  ackLogBatch: (ack: T.LogAck) => call<void>("ack_log_batch", { ack }),
  stopLogStream: (streamId: string) =>
    call<void>("stop_log_stream", { streamId }),
  subscribeLogStreamState: (onEvent: (event: T.LogStreamStateEvent) => void) =>
    listen<T.LogStreamStateEvent>("log-stream-state", (event) =>
      onEvent(event.payload),
    ),
  subscribeProgress: (onEvent: (event: T.StepProgressEvent) => void) =>
    listen<T.StepProgressEvent>("step-progress", (event) =>
      onEvent(event.payload),
    ),
};
export type ToolboxApi = typeof tauriApi;
export function errorCode(error: unknown): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string" &&
    /^[A-Za-z][A-Za-z0-9_]{0,79}$/.test(error.code)
  )
    return error.code;
  return "BackendUnavailable";
}
