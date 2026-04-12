import { useState } from "react";
import { useTauriEvents } from "../hooks/useTauriEvents";

type OverlayState = "idle" | "recording" | "transcribing" | "error";

export function Overlay() {
  const [state, setState] = useState<OverlayState>("idle");
  const [errorMsg, setErrorMsg] = useState("");

  useTauriEvents((event) => {
    if (event.type === "recording-started") {
      setState("recording");
    } else if (event.type === "recording-stopped") {
      setState("transcribing");
    } else if (event.type === "transcription-complete") {
      setState("idle");
    } else if (event.type === "transcription-error") {
      setErrorMsg(event.message);
      setState("error");
      setTimeout(() => setState("idle"), 3000);
    }
  });

  if (state === "idle") return null;

  return (
    <div className="overlay-root">
      {state === "recording" && (
        <div className="overlay-pill overlay-recording">
          <span className="recording-dot" />
          <div className="waveform">
            <span className="waveform-bar" />
            <span className="waveform-bar" />
            <span className="waveform-bar" />
            <span className="waveform-bar" />
            <span className="waveform-bar" />
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
