import type { ShelfItem } from "../../store/shelf";
import { dragFileOut } from "../../lib/dragOut";
import { CardShell } from "./CardShell";

interface ImageCardProps {
  item: ShelfItem;
  onRemove: () => void;
  onHandleMouseDown: (e: React.MouseEvent) => void;
}

export function ImageCard({ item, onRemove, onHandleMouseDown }: ImageCardProps) {
  const filename = item.path?.split(/[\\/]/).pop() ?? "image";

  return (
    <CardShell onRemove={onRemove} onHandleMouseDown={onHandleMouseDown}>
      <div
        className="flex items-center gap-2.5 cursor-grab active:cursor-grabbing"
        onMouseDown={(e) => {
          if ((e.target as HTMLElement).closest("button")) return;
          if (item.path) void dragFileOut([item.path]);
        }}
      >
        <div className="flex-shrink-0 w-14 h-14 rounded-lg overflow-hidden bg-white/10 border border-white/10">
          {item.thumb ? (
            <img
              src={item.thumb}
              alt={filename}
              className="w-full h-full object-cover"
              draggable={false}
            />
          ) : (
            <div className="w-full h-full flex items-center justify-center text-white/25 text-2xl">
              🖼
            </div>
          )}
        </div>
        <div className="flex-1 min-w-0">
          <div className="text-xs font-medium text-white/80 truncate">
            {filename}
          </div>
          <div className="flex gap-1 mt-1.5">
            <button
              onClick={(e) => {
                e.stopPropagation();
                item.path && void dragFileOut([item.path]);
              }}
              className="text-[10px] text-white/40 hover:text-white/75 px-2 py-0.5 rounded hover:bg-white/10 transition-colors"
            >
              Drag out ↗
            </button>
          </div>
        </div>
      </div>
    </CardShell>
  );
}
