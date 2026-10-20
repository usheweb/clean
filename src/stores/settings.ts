import { defineStore } from "pinia";
import { ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { getSettings, saveSettings } from "../api/ipc";
import type { Settings } from "../api/types";

const DEFAULTS: Settings = {
  theme: "light",
  targetRootName: "clean_c_migrated",
  targetDrive: "",
  includeCandidates: false,
  lastPromoDay: "",
};

/** 设置持久化 + 主题 */
export const useSettingsStore = defineStore("settings", () => {
  const settings = ref<Settings>({ ...DEFAULTS });
  const loaded = ref(false);

  function applyTheme() {
    const dark = settings.value.theme === "dark";
    document.documentElement.dataset.theme = dark ? "dark" : "light";
    // 原生标题栏（macOS 红绿灯那一条）不会跟着 CSS 变，深色界面下会顶一条浅灰系统栏，
    // 所以要把窗口主题一起切过去。
    getCurrentWindow()
      .setTheme(dark ? "dark" : "light")
      .catch(() => {
        /* 非 Tauri 环境或权限缺失时不影响网页内的主题 */
      });
  }

  async function load() {
    try {
      settings.value = await getSettings();
    } catch {
      // 非 Tauri 环境或首次运行：用默认值
    }
    applyTheme();
    loaded.value = true;
  }

  async function update(patch: Partial<Settings>) {
    settings.value = { ...settings.value, ...patch };
    applyTheme();
    try {
      await saveSettings(settings.value);
    } catch {
      // 保存失败不阻塞界面
    }
  }

  function toggleTheme() {
    void update({ theme: settings.value.theme === "dark" ? "light" : "dark" });
  }

  return { settings, loaded, load, update, toggleTheme, applyTheme };
});
