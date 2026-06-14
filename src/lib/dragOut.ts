import { invoke } from "@tauri-apps/api/core";

export async function dragFileOut(paths: string[]): Promise<void> {
  try {
    await invoke("start_file_drag", { paths });
  } catch (_err) {}
}
