import { render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import App from "../../App";
import { createTestApi, fixtureSnapshot } from "../../test-fixtures";

it("shows a configuration preset without querying connection details for an unapplied instance", async () => {
  const api = createTestApi();
  api.dashboardSnapshot = vi.fn().mockResolvedValue({
    ...fixtureSnapshot,
    services: fixtureSnapshot.services.map((service) =>
      service.serviceId === "redis"
        ? { ...service, phase: "Uninitialized", appliedGeneration: null }
        : service,
    ),
  });
  api.connectionView = vi
    .fn()
    .mockRejectedValue({ code: "NoAppliedConfiguration" });
  render(<App api={api} />);
  await screen.findByRole("button", { name: "查看 Redis" });
  expect(screen.getByText("配置预设")).toBeInTheDocument();
  expect(screen.queryByText("连接信息待读取")).not.toBeInTheDocument();
  expect(api.connectionView).not.toHaveBeenCalled();
});

it("loads the real endpoint for an applied instance", async () => {
  const api = createTestApi();
  render(<App api={api} />);
  await screen.findByRole("button", { name: "查看 Redis" });
  await screen.findByText("127.0.0.1:16379");
  expect(api.connectionView).toHaveBeenCalledWith("redis-1");
});

it.each([
  { phase: "ExternalObserve" as const, ownership: "External" },
  { phase: "RunningUnverified" as const, ownership: "ExternalConnected" },
])(
  "loads verified external endpoints without an applied generation ($ownership)",
  async ({ phase, ownership }) => {
    const api = createTestApi();
    api.dashboardSnapshot = vi.fn().mockResolvedValue({
      ...fixtureSnapshot,
      services: fixtureSnapshot.services.map((service) =>
        service.serviceId === "redis"
          ? { ...service, phase, ownership, appliedGeneration: null }
          : service,
      ),
    });
    render(<App api={api} />);
    await screen.findByRole("button", { name: "查看 Redis" });
    await waitFor(() =>
      expect(api.connectionView).toHaveBeenCalledWith("redis-1"),
    );
    await screen.findByText("127.0.0.1:16379");
  },
);
