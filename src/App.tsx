import { useEffect, useRef, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { ShelfList } from "./components/ShelfList";
import { useShelfStore } from "./store/shelf";
import { useEviction } from "./store/eviction";
import { detectLanguage, highlightCode } from "./lib/highlight";

const appWindow = getCurrentWebviewWindow();

export default function App() {
  const [visible, setVisible] = useState(false);
  const [dropping, setDropping] = useState(false);
  const { addItem, clearAll, updateItem } = useShelfStore();
  useEviction();

  const hideTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const hideShelf = useCallback(async () => {
    if (hideTimer.current) {
      clearTimeout(hideTimer.current);
      hideTimer.current = null;
    }
    setVisible(false);
    await new Promise<void>((r) => setTimeout(r, 220));
    await invoke("hide_shelf");
  }, []);

  // Schedule hide after mouse leaves — cancelled if mouse re-enters
  const scheduleHide = useCallback(() => {
    if (hideTimer.current) clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => void hideShelf(), 600);
  }, [hideShelf]);

  const cancelHide = useCallback(() => {
    if (hideTimer.current) {
      clearTimeout(hideTimer.current);
      hideTimer.current = null;
    }
  }, []);

  const handleDrop = useCallback(
    async (paths: string[]) => {
      for (const path of paths) {
        const kind = await invoke<string>("classify_path", { path });
        if (kind === "image") {
          const id = addItem({ kind: "image", path });
          invoke<string>("generate_thumbnail", { path })
            .then((thumb) => updateItem(id, { thumb: convertFileSrc(thumb) }))
            .catch(() => {});
        } else {
          addItem({ kind: "file", path, text: path.split(/[\\/]/).pop() });
        }
      }
      setDropping(false);
    },
    [addItem, updateItem],
  );

  const handlePaste = useCallback(async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (!text.trim()) return;
      if (/^https?:\/\//.test(text.trim())) {
        addItem({ kind: "url", url: text.trim(), text: text.trim() });
        return;
      }
      const lang = detectLanguage(text);
      if (lang) {
        const id = addItem({ kind: "code", text, language: lang });
        highlightCode(text, lang).then((highlighted) =>
          updateItem(id, { highlighted }),
        );
      } else {
        addItem({ kind: "text", text });
      }
    } catch (_err) {}
  }, [addItem, updateItem]);

  const handleCopy = useCallback(async (text: string) => {
    try {
      await writeText(text);
    } catch (_err) {
      await navigator.clipboard.writeText(text).catch(() => {});
    }
  }, []);

  useEffect(() => {
    let cancelDragDrop = false;
    let unlistenDragDrop: (() => void) | undefined;
    appWindow
      .onDragDropEvent((e) => {
        const p = e.payload;
        if (p.type === "drop") {
          void handleDrop(p.paths);
          // mouse still over shelf after drop — onMouseLeave handles future hide
        } else if (p.type === "enter") {
          cancelHide();
          setDropping(true);
        } else if (p.type === "leave") {
          // drag left without dropping — start hide countdown
          setDropping(false);
          scheduleHide();
        }
      })
      .then((fn) => {
        if (cancelDragDrop) fn();
        else unlistenDragDrop = fn;
      });

    const listeners = [
      listen("quickdock://shelf-show", () => {
        cancelHide();
        setVisible(true);
      }),
      listen("quickdock://clear-all", () => clearAll()),
    ];

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") void hideShelf();
      if ((e.ctrlKey || e.metaKey) && e.key === "v") {
        e.preventDefault();
        void handlePaste();
      }
    };
    window.addEventListener("keydown", onKey);

    return () => {
      cancelDragDrop = true;
      unlistenDragDrop?.();
      listeners.forEach((p) => p.then((f) => f()));
      window.removeEventListener("keydown", onKey);
      if (hideTimer.current) clearTimeout(hideTimer.current);
    };
  }, [handleDrop, handlePaste, hideShelf, scheduleHide, cancelHide, clearAll]);

  return (
    <div
      className="w-full h-screen flex flex-col overflow-hidden bg-transparent"
      onMouseEnter={cancelHide}
      onMouseLeave={scheduleHide}
    >
      <div
        className={[
          "flex flex-col h-full w-full",
          "transition-transform duration-200 ease-out",
          visible ? "translate-x-0" : "translate-x-full",
        ].join(" ")}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-3 py-2 border-b border-white/10 shrink-0">
          <span className="text-sm font-semibold text-white/80 tracking-wide">
            QuickDock
          </span>
          <div className="flex gap-1">
            <button
              onClick={() => clearAll()}
              className="text-xs text-white/40 hover:text-white/70 px-2 py-1 rounded hover:bg-white/10 transition-colors"
            >
              Clear
            </button>
            <button
              onClick={() => void hideShelf()}
              className="text-xs text-white/40 hover:text-white/70 px-2 py-1 rounded hover:bg-white/10 transition-colors"
            >
              ✕
            </button>
          </div>
        </div>

        {/* Drop-zone indicator */}
        {dropping && (
          <div className="mx-2 mt-2 rounded-lg border-2 border-dashed border-blue-400/60 bg-blue-400/8 py-3 text-center text-xs text-blue-300/80 flex-shrink-0">
            Drop files here
          </div>
        )}

        {/* Scrollable item list */}
        <div className="flex-1 overflow-y-auto overflow-x-hidden px-2 py-2 min-h-0">
          <ShelfList onCopy={handleCopy} />
        </div>

        {/* Footer paste button */}
        <div className="px-2 py-2 border-t border-white/10 shrink-0">
          <button
            onClick={() => void handlePaste()}
            className="w-full text-xs text-white/40 hover:text-white/70 py-1.5 rounded hover:bg-white/10 transition-colors flex items-center justify-center gap-1.5"
          >
            <span>⌨</span>
            <span>Paste clipboard (Ctrl+V)</span>
          </button>
        </div>
      </div>
    </div>
  );
}
