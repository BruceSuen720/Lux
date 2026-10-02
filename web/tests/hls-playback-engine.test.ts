// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from "vitest";
import { HlsVideoEngine, canUseHls } from "../src/features/player/hls-playback-engine";

const mockHls = vi.hoisted(() => {
  type Listener = (...args: unknown[]) => void;
  const state = {
    supported: true,
    loadedSource: null as string | null,
    attachedMedia: null as HTMLVideoElement | null,
    destroyed: false,
    fatalErrorOnLoad: false,
    listeners: new Map<string, Listener>(),
  };

  class FakeHls {
    static readonly Events = {
      MANIFEST_PARSED: "manifest-parsed",
      ERROR: "error",
    };

    static isSupported() {
      return state.supported;
    }

    on(event: string, listener: Listener) {
      state.listeners.set(event, listener);
    }

    off(event: string) {
      state.listeners.delete(event);
    }

    loadSource(source: string) {
      state.loadedSource = source;
      if (state.fatalErrorOnLoad) {
        state.listeners.get(FakeHls.Events.ERROR)?.("hlsError", {
          fatal: true,
          details: "manifestLoadError",
        });
      }
    }

    attachMedia(media: HTMLVideoElement) {
      state.attachedMedia = media;
      state.listeners.get(FakeHls.Events.MANIFEST_PARSED)?.();
    }

    destroy() {
      state.destroyed = true;
    }
  }

  return { state, FakeHls };
});

vi.mock("hls.js/light", () => ({ default: mockHls.FakeHls }));

describe("HlsVideoEngine", () => {
  beforeEach(() => {
    mockHls.state.supported = true;
    mockHls.state.loadedSource = null;
    mockHls.state.attachedMedia = null;
    mockHls.state.destroyed = false;
    mockHls.state.fatalErrorOnLoad = false;
    mockHls.state.listeners.clear();
  });

  it("uses the browser's native HLS path when available", async () => {
    const video = document.createElement("video");
    vi.spyOn(video, "canPlayType").mockReturnValue("maybe");
    const load = vi.spyOn(video, "load").mockImplementation(() => undefined);
    vi.spyOn(video, "pause").mockImplementation(() => undefined);
    const engine = new HlsVideoEngine(video);

    expect(canUseHls(video)).toBe(true);
    await engine.setSource("/api/v1/playback/sessions/session-1/hls/index.m3u8");

    expect(video.src).toContain("/api/v1/playback/sessions/session-1/hls/index.m3u8");
    expect(load).toHaveBeenCalledOnce();
    expect(mockHls.state.loadedSource).toBeNull();
    engine.destroy();
    expect(video.getAttribute("src")).toBeNull();
  });

  it("uses the light HLS.js engine for server-generated HLS", async () => {
    const video = document.createElement("video");
    vi.spyOn(video, "canPlayType").mockReturnValue("");
    vi.spyOn(video, "pause").mockImplementation(() => undefined);
    vi.spyOn(video, "load").mockImplementation(() => undefined);
    const engine = new HlsVideoEngine(video);
    const source = "/api/v1/playback/sessions/session-1/hls/index.m3u8";

    await engine.setSource(source);

    expect(mockHls.state.loadedSource).toBe(source);
    expect(mockHls.state.attachedMedia).toBe(video);
    expect(mockHls.state.listeners.size).toBe(0);
    engine.destroy();
    expect(mockHls.state.destroyed).toBe(true);
  });

  it("reports fatal errors from the light HLS.js engine", async () => {
    const video = document.createElement("video");
    vi.spyOn(video, "canPlayType").mockReturnValue("");
    vi.spyOn(video, "pause").mockImplementation(() => undefined);
    vi.spyOn(video, "load").mockImplementation(() => undefined);
    mockHls.state.fatalErrorOnLoad = true;
    const engine = new HlsVideoEngine(video);

    await expect(engine.setSource("/hls/index.m3u8")).rejects.toThrow(
      "HLS 加载失败：manifestLoadError",
    );
    expect(engine.error?.message).toBe("HLS 加载失败：manifestLoadError");
    expect(mockHls.state.listeners.size).toBe(0);
    engine.destroy();
    expect(mockHls.state.destroyed).toBe(true);
  });
});
