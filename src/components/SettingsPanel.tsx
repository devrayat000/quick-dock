import { useSettingsStore, type OpenMode } from "../store/settings";

interface ModeOption {
  id: OpenMode;
  label: string;
  desc: string;
}

const MODES: ModeOption[] = [
  {
    id: "hover",
    label: "Hover Peek",
    desc: "Slide cursor to screen edge — shelf peeks out, closes when you leave.",
  },
  {
    id: "tab",
    label: "Tab Strip",
    desc: "Thin strip stays at edge. Hover to expand, leave to collapse.",
  },
  {
    id: "tray",
    label: "Tray + Auto-hide",
    desc: "Open from tray icon. Closes when you click another window.",
  },
];

export function SettingsPanel({ onClose }: { onClose: () => void }) {
  const { openMode, setOpenMode } = useSettingsStore();

  return (
    <div className="flex flex-col gap-1.5 px-3 pt-2 pb-3">
      <div className="flex items-center justify-between mb-0.5">
        <span className="text-[10px] font-semibold text-white/40 uppercase tracking-wider">
          Open Mode
        </span>
        <button
          onClick={onClose}
          className="text-[10px] text-white/30 hover:text-white/60 px-2 py-0.5 rounded hover:bg-white/10 transition-colors"
        >
          done
        </button>
      </div>
      {MODES.map((m) => (
        <button
          key={m.id}
          onClick={() => void setOpenMode(m.id)}
          className={[
            "w-full text-left px-2.5 py-2 rounded-lg border transition-all",
            openMode === m.id
              ? "border-blue-400/40 bg-blue-400/10"
              : "border-white/[0.06] bg-white/[0.03] hover:bg-white/[0.06]",
          ].join(" ")}
        >
          <div className="flex items-start gap-2.5">
            <div
              className={[
                "mt-0.5 w-3 h-3 rounded-full border-2 flex-shrink-0 transition-colors",
                openMode === m.id
                  ? "border-blue-400 bg-blue-400"
                  : "border-white/30",
              ].join(" ")}
            />
            <div className="flex-1 min-w-0">
              <div className="text-xs font-medium text-white/80">{m.label}</div>
              <div className="text-[10px] text-white/40 mt-0.5 leading-relaxed">
                {m.desc}
              </div>
            </div>
          </div>
        </button>
      ))}
    </div>
  );
}
