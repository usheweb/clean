import { defineStore } from "pinia";
import { computed, ref } from "vue";

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
      info.value = await getAppInfo();
    } catch {
      // 非 Tauri 环境（纯浏览器调试）时忽略
    }
  }

  return { info, isMac, isWin, isAdmin, load };
});
