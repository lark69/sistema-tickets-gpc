import { invoke } from "@tauri-apps/api/core";
import { getErrorMessage } from "../utils/errors";

export async function callCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw new Error(getErrorMessage(error));
  }
}
