import userEvent from "@testing-library/user-event";
import { act, render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import App from "../../App";
import {
  createTestApi,
  fixtureCatalog,
  fixtureSnapshot,
} from "../../test-fixtures";
const unknown = {
  issues: [],
  capacity: {
    workspaceFreeBytes: null,
    daemonFreeBytes: null,
    daemonStatus: "UnknownCapacity",
  },
  requiredMemoryBytes: 134217728,
  daemonMemoryBytes: 1073741824,
  requiresCapacityConsent: true,
};
const view = {
  draftId: "draft-1",
  instanceId: "redis-1",
  baseGeneration: "applied-1",
  fields: [{ key: "PORT", value: "26379", sensitive: false, editable: true }],
  validationErrors: [],
  applyImpact: ["重建 Redis"],
  requiresRebuild: true,
  preflight: unknown,
};
async function configure(
  api: ReturnType<typeof createTestApi>,
  label = "PORT",
) {
  render(<App api={api} />);
  await userEvent.click(
    await screen.findByRole("button", { name: "全部服务" }),
  );
  await userEvent.click(
    await screen.findByRole("button", { name: "查看 Redis" }),
  );
  await userEvent.click(await screen.findByRole("tab", { name: "配置" }));
  await screen.findByLabelText(label);
  await userEvent.click(screen.getByRole("button", { name: "保存草稿并验证" }));
}
it("requires explicit single-apply unknown-capacity consent and sends it only after confirmation", async () => {
  const api = createTestApi();
  api.validateDraft = vi.fn().mockResolvedValue(view);
  await configure(api);
  expect(
    await screen.findByText("可用磁盘容量未知，继续操作前需要单独确认。"),
  ).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "预览并应用" })).toBeDisabled();
  expect(api.applyDraft).not.toHaveBeenCalled();
  await userEvent.click(
    screen.getByRole("checkbox", { name: "我理解容量未知，仅同意本次应用" }),
  );
  await userEvent.click(screen.getByRole("button", { name: "预览并应用" }));
  await userEvent.click(screen.getByRole("button", { name: "确认应用" }));
  await waitFor(() =>
    expect(api.applyDraft).toHaveBeenCalledWith("draft-1", true),
  );
});
it("clears capacity consent on every revalidation", async () => {
  const api = createTestApi();
  api.validateDraft = vi.fn().mockResolvedValue(view);
  await configure(api);
  const consent = await screen.findByRole("checkbox", {
    name: "我理解容量未知，仅同意本次应用",
  });
  await userEvent.click(consent);
  expect(screen.getByRole("button", { name: "预览并应用" })).toBeEnabled();
  await userEvent.click(screen.getByRole("button", { name: "保存草稿并验证" }));
  await waitFor(() => expect(api.validateDraft).toHaveBeenCalledTimes(2));
  expect(consent).not.toBeChecked();
  expect(screen.getByRole("button", { name: "预览并应用" })).toBeDisabled();
});
it("does not transfer draft capacity consent to a changed Docker target", async () => {
  const api = createTestApi();
  api.validateDraft = vi.fn().mockResolvedValue(view);
  api.dashboardSnapshot = vi
    .fn()
    .mockResolvedValueOnce(fixtureSnapshot)
    .mockResolvedValue({
      ...fixtureSnapshot,
      target: { ...fixtureSnapshot.target!, targetId: "changed-target" },
    });
  await configure(api);
  await userEvent.click(
    await screen.findByRole("checkbox", {
      name: "我理解容量未知，仅同意本次应用",
    }),
  );
  expect(screen.getByRole("button", { name: "预览并应用" })).toBeDisabled();
  expect(api.applyDraft).not.toHaveBeenCalled();
});
it("keeps apply disabled for preflight errors and absent preflight", async () => {
  const api = createTestApi();
  api.validateDraft = vi
    .fn()
    .mockResolvedValueOnce({
      ...view,
      preflight: {
        ...unknown,
        requiresCapacityConsent: false,
        issues: [
          { code: "PortConflict", field: "26379/tcp", severity: "error" },
        ],
      },
    })
    .mockResolvedValueOnce({ ...view, preflight: null });
  await configure(api);
  expect(await screen.findByText(/PortConflict/)).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "预览并应用" })).toBeDisabled();
  await userEvent.click(screen.getByRole("button", { name: "保存草稿并验证" }));
  await waitFor(() => expect(api.validateDraft).toHaveBeenCalledTimes(2));
  expect(screen.getByRole("button", { name: "预览并应用" })).toBeDisabled();
});

function firstApplyApi() {
  const api = createTestApi();
  const snapshot = {
    ...fixtureSnapshot,
    services: fixtureSnapshot.services.map((service) =>
      service.serviceId === "redis"
        ? {
            ...service,
            phase: "Uninitialized" as const,
            appliedGeneration: null,
          }
        : service,
    ),
  };
  api.dashboardSnapshot = vi.fn().mockResolvedValue(snapshot);
  api.serviceCatalog = vi.fn().mockResolvedValue({
    ...fixtureCatalog,
    services: fixtureCatalog.services.map((service) =>
      service.id === "redis"
        ? {
            ...service,
            ports: [
              {
                componentId: "redis",
                hostIp: "127.0.0.1",
                hostPort: 6379,
                containerPort: 6379,
                protocol: "tcp",
              },
            ],
          }
        : service,
    ),
  });
  const draft = {
    ...view,
    baseGeneration: "unapplied",
    fields: [
      {
        key: "port:redis:6379",
        value: "16379",
        sensitive: false,
        editable: true,
      },
    ],
  };
  api.configDraft = vi.fn().mockResolvedValue({ ...draft, preflight: null });
  api.validateDraft = vi.fn().mockResolvedValue(draft);
  return { api, snapshot };
}

it("applies the first configuration after validation and capacity consent without exposing internal identifiers", async () => {
  const { api } = firstApplyApi();
  await configure(api, "Redis 宿主机端口");
  expect(screen.getByLabelText("Redis 宿主机端口")).toHaveValue("16379");
  expect(api.validateDraft).toHaveBeenCalledWith({
    draftId: "draft-1",
    instanceId: "redis-1",
    baseGeneration: "unapplied",
    values: { "port:redis:6379": "16379" },
  });
  expect(screen.getByRole("button", { name: "预览并应用" })).toBeDisabled();
  await userEvent.click(
    await screen.findByRole("checkbox", {
      name: "我理解容量未知，仅同意本次应用",
    }),
  );
  expect(screen.getByRole("button", { name: "预览并应用" })).toBeEnabled();
  await userEvent.click(screen.getByRole("button", { name: "预览并应用" }));
  expect(
    screen.queryByText(/unapplied|draft-1|基础版本|port:redis:6379/),
  ).not.toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "确认应用" }));
  await waitFor(() =>
    expect(api.applyDraft).toHaveBeenCalledWith("draft-1", true),
  );
  expect(api.connectionView).not.toHaveBeenCalled();
});

it("invalidates first-apply consent if another configuration becomes applied before confirmation", async () => {
  const { api, snapshot } = firstApplyApi();
  await configure(api, "Redis 宿主机端口");
  await userEvent.click(
    await screen.findByRole("checkbox", {
      name: "我理解容量未知，仅同意本次应用",
    }),
  );
  expect(screen.getByRole("button", { name: "预览并应用" })).toBeEnabled();
  vi.mocked(api.dashboardSnapshot).mockResolvedValue({
    ...snapshot,
    services: snapshot.services.map((service) =>
      service.serviceId === "redis"
        ? { ...service, appliedGeneration: "applied-elsewhere" }
        : service,
    ),
  });
  await act(async () => {
    window.dispatchEvent(new Event("focus"));
  });
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "预览并应用" })).toBeDisabled(),
  );
  expect(
    screen.queryByRole("checkbox", {
      name: "我理解容量未知，仅同意本次应用",
      checked: true,
    }),
  ).not.toBeInTheDocument();
  expect(api.applyDraft).not.toHaveBeenCalled();
});

it("keeps the submitted draft disabled until its operation completes, then loads a fresh editable draft", async () => {
  const { api, snapshot } = firstApplyApi();
  const operation = await api.applyDraft("fixture", true);
  let current = { ...snapshot, operations: [] as typeof snapshot.operations };
  api.dashboardSnapshot = vi.fn().mockImplementation(async () => current);
  api.applyDraft = vi.fn().mockImplementation(async () => {
    current = { ...current, operations: [{ ...operation, phase: "Running" }] };
    return operation;
  });
  await configure(api, "Redis 宿主机端口");
  await userEvent.click(
    await screen.findByRole("checkbox", {
      name: "我理解容量未知，仅同意本次应用",
    }),
  );
  await userEvent.click(screen.getByRole("button", { name: "预览并应用" }));
  await userEvent.click(screen.getByRole("button", { name: "确认应用" }));
  await waitFor(() =>
    expect(screen.getByLabelText("Redis 宿主机端口")).toBeDisabled(),
  );
  expect(api.configDraft).toHaveBeenCalledTimes(1);
  current = {
    ...current,
    services: current.services.map((service) =>
      service.serviceId === "redis"
        ? { ...service, appliedGeneration: "new-config" }
        : service,
    ),
  };
  await act(async () => {
    window.dispatchEvent(new Event("focus"));
  });
  expect(api.configDraft).toHaveBeenCalledTimes(1);
  expect(screen.getByLabelText("Redis 宿主机端口")).toBeDisabled();
  vi.mocked(api.configDraft).mockResolvedValue({
    ...view,
    draftId: "draft-after-apply",
    baseGeneration: "new-config",
    preflight: null,
    fields: [
      {
        key: "port:redis:6379",
        value: "17379",
        editable: true,
        sensitive: false,
      },
    ],
  });
  current = { ...current, operations: [{ ...operation, phase: "Succeeded" }] };
  await act(async () => {
    window.dispatchEvent(new Event("focus"));
  });
  await waitFor(() =>
    expect(screen.getByLabelText("Redis 宿主机端口")).toBeEnabled(),
  );
  expect(screen.getByLabelText("Redis 宿主机端口")).toHaveValue("17379");
  expect(api.configDraft).toHaveBeenCalledTimes(2);
  await userEvent.click(screen.getByRole("tab", { name: "连接与入口" }));
  expect(screen.getByText("当前配置已应用")).toBeInTheDocument();
  expect(screen.queryByText("new-config")).not.toBeInTheDocument();
});

it("preserves local field edits when another operation changes the applied configuration", async () => {
  const { api, snapshot } = firstApplyApi();
  await configure(api, "Redis 宿主机端口");
  const field = screen.getByLabelText("Redis 宿主机端口");
  await userEvent.clear(field);
  await userEvent.type(field, "18379");
  vi.mocked(api.dashboardSnapshot).mockResolvedValue({
    ...snapshot,
    services: snapshot.services.map((service) =>
      service.serviceId === "redis"
        ? { ...service, appliedGeneration: "another-config" }
        : service,
    ),
  });
  await act(async () => {
    window.dispatchEvent(new Event("focus"));
  });
  expect(screen.getByLabelText("Redis 宿主机端口")).toHaveValue("18379");
  expect(api.configDraft).toHaveBeenCalledTimes(1);
  expect(screen.getByRole("button", { name: "预览并应用" })).toBeDisabled();
});
