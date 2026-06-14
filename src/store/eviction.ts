import { useEffect } from "react";
import { useShelfStore } from "./shelf";

const DEFAULT_TTL_MS = 15 * 60 * 1000; // 15 minutes

export function useEviction(ttlMs: number = DEFAULT_TTL_MS): void {
  const clearExpired = useShelfStore((s) => s.clearExpired);

  useEffect(() => {
    const id = setInterval(() => clearExpired(ttlMs), 60_000);
    return () => clearInterval(id);
  }, [clearExpired, ttlMs]);
}
