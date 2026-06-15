import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export type OpenMode = "hover" | "tab" | "tray";

interface SettingsStore {
  openMode: OpenMode;
  loaded: boolean;
  load: () => Promise<void>;
  setOpenMode: (mode: OpenMode) => Promise<void>;
}

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  openMode: "hover",
  loaded: false,

  load: async () => {
    if (get().loaded) return;
    try {
      const s = await invoke<{ open_mode: string }>("get_settings");
      set({ openMode: (s.open_mode as OpenMode) || "hover", loaded: true });
    } catch {
      set({ loaded: true });
    }
  },

  setOpenMode: async (mode: OpenMode) => {
    set({ openMode: mode });
    try {
      await invoke("set_open_mode", { mode });
    } catch {
      // ignore
    }
  },
}));
