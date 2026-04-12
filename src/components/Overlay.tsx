import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTauriEvents } from "../hooks/useTauriEvents";

type OverlayState = "idle" | "recording" | "transcribing" | "error";

// Each bar has a base weight that shapes the waveform pattern (taller in center)
const BAR_WEIGHTS = [0.35, 0.65, 1.0, 0.65, 0.35];
const MIN_HEIGHT = 3;
const MAX_HEIGHT = 16;

export function Overlay() {
  const [state, setState] = useState<OverlayState>("idle");
  const [errorMsg, setErrorMsg] = useState("");
  const [barHeights, setBarHeights] = useState<number[]>(BAR_WEIGHTS.map(() => MIN_HEIGHT));
  const smoothedLevel = useRef(0);

  useTauriEvents((event) => {
    if (event.type === "recording-started") {
      setState("recording");
    } else if (event.type === "recording-stopped") {
      setState("transcribing");
      setBarHeights(BAR_WEIGHTS.map(() => MIN_HEIGHT));
    } else if (event.type === "transcription-complete") {
      setState("idle");
    } else if (event.type === "transcription-error") {
      setErrorMsg(event.message);
      setState("error");
      setTimeout(() => setState("idle"), 3000);
    }
  });

  // Listen for real-time audio level events from the Rust backend
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;

    listen<{ level: number }>("audio-level", (e) => {
      if (cancelled) return;
      const raw = e.payload.level;
      // Smooth the level to avoid jittery bars
      smoothedLevel.current += (raw - smoothedLevel.current) * 0.4;
      const level = smoothedLevel.current;

      const heights = BAR_WEIGHTS.map((weight) => {
        // Add slight random jitter for organic feel
        const jitter = 1 + (Math.random() - 0.5) * 0.3;
        const h = MIN_HEIGHT + (MAX_HEIGHT - MIN_HEIGHT) * weight * level * jitter;
        return Math.max(MIN_HEIGHT, Math.min(MAX_HEIGHT, h));
      });
      setBarHeights(heights);
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  if (state === "idle") return null;

  return (
    <div className="overlay-root">
      {state === "recording" && (
        <div className="overlay-pill overlay-recording">
          <span className="recording-dot" />
          <div className="waveform">
            {barHeights.map((h, i) => (
              <span
                key={i}
                className="waveform-bar"
                style={{ height: `${h}px` }}
              />
            ))}
          </div>
        </div>
      )}
      {state === "transcribing" && (
        <div className="overlay-pill overlay-transcribing">
          <span className="spinner" />
        </div>
      )}
      {state === "error" && (
        <div className="overlay-pill overlay-error">
          <span className="overlay-text">{errorMsg}</span>
        </div>
      )}
    </div>
  );
}
