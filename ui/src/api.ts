import { invoke } from "@tauri-apps/api/tauri";
import { SystemStatus } from "./types";

export async function getStatus(): Promise<SystemStatus> {
  return await invoke("get_status_update");
}

export async function submitJob(gdsiiPath: string) {
  return await invoke("submit_job", { gdsiiPath });
}

export async function getSystemHealth(): Promise<"RUNNING" | "HALTED"> {
  return await invoke("get_system_health");
}
