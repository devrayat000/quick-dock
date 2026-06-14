import { create } from "zustand";
import { nanoid } from "nanoid";

export type ItemKind = "file" | "image" | "code" | "text" | "url";

export interface ShelfItem {
  id: string;
  kind: ItemKind;
  path?: string;
  text?: string;
  url?: string;
  thumb?: string;
  language?: string;
  highlighted?: string;
  createdAt: number;
}

interface ShelfStore {
  items: ShelfItem[];
  addItem: (item: Omit<ShelfItem, "id" | "createdAt">) => string;
  removeItem: (id: string) => void;
  reorderItems: (fromIdx: number, toIdx: number) => void;
  clearAll: () => void;
  updateItem: (id: string, updates: Partial<ShelfItem>) => void;
  clearExpired: (ttlMs: number) => void;
}

export const useShelfStore = create<ShelfStore>((set) => ({
  items: [],

  addItem: (item) => {
    const id = nanoid();
    set((s) => ({
      items: [...s.items, { ...item, id, createdAt: Date.now() }],
    }));
    return id;
  },

  removeItem: (id) =>
    set((s) => ({ items: s.items.filter((i) => i.id !== id) })),

  reorderItems: (fromIdx, toIdx) =>
    set((s) => {
      const next = [...s.items];
      const [moved] = next.splice(fromIdx, 1);
      next.splice(toIdx, 0, moved);
      return { items: next };
    }),

  clearAll: () => set({ items: [] }),

  updateItem: (id, updates) =>
    set((s) => ({
      items: s.items.map((i) => (i.id === id ? { ...i, ...updates } : i)),
    })),

  clearExpired: (ttlMs) =>
    set((s) => ({
      items: s.items.filter((i) => Date.now() - i.createdAt < ttlMs),
    })),
}));
