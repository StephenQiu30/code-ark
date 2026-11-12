import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import App from "../../App";
import { createTestApi, fixtureSnapshot } from "../../test-fixtures";
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
async function configure(api: ReturnType<typeof createTestApi>) {
  render(<App api={api} />);
  fireEvent.click(await screen.findByRole("button", { name: "全部服务" }));
  fireEvent.click(await screen.findByRole("button", { name: "查看 Redis" }));
  fireEvent.click(screen.getByRole("tab", { name: "配置" }));
  await screen.findByLabelText("PORT");
  fireEvent.click(screen.getByRole("button", { name: "保存草稿并验证" }));
}
it("requires explicit single-apply unknown-capacity consent and sends it only after confirmation", async () => {
  const api = createTestApi();
  api.validateDraft = vi.fn().mockResolvedValue(view);
  await configure(api);
  expect(await screen.findByText("可用磁盘容量未知")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "预览并应用" })).toBeDisabled();
  expect(api.applyDraft).not.toHaveBeenCalled();
  fireEvent.click(
    screen.getByRole("checkbox", { name: "我理解容量未知，仅同意本次应用" }),
  );
  fireEvent.click(screen.getByRole("button", { name: "预览并应用" }));
  fireEvent.click(screen.getByRole("button", { name: "确认应用" }));
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
  fireEvent.click(consent);
  expect(screen.getByRole("button", { name: "预览并应用" })).toBeEnabled();
  fireEvent.click(screen.getByRole("button", { name: "保存草稿并验证" }));
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
  fireEvent.click(
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
  fireEvent.click(screen.getByRole("button", { name: "保存草稿并验证" }));
  await waitFor(() => expect(api.validateDraft).toHaveBeenCalledTimes(2));
  expect(screen.getByRole("button", { name: "预览并应用" })).toBeDisabled();
});
