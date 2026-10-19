import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { getAppInfo } from "../api/ipc";
import type { AppInfo } from "../api/types";

/** 应用元信息（平台、版本、是否管理员） */
export const useAppStore = defineStore("app", () => {
  const info = ref<AppInfo | null>(null);

  const isMac = computed(() => info.value?.platform === "macos");
  const isWin = computed(() => info.value?.platform === "windows");
  const isAdmin = computed(() => info.value?.isAdmin ?? false);

  async function load() {
    try {
      const i = await getAppInfo();
      info.value = i;
      // 标题栏带上版本号，任务栏 / 窗口列表里就能直接分辨当前跑的是哪个版本
      await getCurrentWindow().setTitle(`${i.productName} v${i.version}`);
    } catch (e) {
      // 非 Tauri 环境（纯浏览器调试）时忽略；另，设标题失败不影响其余功能
      console.error("读取应用信息或设置窗口标题失败", e);
    }
  }

  return { info, isMac, isWin, isAdmin, load };
});
