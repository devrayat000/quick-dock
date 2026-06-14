import { useRef, useState, useCallback, useEffect } from "react";
import { useShelfStore } from "../store/shelf";
import { ItemCard } from "./cards/ItemCard";

interface ShelfListProps {
  onCopy: (text: string) => void;
}

const CARD_HEIGHT = 86; // estimated avg card height in px
const DRAG_THRESHOLD = 4; // px movement before drag starts

export function ShelfList({ onCopy }: ShelfListProps) {
  const items = useShelfStore((s) => s.items);
  const { reorderItems, removeItem } = useShelfStore();

  const dragRef = useRef<{
    idx: number;
    startY: number;
    started: boolean;
  } | null>(null);

  const [draggingIdx, setDraggingIdx] = useState<number | null>(null);
  const [dragOffset, setDragOffset] = useState(0);

  const beginDrag = useCallback((idx: number, startY: number) => {
    dragRef.current = { idx, startY, started: false };
  }, []);

  const onMouseMove = useCallback((e: MouseEvent) => {
    if (!dragRef.current) return;
    const offset = e.clientY - dragRef.current.startY;
    if (!dragRef.current.started && Math.abs(offset) > DRAG_THRESHOLD) {
      dragRef.current.started = true;
      setDraggingIdx(dragRef.current.idx);
    }
    if (dragRef.current.started) {
      setDragOffset(offset);
    }
  }, []);

  const onMouseUp = useCallback(
    (e: MouseEvent) => {
      if (!dragRef.current) return;
      if (dragRef.current.started) {
        const { idx } = dragRef.current;
        const offset = e.clientY - dragRef.current.startY;
        const delta = Math.round(offset / CARD_HEIGHT);
        const target = Math.max(0, Math.min(items.length - 1, idx + delta));
        if (target !== idx) reorderItems(idx, target);
      }
      dragRef.current = null;
      setDraggingIdx(null);
      setDragOffset(0);
    },
    [items.length, reorderItems],
  );

  useEffect(() => {
    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
    return () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    };
  }, [onMouseMove, onMouseUp]);

  if (items.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-white/25 text-xs gap-3 select-none">
        <span className="text-4xl opacity-60">📦</span>
        <span className="font-medium">Shelf is empty</span>
        <span className="text-center text-white/20 leading-relaxed">
          Drag files here, paste text,
          <br />
          or hover the screen edge while dragging
        </span>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {items.map((item, idx) => {
        const isDragging = draggingIdx === idx;
        return (
          <div
            key={item.id}
            className={isDragging ? "cursor-grabbing" : ""}
            style={
              isDragging
                ? {
                    transform: `translateY(${dragOffset}px)`,
                    zIndex: 50,
                    position: "relative",
                    opacity: 0.88,
                    filter: "drop-shadow(0 4px 12px rgba(0,0,0,0.4))",
                  }
                : undefined
            }
          >
            <ItemCard
              item={item}
              onRemove={() => removeItem(item.id)}
              onCopy={onCopy}
              onHandleMouseDown={(e) => beginDrag(idx, e.clientY)}
            />
          </div>
        );
      })}
    </div>
  );
}
