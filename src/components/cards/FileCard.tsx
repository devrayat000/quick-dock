import type { ShelfItem } from "../../store/shelf";
import { dragFileOut } from "../../lib/dragOut";
import { CardShell } from "./CardShell";

interface FileCardProps {
  item: ShelfItem;
  onRemove: () => void;
  onCopy: (text: string) => void;
  onHandleMouseDown: (e: React.MouseEvent) => void;
}

export function FileCard({
  item,
  onRemove,
  onCopy,
  onHandleMouseDown,
}: FileCardProps) {
  const filename = item.path?.split(/[\\/]/).pop() ?? "unknown";
  const ext = filename.includes(".")
    ? filename.split(".").pop()?.toUpperCase() ?? ""
    : "";

  return (
    <CardShell onRemove={onRemove} onHandleMouseDown={onHandleMouseDown}>
      <div
        className="flex items-center gap-2.5 min-w-0 cursor-grab active:cursor-grabbing"
        onMouseDown={(e) => {
          if ((e.target as HTMLElement).closest("button")) return;
          if (item.path) void dragFileOut([item.path]);
        }}
      >
        <div className="flex-shrink-0 w-9 h-9 rounded-lg bg-white/10 flex items-center justify-center text-[9px] font-bold text-white/50 leading-none tracking-tight">
          {ext || "FILE"}
        </div>
        <div className="flex-1 min-w-0">
          <div className="text-xs font-medium text-white/80 truncate leading-tight">
            {filename}
          </div>
          <div className="text-[10px] text-white/35 truncate mt-0.5">
            {item.path}
          </div>
        </div>
      </div>
      <div className="flex gap-1 mt-2">
        <Btn
          label="Copy path"
          onClick={() => item.path && onCopy(item.path)}
        />
        <Btn
          label="Drag out ↗"
          onClick={() => item.path && void dragFileOut([item.path])}
        />
      </div>
    </CardShell>
  );
}

function Btn({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <button
      onClick={(e) => {
        e.stopPropagation();
        onClick();
      }}
      className="text-[10px] text-white/40 hover:text-white/75 px-2 py-0.5 rounded hover:bg-white/10 transition-colors"
    >
      {label}
    </button>
  );
}
