import { useEffect, useRef } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { invoke } from "@tauri-apps/api/core";

const edgeWindow = getCurrentWebviewWindow();

export default function Edge() {
  const invoking = useRef(false);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    edgeWindow
      .onDragDropEvent((e) => {
        const p = e.payload;
        if ((p.type === "enter" || p.type === "over") && !invoking.current) {
          invoking.current = true;
          void invoke("show_shelf").finally(() => {
            invoking.current = false;
          });
        }
      })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return (
    <div style={{ width: "100vw", height: "100vh", background: "rgba(0,0,0,0.02)" }} />
  );
}
