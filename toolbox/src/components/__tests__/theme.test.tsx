import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { ModeToggle } from "../ModeToggle";
import { ThemeProvider, THEME_STORAGE_KEY } from "../../lib/theme";
import App from "../../App";
import { createTestApi } from "../../test-fixtures";

it("switches using the official menu, persists the preference, and closes after selection", async () => {
  render(
    <ThemeProvider>
      <ModeToggle />
    </ThemeProvider>,
  );
  await userEvent.click(screen.getByRole("button", { name: "切换主题" }));
  expect(screen.getAllByRole("menuitem")).toHaveLength(3);
  await userEvent.click(screen.getByRole("menuitem", { name: "深色" }));
  expect(document.documentElement).toHaveClass("dark");
  expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe("dark");
  expect(screen.queryByRole("menu")).not.toBeInTheDocument();
});

it("restores a saved theme and reacts to preferences changed in another window", async () => {
  localStorage.setItem(THEME_STORAGE_KEY, "dark");
  render(
    <ThemeProvider>
      <ModeToggle />
    </ThemeProvider>,
  );
  expect(document.documentElement).toHaveClass("dark");
  act(() =>
    window.dispatchEvent(
      new StorageEvent("storage", {
        key: THEME_STORAGE_KEY,
        newValue: "light",
      }),
    ),
  );
  expect(document.documentElement).toHaveClass("light");
});

it("follows live system changes only while the system option is selected", async () => {
  let matches = false;
  const events = new EventTarget();
  const media = {
    get matches() {
      return matches;
    },
    media: "(prefers-color-scheme: dark)",
    onchange: null,
    addListener() {},
    removeListener() {},
    addEventListener: events.addEventListener.bind(events),
    removeEventListener: events.removeEventListener.bind(events),
    dispatchEvent: events.dispatchEvent.bind(events),
  };
  const mock = vi
    .spyOn(window, "matchMedia")
    .mockReturnValue(media as MediaQueryList);
  try {
    render(
      <ThemeProvider>
        <ModeToggle />
      </ThemeProvider>,
    );
    expect(document.documentElement).toHaveClass("light");
    act(() => {
      matches = true;
      events.dispatchEvent(new Event("change"));
    });
    expect(document.documentElement).toHaveClass("dark");
    await userEvent.click(screen.getByRole("button", { name: "切换主题" }));
    await userEvent.click(screen.getByRole("menuitem", { name: "浅色" }));
    act(() => {
      matches = false;
      events.dispatchEvent(new Event("change"));
      matches = true;
      events.dispatchEvent(new Event("change"));
    });
    expect(document.documentElement).toHaveClass("light");
  } finally {
    mock.mockRestore();
  }
});

it("keeps theme switching usable when local storage cannot be written", async () => {
  const mock = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
    throw new Error("storage unavailable");
  });
  try {
    render(
      <ThemeProvider>
        <ModeToggle />
      </ThemeProvider>,
    );
    await userEvent.click(screen.getByRole("button", { name: "切换主题" }));
    await userEvent.click(screen.getByRole("menuitem", { name: "深色" }));
    expect(document.documentElement).toHaveClass("dark");
  } finally {
    mock.mockRestore();
  }
});

it("collapses the official sidebar with the keyboard without starting services", async () => {
  const api = createTestApi();
  const { container } = render(<App api={api} />);
  await screen.findByRole("button", { name: "启动 Redis" });
  await userEvent.keyboard("{Control>}b{/Control}");
  await waitFor(() =>
    expect(container.querySelector('[data-slot="sidebar"]')).toHaveAttribute(
      "data-state",
      "collapsed",
    ),
  );
  expect(screen.getByRole("button", { name: "全部服务" })).toBeInTheDocument();
  expect(api.startServices).not.toHaveBeenCalled();
});
