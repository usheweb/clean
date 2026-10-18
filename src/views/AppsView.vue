<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";

import { onDataDone, onDataRow, openPath, revealPath, startAppsScan, type InstalledApp } from "../api/ipc";
import { useAppStore } from "../stores/app";
import { useTaskStore } from "../stores/task";
import { useUiStore } from "../stores/ui";
import { fmtCount, fmtSize } from "../utils/format";

const app = useAppStore();
const task = useTaskStore();
const ui = useUiStore();

const rows = ref<InstalledApp[]>([]);
const scanned = ref(false);
const scanning = ref(false);

let unRow: (() => void) | null = null;
let unDone: (() => void) | null = null;

onMounted(async () => {
  unRow = await onDataRow<InstalledApp>("installed_apps", (r) => rows.value.push(r.row));
  unDone = await onDataDone("installed_apps", () => {
    scanning.value = false;
    scanned.value = true;
  });
  task.init();
});

onBeforeUnmount(() => {
  unRow?.();
  unDone?.();
});

const sorted = computed(() => [...rows.value].sort((a, b) => b.bytes - a.bytes));
const total = computed(() => sorted.value.reduce((s, x) => s + x.bytes, 0));

async function scan() {
  rows.value = [];
  scanning.value = true;
  await task.start(() => startAppsScan());
}

async function reveal(path: string) {
  try {
    await revealPath(path);
  } catch (e) {
    ui.push(String(e), "error");
  }
}

async function openApp(path: string) {
  try {
    await openPath(path);
  } catch (e) {
    ui.push(String(e), "error");
  }
}
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div>
        <h1>{{ app.isMac ? "已装应用" : "C 盘已装软件" }}</h1>
        <p class="sub">
          这里列的是<b>程序本体</b>，既不能迁移也不能直接删：搬去别的盘会让软件无法修复 / 更新，手动删会让软件损坏。
          要卸载请走系统自带的卸载入口。本页只做盘点。
        </p>
      </div>
      <button class="btn btn-primary" :disabled="task.running" @click="scan">
        {{ scanned ? "重新扫描" : "开始盘点" }}
      </button>
    </header>

    <div class="toolbar card">
      <span class="text-2">
        {{ app.isMac ? "来自 /Applications 与 ~/Applications" : "来自注册表卸载项（只读，不联网）" }}
      </span>
      <span class="grow" />
      <span class="summary">共 {{ fmtCount(sorted.length) }} 个 · 合计 {{ fmtSize(total) }}</span>
    </div>

    <div v-if="!sorted.length" class="empty card">
      <span v-if="scanning" class="spinner spinner-lg" />
      <p class="empty-title">{{ scanning ? "正在盘点…" : scanned ? "没有找到可盘点的软件" : "还没有扫描结果" }}</p>
      <p class="text-2">盘点过程只读，任何文件都不会被改动。</p>
    </div>

    <div v-else class="table card">
      <div class="thead">
        <span class="c-name">名称</span>
        <span class="c-ver">版本</span>
        <span class="c-size">体积</span>
        <span class="c-loc">安装位置</span>
        <span class="c-act">操作</span>
      </div>
      <div class="tbody">
        <div v-for="a in sorted" :key="a.name + a.location" class="tr">
          <span class="c-name ellipsis" :title="a.name">{{ a.name }}</span>
          <span class="c-ver text-3">{{ a.version || "—" }}</span>
          <span class="c-size">{{ fmtSize(a.bytes) }}</span>
          <span class="c-loc mono ellipsis" :title="a.location">{{ a.location }}</span>
          <span class="c-act">
            <button class="btn btn-ghost btn-sm" @click="reveal(a.location)">打开位置</button>
            <button class="btn btn-ghost btn-sm" @click="openApp(a.location)">打开</button>
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: var(--sp-4);
  height: 100%;
  min-height: 0;
}
.page-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--sp-4);
}
.page-head h1 {
  font-size: 20px;
  font-weight: 650;
}
.sub {
  margin-top: 4px;
  color: var(--text-2);
  font-size: 13px;
  max-width: 680px;
}
.toolbar {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  padding: var(--sp-3) var(--sp-4);
}
.grow {
  flex: 1;
}
.summary {
  font-size: 13px;
  color: var(--text-2);
}
.empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-3);
  padding: var(--sp-8);
  text-align: center;
}
.empty-title {
  font-weight: 600;
  font-size: 15px;
}
.table {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.thead,
.tr {
  display: grid;
  grid-template-columns: 1.2fr 100px 110px 1.6fr 170px;
  gap: var(--sp-3);
  align-items: center;
  padding: var(--sp-3) var(--sp-4);
}
.thead {
  font-size: 12px;
  color: var(--text-3);
  border-bottom: 1px solid var(--border);
  background: var(--surface-2);
  flex: none;
}
.tbody {
  overflow-y: auto;
}
.tr {
  border-bottom: 1px solid var(--border);
  font-size: 13px;
}
.tr:last-child {
  border-bottom: none;
}
.tr:hover {
  background: var(--hover);
}
.c-size {
  font-variant-numeric: tabular-nums;
  font-weight: 600;
}
.c-loc {
  font-size: 11.5px;
  color: var(--text-3);
}
.c-act {
  display: flex;
  gap: 6px;
  justify-content: flex-end;
}
</style>
