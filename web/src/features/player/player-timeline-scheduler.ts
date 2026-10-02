export type PlaybackTimelineSnapshot = {
  currentTime: number;
  duration: number;
  bufferedEnd: number;
};

type FrameHandle =
  | { kind: "animation"; id: number }
  | { kind: "timeout"; id: ReturnType<typeof setTimeout> };
type FrameRequest = (callback: () => void) => FrameHandle;
type FrameCancel = (frame: FrameHandle) => void;

export type PlaybackTimelineSchedulerOptions = {
  /** Minimum interval between non-critical React timeline updates. */
  minIntervalMs?: number;
  now?: () => number;
};

const defaultRequestFrame: FrameRequest = (callback) => {
  if (typeof globalThis.requestAnimationFrame === "function") {
    return { kind: "animation", id: globalThis.requestAnimationFrame(callback) };
  }
  return { kind: "timeout", id: globalThis.setTimeout(callback, 0) };
};

const defaultCancelFrame: FrameCancel = (frame) => {
  if (frame.kind === "animation") {
    if (typeof globalThis.cancelAnimationFrame === "function") {
      globalThis.cancelAnimationFrame(frame.id);
    }
  } else {
    globalThis.clearTimeout(frame.id);
  }
};

/** Coalesces high-frequency media timeline events into one React update per frame. */
export function createPlaybackTimelineScheduler(
  onUpdate: (snapshot: PlaybackTimelineSnapshot) => void,
  requestFrame: FrameRequest = defaultRequestFrame,
  cancelFrame: FrameCancel = defaultCancelFrame,
  options: PlaybackTimelineSchedulerOptions = {},
) {
  const minIntervalMs = Math.max(0, options.minIntervalMs ?? 0);
  const now = options.now ?? (() => Date.now());
  let pending: PlaybackTimelineSnapshot | null = null;
  let frameId: FrameHandle | null = null;
  let lastFlushAt = Number.NEGATIVE_INFINITY;

  const flush = (immediate = false) => {
    frameId = null;
    const snapshot = pending;
    if (!snapshot) return;
    if (!immediate && now() - lastFlushAt < minIntervalMs) return;
    pending = null;
    lastFlushAt = now();
    onUpdate(snapshot);
  };

  return {
    schedule(snapshot: PlaybackTimelineSnapshot, immediate = false) {
      pending = snapshot;
      if (immediate) {
        if (frameId !== null) cancelFrame(frameId);
        frameId = null;
        flush(true);
        return;
      }
      if (frameId === null && now() - lastFlushAt >= minIntervalMs) {
        frameId = requestFrame(flush);
      }
    },
    dispose() {
      if (frameId !== null) cancelFrame(frameId);
      frameId = null;
      pending = null;
    },
  };
}
