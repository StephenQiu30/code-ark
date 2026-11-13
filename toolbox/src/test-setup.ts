import "@testing-library/jest-dom/vitest";
import { toast } from "sonner";
import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";
afterEach(() => {
  toast.dismiss();
  cleanup();
  localStorage.clear();
  document.documentElement.classList.remove("light", "dark");
});
if (!window.matchMedia)
  window.matchMedia = (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener() {},
    removeListener() {},
    addEventListener() {},
    removeEventListener() {},
    dispatchEvent() {
      return true;
    },
  });
if (!HTMLElement.prototype.scrollIntoView)
  HTMLElement.prototype.scrollIntoView = () => {};
if (!HTMLElement.prototype.hasPointerCapture)
  HTMLElement.prototype.hasPointerCapture = () => false;
if (!HTMLElement.prototype.setPointerCapture)
  HTMLElement.prototype.setPointerCapture = () => {};
if (!HTMLElement.prototype.releasePointerCapture)
  HTMLElement.prototype.releasePointerCapture = () => {};
if (!window.requestAnimationFrame)
  window.requestAnimationFrame = (callback) =>
    window.setTimeout(() => callback(performance.now()), 0);

if (!globalThis.ResizeObserver)
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
