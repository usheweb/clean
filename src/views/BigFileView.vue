<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";

import { onDataDone, onDataRow, openPath, revealPath, startBigFileScan, type BigFile } from "../api/ipc";
import { useTaskStore } from "../stores/task";
import { useUiStore } from "../stores/ui";
import { fmtCount, fmtDate, fmtSize } from "../utils/format";

const task = useTaskStore();
const ui = useUiStore();

const rows = ref<BigFile[]>([]);
const includeAppData = ref(false);
const minMB = ref(100);
const scanned = ref(false);
const scanning = ref(false);

let unRow: (() => void) | null = null;
let unDone: (() => void) | null = null;

onMounted(async () => {
  unRow = await onDataRow<BigFile>("big_files", (r) => rows.value.push(r.row));
  unDone = await onDataDone("big_files", () => {
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
  await task.start(() => startBigFileScan(includeAppData.value, minMB.value * 1024 * 1024));
}

async function reveal(path: string) {
  try {
    await revealPath(path);
  } catch (e) {
    ui.push(String(e), "error");
  }
}

async function openFile(path: string) {
  try {
    await openPath(path);
  } catch (e) {
    ui.push(String(e), "error");
  }
}

async function copyPath(path: string) {
  try {
    await navigator.clipboard.writeText(path);
    ui.push("路径已复制", "ok");
  } catch {
    ui.push("复制失败", "error");
  }
}
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div>
        <h1>大文件</h1>
        <p class="sub">
          按体积从大到小列出大文件，<b>只读展示，不提供删除入口</b>；删不删由你在文件管理器里自己决定，这样天然有回收站兜底。
        </p>
      </div>
      <button class="btn btn-primary" :disabled="task.running" @click="scan">
        {{ scanned ? "重新扫描" : "扫描大文件" }}
      </button>
    </header>

    <div class="toolbar card">
      <label class="switch">
        <input v-model="includeAppData" type="checkbox" class="checkbox" />
        <span>包含软件数据目录（更慢，会进入 AppData / ~/Library）</span>
      </label>
      <span class="grow" />
      <label class="switch">
        <span>最小体积</span>
        <select v-model.number="minMB" class="field">
          <option :value="100">100 MB</option>
          <option :value="500">500 MB</option>
          <option :value="1024">1 GB</option>
          <option :value="5120">5 GB</option>
        </select>
      </label>
      <span class="summary">共 {{ fmtCount(sorted.length) }} 个 · {{ fmtSize(total) }}</span>
    </div>

    <div v-if="!sorted.length" class="empty card">
      <span v-if="scanning" class="spinner spinner-lg" />
      <p class="empty-title">{{ scanning ? "正在扫描…" : scanned ? "没有发现符合条件的大文件" : "还没有扫描结果" }}</p>
      <p class="text-2">
        {{
          scanning
            ? "扫描期间可以随时暂停或停止。"
            : "系统目录与 pagefile/hiberfil 等系统文件已硬排除，不会出现在结果里。"
        }}
      </p>
    </div>

    <div v-else class="table card">
      <div class="thead">
        <span class="c-kind">类别</span>
        <span class="c-name">名称 / 路径</span>
        <span class="c-size">体积</span>
        <span class="c-time">修改时间</span>
        <span class="c-act">操作</span>
      </div>
      <div class="tbody">
        <div v-for="f in sorted" :key="f.path" class="tr">
          <span class="c-kind"><span class="tag tag-gray">{{ f.kind }}</span></span>
          <span class="c-name">
            <span class="nm ellipsis" :title="f.name">{{ f.name }}</span>
            <span class="pth mono ellipsis" :title="f.path">{{ f.path }}</span>
          </span>
          <span class="c-size">{{ fmtSize(f.bytes) }}</span>
          <span class="c-time text-3">{{ fmtDate(f.mtime) }}</span>
          <span class="c-act">
            <button class="btn btn-ghost btn-sm" @click="reveal(f.path)">打开位置</button>
            <button class="btn btn-ghost btn-sm" @click="openFile(f.path)">打开</button>
            <button class="btn btn-ghost btn-sm" @click="copyPath(f.path)">复制路径</button>
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
  flex-wrap: wrap;
}
.switch {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  font-size: 13px;
  cursor: pointer;
}
.switch select {
  height: 30px;
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
  grid-template-columns: 110px 1fr 110px 140px 250px;
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
.nm {
  display: block;
  font-weight: 550;
}
.pth {
  display: block;
  color: var(--text-3);
  font-size: 11.5px;
}
.c-size {
  font-variant-numeric: tabular-nums;
  font-weight: 600;
}
.c-time {
  font-size: 12px;
}
.c-act {
  display: flex;
  gap: 6px;
  justify-content: flex-end;
}
</style>
