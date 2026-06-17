import type { ReactNode } from "react";

interface CardShellProps {
  children: ReactNode;
  onRemove: () => void;
  onHandleMouseDown: (e: React.MouseEvent) => void;
  className?: string;
}

export function CardShell({
  children,
  onRemove,
  onHandleMouseDown,
  className,
}: CardShellProps) {
  return (
    <div
      className={[
        "group relative rounded-xl border border-white/[0.1] bg-white/[0.06]",
        "px-3 pt-2.5 pb-2.5 backdrop-blur-[2px]",
        className ?? "",
      ].join(" ")}
    >
      {/* Drag handle */}
      <div
        onMouseDown={onHandleMouseDown}
        className="absolute left-1.5 top-1/2 -translate-y-1/2 cursor-grab active:cursor-grabbing text-white/20 hover:text-white/50 px-0.5 select-none text-sm transition-colors"
        title="Drag to reorder"
      >
        ⠿
      </div>

      {/* Content area (offset for handle) */}
      <div className="ml-4">{children}</div>

      {/* Remove button – visible on hover */}
      <button
        onClick={(e) => {
          e.stopPropagation();
          onRemove();
        }}
        className="absolute top-1.5 right-1.5 w-4 h-4 flex items-center justify-center rounded text-[10px] text-white/20 hover:text-white/80 hover:bg-white/10 opacity-0 group-hover:opacity-100 transition-opacity"
        aria-label="Remove"
      >
        ✕
      </button>
    </div>
  );
}
