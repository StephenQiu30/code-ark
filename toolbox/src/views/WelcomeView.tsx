import { Button } from "../components/ui/button";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  CardFooter,
} from "../components/ui/card";
import { Badge } from "../components/ui/badge";
import { StartAction } from "../components/StartAction";
import { useWorkspace } from "../lib/workspace";
import { useT, phaseText } from "../lib/i18n";
import { LaunchpadView } from "./LaunchpadView";
export function WelcomeView({
  onService,
}: {
  onService: (id: string) => void;
}) {
  const { state, api, run } = useWorkspace();
  const t = useT();
  const redis = state.snapshot?.services.find((s) => s.serviceId === "redis");
  return (
    <div className="flex flex-col gap-6">
      <div>
        <p className="eyebrow">{t("从一小步开始")}</p>
        <h1>{t("先完成一次真实连接")}</h1>
        <p className="muted">
          {t("选择服务，工作箱负责准备；连接验证结果会如实显示。")}
        </p>
      </div>
      <Card className="first-service">
        <CardHeader>
          <Badge variant="secondary">{t("推荐入门 · 轻量")}</Badge>
          <CardTitle>Redis</CardTitle>
          <CardDescription>
            {t("启动缓存服务，完成内置连接测试，再复制真实地址。")}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <ol className="steps">
            <li>{t("检测 Docker 环境")}</li>
            <li>{t("准备镜像与配置")}</li>
            <li>{t("启动并验证连接")}</li>
            <li>{t("复制客户端配置")}</li>
          </ol>
          {redis && (
            <p role="status">
              {t(phaseText[redis.phase])}
              {redis.phase === "Ready"
                ? " · 基础设施可用，业务客户端待接入"
                : ""}
            </p>
          )}
          <p className="muted">
            {t("首次拉取镜像需要联网，耗时取决于网络与缓存。")}
          </p>
        </CardContent>
        <CardFooter className="flex-wrap">
          <Button
            variant="outline"
            onClick={() => void run(() => api.detectEnvironment())}
          >
            {t("检测 Docker 环境")}
          </Button>
          {!state.snapshot?.target && (
            <Button
              variant="outline"
              onClick={() => void run(() => api.openOfficial("docker-install"))}
            >
              {t("安装 Docker")}
            </Button>
          )}
          <StartAction serviceIds={["redis"]} label="先试运行 Redis" />
          {redis?.instanceId && (
            <button className="text-link" onClick={() => onService("redis")}>
              {t("查看连接")}
            </button>
          )}
        </CardFooter>
      </Card>
      <LaunchpadView />
    </div>
  );
}
