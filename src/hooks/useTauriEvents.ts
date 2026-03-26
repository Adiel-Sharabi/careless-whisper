import { useEffect, useEffectEvent } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type AppEvent =
  | { type: "recording-started" }
  | { type: "recording-stopped" }
  | { type: "transcription-complete"; text: string }
  | { type: "transcription-error"; message: string }
  | { type: "download-progress"; model: string; percent: number }
  | { type: "hotkey-start" }
  | { type: "hotkey-stop" }
  | { type: "backend-error"; message: string };

type Handler = (event: AppEvent) => void;

export function useTauriEvents(handler: Handler) {
  const onEvent = useEffectEvent(handler);

  useEffect(() => {
    let cancelled = false;
    const unlisteners: UnlistenFn[] = [];

    const setup = async () => {
      const subscriptions = await Promise.all([
        listen("recording-started", () => onEvent({ type: "recording-started" })),
        listen("recording-stopped", () => onEvent({ type: "recording-stopped" })),
        listen<{ text: string }>("transcription-complete", (e) =>
          onEvent({ type: "transcription-complete", text: e.payload.text })
        ),
        listen<{ message: string }>("transcription-error", (e) =>
          onEvent({ type: "transcription-error", message: e.payload.message })
        ),
        listen<{ model: string; percent: number }>("download-progress", (e) =>
          onEvent({
            type: "download-progress",
            model: e.payload.model,
            percent: e.payload.percent,
          })
        ),
        listen("hotkey-start", () => onEvent({ type: "hotkey-start" })),
        listen("hotkey-stop", () => onEvent({ type: "hotkey-stop" })),
        listen<{ message: string }>("backend-error", (e) =>
          onEvent({ type: "backend-error", message: e.payload.message })
        ),
      ]);

      if (cancelled) {
        subscriptions.forEach((unsubscribe) => unsubscribe());
        return;
      }

      unlisteners.push(...subscriptions);
    };

    void setup();

    return () => {
      cancelled = true;
      unlisteners.forEach((unsubscribe) => unsubscribe());
    };
  }, [onEvent]);
}
