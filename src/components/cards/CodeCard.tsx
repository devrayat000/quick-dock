import type { ShelfItem } from "../../store/shelf";
import { CardShell } from "./CardShell";

interface CodeCardProps {
  item: ShelfItem;
  onRemove: () => void;
  onCopy: (text: string) => void;
  onHandleMouseDown: (e: React.MouseEvent) => void;
}

export function CodeCard({
  item,
  onRemove,
  onCopy,
  onHandleMouseDown,
}: CodeCardProps) {
  const body = item.text ?? "";

  return (
    <CardShell onRemove={onRemove} onHandleMouseDown={onHandleMouseDown}>
      {/* Language badge + copy button row */}
      <div className="flex items-center justify-between mb-1.5">
        <span className="text-[9px] font-mono font-semibold text-blue-400/80 uppercase tracking-widest">
          {item.language ?? "code"}
        </span>
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

      {/* Highlighted HTML from Shiki, or fallback plain pre */}
      {item.highlighted ? (
        <div
          className="shiki rounded overflow-x-auto max-h-44 text-xs"
          dangerouslySetInnerHTML={{ __html: item.highlighted }}
        />
      ) : (
        <pre className="text-xs text-white/60 font-mono overflow-x-auto max-h-44 rounded bg-black/20 p-2 leading-relaxed">
          {body.slice(0, 400)}
          {body.length > 400 && "…"}
        </pre>
      )}
    </CardShell>
  );
}
