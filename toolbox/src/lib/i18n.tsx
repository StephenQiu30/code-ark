import { english } from "./en";
import type { ServicePhase } from "../types/toolbox";
import { createContext, useContext } from "react";
export type Language = "zh" | "en";
export const LanguageContext = createContext<Language>("zh");
const words: Record<string, string> = {
  推荐入门: "Get started",
  场景启动台: "Launchpad",
  全部服务: "All services",
  环境体检: "Environment",
  运行任务: "Operations",
  端口和日志: "Ports & logs",
  教程: "Guides",
  设置: "Settings",
  工作台: "Workspace",
  运维: "Operations",
  "先试运行 Redis": "Try Redis first",
  先完成一次真实连接: "Start with a real connection",
  选择需要的基础设施: "Choose your infrastructure",
  重新检测: "Check again",
  查看: "View",
  启动: "Start",
  停止: "Stop",
  取消: "Cancel",
  关闭: "Close",
  重试: "Retry",
  未初始化: "Not initialized",
  已停止: "Stopped",
  正在启动: "Starting",
  "运行中 · 尚未验证": "Running · unverified",
  连接已验证: "Connection verified",
  降级: "Degraded",
  失败: "Failed",
  需要处理: "Action required",
  "外部 · 仅观察": "External · observe only",
  连接与入口: "Connections",
  配置: "Configuration",
  日志: "Logs",
  依赖与数据: "Dependencies & data",
  连接工作箱后端失败: "Cannot connect to desktop backend",
  等待后端快照: "Waiting for backend snapshot",
  没有运行任务: "No operations",
  "Docker 官方安装指引": "Official Docker installation",
  预览诊断: "Preview diagnostics",
  服务: "services",
  场景: "scenes",
  已就绪: "ready",
  托管: "managed",
  "首次拉取镜像需要联网，耗时取决于网络与缓存。":
    "The first image download requires a network connection. Timing depends on network and cache.",
};
export function useT() {
  const language = useContext(LanguageContext);
  return (text: string) =>
    language === "en" ? (words[text] ?? english[text] ?? text) : text;
}
export const phaseText: Record<ServicePhase, string> = {
  Uninitialized: "未初始化",
  Stopped: "已停止",
  Starting: "正在启动",
  RunningUnverified: "运行中 · 尚未验证",
  Ready: "连接已验证",
  Degraded: "降级",
  Failed: "失败",
  AwaitingAction: "需要处理",
  ExternalObserve: "外部 · 仅观察",
};
