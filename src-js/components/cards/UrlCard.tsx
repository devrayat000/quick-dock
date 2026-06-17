import { openUrl } from "@tauri-apps/plugin-opener";
import type { ShelfItem } from "../../store/shelf";
import { CardShell } from "./CardShell";

interface UrlCardProps {
  item: ShelfItem;
  onRemove: () => void;
  onHandleMouseDown: (e: React.MouseEvent) => void;
}

export function UrlCard({ item, onRemove, onHandleMouseDown }: UrlCardProps) {
  const url = item.url ?? item.text ?? "";
  const display = url.replace(/^https?:\/\//, "").slice(0, 55);

  return (
    <CardShell onRemove={onRemove} onHandleMouseDown={onHandleMouseDown}>
      <div className="flex items-start gap-2 min-w-0">
        <span className="text-white/40 text-sm flex-shrink-0 mt-0.5">🔗</span>
        <span className="text-xs text-blue-300/80 truncate flex-1 leading-relaxed">
          {display}
        </span>
      </div>
      <div className="flex gap-1 mt-2">
        <button
          onClick={(e) => {
            e.stopPropagation();
            void openUrl(url);
          }}
          className="text-[10px] text-white/40 hover:text-white/75 px-2 py-0.5 rounded hover:bg-white/10 transition-colors"
        >
          Open
        </button>
        <button
          onClick={(e) => {
            e.stopPropagation();
            void navigator.clipboard.writeText(url);
          }}
          className="text-[10px] text-white/40 hover:text-white/75 px-2 py-0.5 rounded hover:bg-white/10 transition-colors"
        >
          Copy
        </button>
      </div>
    </CardShell>
  );
}
