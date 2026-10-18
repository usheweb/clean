<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from "vue";

import ConfirmDialog from "../components/ConfirmDialog.vue";
import {
  listCleanItems,
  onDataDone,
  onDataRow,
  startCleanRun,
  startCleanScan,
  type CleanItemMeta,
  type CleanTarget,
} from "../api/ipc";
import { useAppStore } from "../stores/app";
import { useTaskStore } from "../stores/task";
import { useUiStore } from "../stores/ui";
import { fmtCount, fmtSize } from "../utils/format";

const app = useAppStore();
const task = useTaskStore();
const ui = useUiStore();

const metas = ref<CleanItemMeta[]>([]);
const targets = ref<CleanTarget[]>([]);
const selected = reactive(new Set<string>());
const permanent = ref(false);
const confirmOpen = ref(false);
const scanning = ref(false);

let unRow: (() => void) | null = null;
let unDone: (() => void) | null = null;

onMounted(async () => {
  try {
    metas.value = await listCleanItems();
  } catch {
    metas.value = [];
  }
  unRow = await onDataRow<CleanTarget>("clean_targets", (r) => {
    targets.value.push(r.row);
    if (r.row.level === "green" && r.row.deletable) selected.add(r.row.id);
  });
  unDone = await onDataDone("clean_targets", () => {
    scanning.value = false;
  });
  task.init();
});

onBeforeUnmount(() => {
  unRow?.();
  unDone?.();
});

const groups = computed(() => {
  const order = ["系统", "开发工具", "浏览器", "编辑器", "聊天办公", "其他"];
  const map = new Map<string, CleanTarget[]>();
  for (const t of targets.value) {
    const list = map.get(t.category) || [];
    list.push(t);
    map.set(t.category, list);
  }
  return [...map.entries()]
    .sort((a, b) => {
      const ia = order.indexOf(a[0]);
      const ib = order.indexOf(b[0]);
      return (ia < 0 ? 99 : ia) - (ib < 0 ? 99 : ib);
    })
    .map(([name, items]) => ({
      name,
      items: items.sort((a, b) => b.bytes - a.bytes),
      bytes: items.reduce((s, x) => s + x.bytes, 0),
    }))
    .filter((g) => g.items.length > 0);
});

const selectedTargets = computed(() => targets.value.filter((t) => selected.has(t.id)));
const selectedBytes = computed(() => selectedTargets.value.reduce((s, t) => s + t.bytes, 0));
const selectedAdmin = computed(() => selectedTargets.value.filter((t) => t.admin).length);
const hasScan = computed(() => targets.value.length > 0);

async function scan() {
  targets.value = [];
  selected.clear();
  scanning.value = true;
  const id = await task.start(() => startCleanScan());
  if (!id) scanning.value = false;
}

function toggle(id: string) {
  if (selected.has(id)) selected.delete(id);
  else selected.add(id);
}

function selectOnlyGreen() {
  selected.clear();
  for (const t of targets.value) if (t.level === "green") selected.add(t.id);
}

function selectAll() {
  selected.clear();
  for (const t of targets.value) selected.add(t.id);
}

function selectNone() {
  selected.clear();
}

function askClean() {
  if (selectedTargets.value.length === 0) {
    ui.push("请先勾选要清理的条目", "warn");
    return;
  }
  confirmOpen.value = true;
}

const confirmMessage = computed(() => {
  const n = selectedTargets.value.length;
  const lines = [
    `将处理 ${n} 个条目，合计约 ${fmtSize(selectedBytes.value)}。`,
    permanent.value
      ? "模式：直接删除 —— 不进回收站/废纸篓，删除后不可恢复。"
      : "模式：移入回收站/废纸篓 —— 随时可以还原。",
  ];
  if (selectedAdmin.value > 0 && !app.isAdmin) {
    lines.push(`其中 ${selectedAdmin.value} 个条目需要管理员权限，当前不是管理员，这些条目会被跳过或部分失败。`);
  }
  return lines.join("\n");
});

async function doClean() {
  confirmOpen.value = false;
  const list = selectedTargets.value;
  await task.start(() => startCleanRun(list, permanent.value));
}
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div>
        <h1>安全清理</h1>
        <p class="sub">
          内置白名单，只清理缓存 / 日志等可安全重建的目录；删除默认进回收站（macOS 为废纸篓），可随时还原。
        </p>
      </div>
      <div class="head-actions">
        <button class="btn btn-ghost" :disabled="task.running" @click="scan">
          {{ hasScan ? "重新扫描" : "开始扫描" }}
        </button>
        <button class="btn btn-primary" :disabled="task.running || !hasScan" @click="askClean">
          清理选中项
        </button>
      </div>
    </header>

    <div class="toolbar card">
      <label class="switch">
        <input v-model="permanent" type="checkbox" class="checkbox" />
        <span>直接删除（不进回收站 / 废纸篓，不可恢复）</span>
      </label>
      <span class="grow" />
      <button class="btn btn-ghost btn-sm" :disabled="!hasScan" @click="selectOnlyGreen">只勾绿色</button>
      <button class="btn btn-ghost btn-sm" :disabled="!hasScan" @click="selectAll">全选</button>
      <button class="btn btn-ghost btn-sm" :disabled="!hasScan" @click="selectNone">全不选</button>
      <span class="summary">
        已选 <b>{{ selectedTargets.length }}</b> 项 · <b>{{ fmtSize(selectedBytes) }}</b>
      </span>
    </div>

    <div v-if="scanning && !hasScan" class="empty card">
      <span class="spinner spinner-lg" />
      <p>正在扫描可清理项，请稍候…</p>
    </div>

    <div v-else-if="!hasScan" class="empty card">
      <p class="empty-title">还没有扫描结果</p>
      <p class="text-2">点右上角【开始扫描】，会列出当前系统中可安全清理的缓存与日志。</p>
      <div class="meta-preview">
        <div v-for="m in metas" :key="m.key" class="meta-chip" :class="`m-${m.level}`">
          {{ m.name }}
        </div>
      </div>
    </div>

    <div v-else class="groups">
      <section v-for="g in groups" :key="g.name" class="group">
        <div class="group-head">
          <span class="group-name">{{ g.name }}</span>
          <span class="text-3">{{ g.items.length }} 项 · {{ fmtSize(g.bytes) }}</span>
        </div>
        <div class="rows card">
          <label v-for="t in g.items" :key="t.id" class="row" :class="{ sel: selected.has(t.id) }">
            <input
              class="checkbox"
              type="checkbox"
              :checked="selected.has(t.id)"
              @change="toggle(t.id)"
            />
            <div class="row-main">
              <div class="row-title">
                <span>{{ t.itemName }}</span>
                <span class="tag" :class="t.level === 'green' ? 'tag-green' : 'tag-yellow'">
                  {{ t.level === "green" ? "安全" : "建议先关软件" }}
                </span>
                <span v-if="t.admin" class="tag tag-blue">需管理员</span>
                <span v-if="!t.deletable" class="tag tag-gray">仅可直接删除</span>
              </div>
              <div class="row-path mono ellipsis" :title="t.path">{{ t.path }}</div>
              <div class="row-desc text-3">{{ t.desc }}</div>
            </div>
            <div class="row-size">
              <b>{{ fmtSize(t.bytes) }}</b>
              <span class="text-3">{{ fmtCount(t.files) }} 个文件</span>
            </div>
          </label>
        </div>
      </section>
    </div>

    <ConfirmDialog
      v-model:visible="confirmOpen"
      :title="permanent ? '确认直接删除？' : '确认清理？'"
      :message="confirmMessage"
      :confirm-text="permanent ? '直接删除' : '开始清理'"
      :danger="permanent"
      @cancel="confirmOpen = false"
      @confirm="doClean"
    />
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
  max-width: 640px;
}
.head-actions {
  display: flex;
  gap: var(--sp-2);
  flex: none;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  padding: var(--sp-3) var(--sp-4);
}
.switch {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  font-size: 13px;
  cursor: pointer;
}
.grow {
  flex: 1;
}
.summary {
  font-size: 13px;
  color: var(--text-2);
  margin-left: var(--sp-2);
}
.summary b {
  color: var(--brand);
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
.meta-preview {
  display: flex;
  flex-wrap: wrap;
  gap: var(--sp-2);
  justify-content: center;
  max-width: 720px;
  margin-top: var(--sp-4);
}
.meta-chip {
  font-size: 12px;
  padding: 3px 10px;
  border-radius: var(--r-full);
  background: var(--surface-3);
  color: var(--text-3);
}
.meta-chip.m-green {
  background: var(--green-soft);
  color: var(--green);
}
.meta-chip.m-yellow {
  background: var(--yellow-soft);
  color: var(--yellow);
}

.groups {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: var(--sp-4);
  padding-bottom: var(--sp-2);
}
.group-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  padding: 0 var(--sp-1) var(--sp-2);
  font-size: 12.5px;
}
.group-name {
  font-weight: 600;
  color: var(--text-2);
}

.rows {
  overflow: hidden;
}
.row {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  padding: var(--sp-3) var(--sp-4);
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  transition: background var(--transition);
}
.row:last-child {
  border-bottom: none;
}
.row:hover {
  background: var(--hover);
}
.row.sel {
  background: var(--brand-soft);
}
.row-main {
  flex: 1;
  min-width: 0;
}
.row-title {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  font-weight: 550;
  font-size: 13.5px;
}
.row-path {
  font-size: 12px;
  color: var(--text-3);
  margin-top: 2px;
}
.row-desc {
  font-size: 12px;
  margin-top: 2px;
}
.row-size {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  flex: none;
  font-size: 12px;
  gap: 2px;
}
.row-size b {
  font-size: 13.5px;
  font-variant-numeric: tabular-nums;
}
</style>
