import type { ShelfItem } from "../../store/shelf";
import { CardShell } from "./CardShell";

interface TextCardProps {
  item: ShelfItem;
  onRemove: () => void;
  onCopy: (text: string) => void;
  onHandleMouseDown: (e: React.MouseEvent) => void;
}

export function TextCard({
  item,
  onRemove,
  onCopy,
  onHandleMouseDown,
}: TextCardProps) {
  const body = item.text ?? "";
  const preview = body.slice(0, 140);
  const truncated = body.length > 140;

  return (
    <CardShell onRemove={onRemove} onHandleMouseDown={onHandleMouseDown}>
      <p className="text-xs text-white/70 leading-relaxed break-words whitespace-pre-wrap">
        {preview}
        {truncated && <span className="text-white/30">…</span>}
      </p>
      <div className="flex gap-1 mt-2">
        <button
          onClick={(e) => {
            e.stopPropagation();
            onCopy(body);
          }}
          className="text-[10px] text-white/40 hover:text-white/75 px-2 py-0.5 rounded hover:bg-white/10 transition-colors"
        >
          Copy
        </button>
      </div>
    </CardShell>
  );
}
