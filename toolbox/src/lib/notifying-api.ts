import type { ToolboxApi } from "../api/tauri";
import type { Feedback } from "./feedback";

const successes: Partial<Record<keyof ToolboxApi, string>> = {
  startServices: "启动任务已提交",
  stopServices: "停止任务已提交",
  retryOperation: "重试任务已提交",
  cancelOperation: "取消请求已提交",
  applyDraft: "配置应用任务已提交",
  executeReset: "重置任务已提交",
  exportDiagnostics: "诊断内容已导出",
  exportGuideSources: "教学源码已导出",
  authorizeDatabaseBinding: "数据库授权已保存",
  chooseWorkspace: "工作区已打开",
};
const background = new Set([
  "dashboardSnapshot",
  "serviceCatalog",
  "subscribeProgress",
]);
const reported = new WeakSet<object>();
export function consumeApiFailure(error: unknown) {
  if (typeof error !== "object" || error === null || !reported.has(error))
    return false;
  reported.delete(error);
  return true;
}

// A single notification boundary also covers callers that handle a rejection locally.
// Commands keep their original arguments, callbacks, result, and rejection semantics.
export function notifyingApi(api: ToolboxApi, feedback: Feedback): ToolboxApi {
  let detectionSequence = 0;
  return Object.fromEntries(
    Object.entries(api).map(([name, method]) => [
      name,
      async (...args: unknown[]) => {
        const detectionId =
          name === "detectEnvironment"
            ? `environment-detection:${++detectionSequence}`
            : undefined;
        if (detectionId)
          feedback.loading("正在检测 Docker 连接…", { id: detectionId });
        try {
          const result = await Reflect.apply(method, api, args);
          const message = successes[name as keyof ToolboxApi];
          if (message) feedback.success(message);
          if (detectionId) {
            if (result.environmentError || !result.target)
              feedback.error(
                { code: result.environmentError ?? "EnvironmentNotDetected" },
                { id: detectionId },
              );
            else feedback.success("Docker 连接已确认", { id: detectionId });
          }
          if (name === "validateDraft") {
            if (result.validationErrors.length)
              feedback.error({ code: "DraftValidationFailed" });
            else if (result.preflight)
              feedback.preflight(result.preflight, true);
            else feedback.error({ code: "PreflightRequired" });
          }
          return result;
        } catch (error) {
          if (!background.has(name)) {
            feedback.error(
              error,
              detectionId ? { id: detectionId } : undefined,
            );
            if (typeof error === "object" && error !== null)
              reported.add(error);
          }
          throw error;
        }
      },
    ]),
  ) as ToolboxApi;
}
