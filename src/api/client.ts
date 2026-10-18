// 补充的底层调用：不属于"任务/数据"体系的零散命令
import { invoke } from "@tauri-apps/api/core";

export async function relaunchAsAdmin(): Promise<void> {
  return invoke<void>("relaunch_as_admin");
}
