<script setup lang="ts">
import { computed, onMounted, ref, watch, type Component } from "vue";

import PromoModal from "./components/PromoModal.vue";
import TaskBar from "./components/TaskBar.vue";
import ToastHost from "./components/ToastHost.vue";
import { useAppStore } from "./stores/app";
import { useSettingsStore } from "./stores/settings";
import { useTaskStore } from "./stores/task";
import { useUiStore } from "./stores/ui";
import AppsView from "./views/AppsView.vue";
import BigFileView from "./views/BigFileView.vue";
import CleanView from "./views/CleanView.vue";
import HelpView from "./views/HelpView.vue";
import MigrateView from "./views/MigrateView.vue";
import SettingsView from "./views/SettingsView.vue";
import logoUrl from "./assets/logo.svg";

const app = useAppStore();
const cfg = useSettingsStore();
const task = useTaskStore();
const ui = useUiStore();

interface NavItem {
  key: string;
  label: string;
  icon: string;
  winOnly?: boolean;
}

const NAV: NavItem[] = [
  { key: "clean", label: "安全清理", icon: "clean" },
  { key: "bigfile", label: "大文件", icon: "file" },
  { key: "migrate", label: "高级迁移", icon: "move", winOnly: true },
  { key: "apps", label: "已装软件", icon: "apps" },
  { key: "help", label: "使用说明", icon: "help" },
  { key: "settings", label: "设置", icon: "gear" },
];

const VIEWS: Record<string, Component> = {
  clean: CleanView,
  bigfile: BigFileView,
  migrate: MigrateView,
  apps: AppsView,
  help: HelpView,
  settings: SettingsView,
};

const ICONS: Record<string, string> = {
  clean: "M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14M10 11v6M14 11v6",
  file: "M13 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V9zM13 3v6h6",
  move: "M4 8h11M11 4l4 4-4 4M20 16H9M13 12l-4 4 4 4",
  apps: "M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z",
  help: "M5 4h11a2 2 0 0 1 2 2v14H7a2 2 0 0 0-2 2zM9 8h6M9 12h6",
  gear: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.4 15a1.6 1.6 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.6 1.6 0 0 0-1.8-.3 1.6 1.6 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.6 1.6 0 0 0-1-1.5 1.6 1.6 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.6 1.6 0 0 0 .3-1.8 1.6 1.6 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.6 1.6 0 0 0 1.5-1 1.6 1.6 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.6 1.6 0 0 0 1.8.3H9a1.6 1.6 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.6 1.6 0 0 0 1 1.5 1.6 1.6 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.6 1.6 0 0 0-.3 1.8V9a1.6 1.6 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.6 1.6 0 0 0-1.5 1z",
};

const navItems = computed(() => NAV.filter((i) => !i.winOnly || app.isWin));
const active = ref("clean");
const current = computed<Component>(() => VIEWS[active.value] ?? CleanView);
const promoVisible = ref(false);

/** 日期键（YYYY-MM-DD），字符串比较即时间序 */
function todayKey(): string {
  const d = new Date();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

/** lastPromoDay 存的是「跳过到哪天」：为空或已过期就弹 */
function shouldShowPromo(): boolean {
  const until = cfg.settings.lastPromoDay;
  if (!until) return true;
  return todayKey() > until;
}

onMounted(async () => {
  task.init();
  await Promise.all([app.load(), cfg.load()]);
  promoVisible.value = shouldShowPromo();
});

// 任务结束：汇总一条 Toast
watch(
  () => task.lastDone,
  (d) => {
    if (!d) return;
    let level: "ok" | "warn" | "error" = "ok";
    if (d.failed > 0 && d.ok === 0) level = "error";
    else if (d.stopped || d.failed > 0) level = "warn";
    ui.push(d.message || "处理完成", level);
  },
);

// macOS 没有高级迁移页，切到别的平台入口时兜底
watch(navItems, (list) => {
  if (!list.some((i) => i.key === active.value)) active.value = list[0]?.key ?? "clean";
});
</script>

<template>
  <div class="app">
    <aside class="sidebar">
      <div class="brand">
        <img class="brand-mark" :src="logoUrl" alt="唐三浪清理" draggable="false" />
        <div class="brand-text">
          <div class="brand-name">唐三浪清理</div>
          <div class="brand-sub">
            {{ app.isMac ? "macOS 垃圾清理" : "C 盘清理 · 搬家" }}
          </div>
        </div>
      </div>

      <nav class="nav">
        <button
          v-for="it in navItems"
          :key="it.key"
          class="nav-item"
          :class="{ active: active === it.key }"
          @click="active = it.key"
        >
          <svg class="nav-icon" viewBox="0 0 24 24" width="18" height="18">
            <path :d="ICONS[it.icon]" />
          </svg>
          <span>{{ it.label }}</span>
        </button>
      </nav>

      <div class="side-foot">
        <span class="ver">v{{ app.info?.version || "0.0.7" }}</span>
        <button class="theme-btn" :title="cfg.settings.theme === 'dark' ? '切换到浅色' : '切换到深色'" @click="cfg.toggleTheme()">
          <svg v-if="cfg.settings.theme === 'dark'" viewBox="0 0 24 24" width="16" height="16">
            <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" />
          </svg>
          <svg v-else viewBox="0 0 24 24" width="16" height="16">
            <path d="M12 4V2M12 22v-2M4 12H2M22 12h-2M5.6 5.6L4.2 4.2M19.8 19.8l-1.4-1.4M5.6 18.4l-1.4 1.4M19.8 4.2l-1.4 1.4M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8z" />
          </svg>
          <span>{{ cfg.settings.theme === "dark" ? "深色" : "浅色" }}</span>
        </button>
      </div>
    </aside>

    <main class="main">
      <div class="content">
        <Transition name="view" mode="out-in">
          <component :is="current" :key="active" />
        </Transition>
      </div>
      <TaskBar />
    </main>

    <ToastHost />
    <PromoModal v-model:visible="promoVisible" />
  </div>
</template>

<style scoped>
.app {
  display: flex;
  height: 100%;
  background: var(--bg);
  color: var(--text);
}

/* ---------- 侧边栏 ---------- */
.sidebar {
  width: var(--sidebar-w);
  flex: none;
  display: flex;
  flex-direction: column;
  background: var(--bg-elevated);
  border-right: 1px solid var(--border);
}
.brand {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  padding: var(--sp-4);
  border-bottom: 1px solid var(--border);
}
.brand-mark {
  width: 36px;
  height: 36px;
  flex: none;
  border-radius: var(--r);
  box-shadow: var(--shadow-sm);
  display: block;
}
.brand-text {
  min-width: 0;
}
.brand-name {
  font-size: 14px;
  font-weight: 650;
  line-height: 1.3;
}
.brand-sub {
  font-size: 11.5px;
  color: var(--text-3);
  line-height: 1.4;
}

.nav {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: var(--sp-2);
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 38px;
  padding: 0 12px;
  border-radius: var(--r);
  color: var(--text-2);
  font-size: 13.5px;
  text-align: left;
  transition: background var(--transition), color var(--transition);
}
.nav-item:hover {
  background: var(--hover);
  color: var(--text);
}
.nav-item.active {
  background: var(--brand-soft);
  color: var(--brand);
  font-weight: 600;
}
.nav-icon {
  flex: none;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.8;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.side-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-2);
  padding: var(--sp-3) var(--sp-4);
  border-top: 1px solid var(--border);
}
.ver {
  font-size: 11.5px;
  color: var(--text-3);
}
.theme-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  padding: 0 10px;
  border-radius: var(--r-full);
  background: var(--surface-3);
  color: var(--text-2);
  font-size: 12px;
  transition: background var(--transition), color var(--transition);
}
.theme-btn:hover {
  background: var(--brand-soft);
  color: var(--brand);
}
.theme-btn svg {
  fill: none;
  stroke: currentColor;
  stroke-width: 1.8;
  stroke-linecap: round;
  stroke-linejoin: round;
}

/* ---------- 主区 ---------- */
.main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.content {
  flex: 1;
  min-height: 0;
  padding: var(--sp-5) var(--sp-6);
  overflow: hidden;
}

/* 视图切换淡入 */
.view-enter-active,
.view-leave-active {
  transition: opacity 160ms ease;
}
.view-enter-from,
.view-leave-to {
  opacity: 0;
}
</style>
