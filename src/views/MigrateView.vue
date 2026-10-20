<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";

import ConfirmDialog from "../components/ConfirmDialog.vue";
import {
  listDrives,
  listLedger,
  listMigrateItems,
  onDataDone,
  onDataRow,
  startMigrateRun,
  startMigrateScan,
  startRebuildLedger,
  startReorganize,
  startRestore,
  type LedgerEntry,
  type MigrateDoneRow,
  type MigrateItem,
  type MigrateRow,
} from "../api/ipc";
import { useSettingsStore } from "../stores/settings";
import { useTaskStore } from "../stores/task";
import { useUiStore } from "../stores/ui";
import { fmtCount, fmtSize } from "../utils/format";

const task = useTaskStore();
const ui = useUiStore();
const cfg = useSettingsStore();

const drives = ref<string[]>([]);
const items = ref<MigrateItem[]>([]);
const rows = ref<MigrateRow[]>([]);
const ledger = ref<LedgerEntry[]>([]);
const selected = reactive(new Set<string>());
const restoring = reactive(new Set<string>());
const scanned = ref(false);
const confirmMigrate = ref(false);
const confirmRestore = ref(false);

let unRow: (() => void) | null = null;
let unDone: (() => void) | null = null;
let unLedger: (() => void) | null = null;
let unAllDone: (() => void) | null = null;
let unMigrated: (() => void) | null = null;

const drive = ref("");
const rootName = ref("clean_c_migrated");
const includeCandidates = ref(false);
const newRoot = ref("");

onMounted(async () => {
  try {
    drives.value = await listDrives();
    items.value = await listMigrateItems();
    ledger.value = await listLedger();
  } catch {
    drives.value = [];
  }
  rootName.value = cfg.settings.targetRootName || "clean_c_migrated";
  drive.value = cfg.settings.targetDrive || drives.value.find((d) => !d.toUpperCase().startsWith("C")) || drives.value[0] || "";
  includeCandidates.value = cfg.settings.includeCandidates;

  unRow = await onDataRow<MigrateRow>("migrate_rows", (r) => rows.value.push(r.row));
  unDone = await onDataDone("migrate_rows", () => undefined);
  unLedger = await onDataRow<LedgerEntry>("migrate_ledger", (r) => {
    const i = ledger.value.findIndex((x) => x.src === r.row.src);
    if (i >= 0) ledger.value[i] = r.row;
    else ledger.value.push(r.row);
  });
  unAllDone = await onDataDone("migrate_ledger", () => undefined);
  // 每搬完一条就把该行标成「已迁移」并取消勾选。不这样做的话，列表会一直停在扫描时的
  // 样子（条目还在、还勾着、体积还是原值、状态还写着"可迁移"），用户只能自己再扫一遍。
  // 失败的条目不发这个事件 —— 数据已回搬原位置，保持"可迁移 + 勾选"状态正好可以重试。
  unMigrated = await onDataRow<MigrateDoneRow>("migrate_done", (r) => {
    const row = rows.value.find((x) => x.key === r.row.key);
    if (!row) return;
    row.migrated = true;
    selected.delete(r.row.key);
  });
  task.init();
});

// 迁移/还原/整理/重建结束后刷新账本，并让列表行的「已迁移」跟账本对齐：
// 迁移成功的亮起来，被还原掉的退回去。账本事件只能追加/替换，删不掉已还原的记录，
// 所以必须重新拉一次；列表则按 key 对账，免得还原完还挂着"已迁移"的旧状态。
watch(
  () => task.running,
  async (now, before) => {
    if (!before || now) return;
    ledger.value = await listLedger();
    const keys = new Set(ledger.value.map((e) => e.key));
    for (const r of rows.value) {
      r.migrated = keys.has(r.key);
      if (r.migrated) selected.delete(r.key);
    }
  },
);

onBeforeUnmount(() => {
  unRow?.();
  unDone?.();
  unLedger?.();
  unAllDone?.();
  unMigrated?.();
});

const catOrder = ["开发", "聊天", "办公", "浏览器", "工具", "系统", "候选"];
const sorted = computed(() =>
  [...rows.value].sort((a, b) => {
    const ia = catOrder.indexOf(a.category);
    const ib = catOrder.indexOf(b.category);
    if (a.candidate !== b.candidate) return a.candidate ? 1 : -1;
    if (ia !== ib) return (ia < 0 ? 99 : ia) - (ib < 0 ? 99 : ib);
    return b.bytes - a.bytes;
  }),
);
const selectedRows = computed(() => sorted.value.filter((r) => selected.has(r.key)));
const selectedBytes = computed(() => selectedRows.value.reduce((s, r) => s + r.bytes, 0));
const hasCandidates = computed(() => selectedRows.value.some((r) => r.candidate));

async function scan() {
  if (!drive.value) {
    ui.push("请先选择目标盘", "warn");
    return;
  }
  rows.value = [];
  selected.clear();
  await cfg.update({ targetRootName: rootName.value, targetDrive: drive.value, includeCandidates: includeCandidates.value });
  await task.start(() => startMigrateScan(drive.value, rootName.value, includeCandidates.value));
  scanned.value = true;
}

function toggle(key: string) {
  if (selected.has(key)) selected.delete(key);
  else selected.add(key);
}

async function doMigrate() {
  confirmMigrate.value = false;
  await task.start(() => startMigrateRun(selectedRows.value, drive.value, rootName.value));
}

function askRestore() {
  if (restoring.size === 0) {
    ui.push("请先勾选要还原的账本记录", "warn");
    return;
  }
  confirmRestore.value = true;
}

async function doRestore() {
  confirmRestore.value = false;
  await task.start(() => startRestore([...restoring]));
}

async function reorganize() {
  if (!newRoot.value) {
    ui.push("请先填写要归拢到的目标文件夹名", "warn");
    return;
  }
  await task.start(() => startReorganize(drive.value, newRoot.value));
}

async function rebuild() {
  await task.start(() => startRebuildLedger());
}

function toggleRestore(src: string) {
  if (restoring.has(src)) restoring.delete(src);
  else restoring.add(src);
}
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div>
        <h1>高级迁移（Windows）</h1>
        <p class="sub">
          把大目录整体搬到其它盘，原位置留一个 junction 联接，系统和软件完全无感知。
          任何一步失败都会自动回滚，绝不丢数据。
        </p>
      </div>
    </header>

    <div class="toolbar card">
      <label class="fld">
        <span>目标盘</span>
        <select v-model="drive" class="field">
          <option v-for="d in drives" :key="d" :value="d">{{ d }}</option>
        </select>
      </label>
      <label class="fld">
        <span>目标文件夹名</span>
        <input v-model="rootName" class="field" placeholder="clean_c_migrated" />
      </label>
      <label class="switch">
        <input v-model="includeCandidates" type="checkbox" class="checkbox" />
        <span>同时发现未验证候选</span>
      </label>
      <span class="grow" />
      <button class="btn btn-ghost" :disabled="task.running" @click="scan">扫描可迁移目录</button>
      <button class="btn btn-primary" :disabled="task.running || selected.size === 0" @click="confirmMigrate = true">
        迁移选中项
      </button>
    </div>

    <div v-if="!sorted.length" class="empty card">
      <p class="empty-title">{{ scanned ? "没有扫描到可迁移目录" : "还没有扫描结果" }}</p>
      <p class="text-2">内置白名单共 {{ items.length }} 条。标 ⚠ 的行表示对应软件正在运行，关掉后可迁移。</p>
    </div>

    <div v-else class="table card">
      <div class="thead">
        <span class="c-chk" />
        <span class="c-name">名称 / 路径</span>
        <span class="c-cat">类别</span>
        <span class="c-size">体积</span>
        <span class="c-state">状态</span>
      </div>
      <div class="tbody">
        <label
          v-for="r in sorted"
          :key="r.key"
          class="tr"
          :class="{ sel: selected.has(r.key), blocked: r.running.length > 0 || r.migrated, cand: r.candidate }"
        >
          <span class="c-chk">
            <input
              class="checkbox"
              type="checkbox"
              :disabled="r.running.length > 0 || r.migrated"
              :checked="selected.has(r.key)"
              @change="toggle(r.key)"
            />
          </span>
          <span class="c-name">
            <span class="nm">{{ r.name }}</span>
            <span class="pth mono ellipsis" :title="r.src">{{ r.src }}</span>
            <span v-if="r.desc" class="dsc text-3">{{ r.desc }}</span>
          </span>
          <span class="c-cat">
            <span class="tag" :class="r.candidate ? 'tag-purple' : 'tag-gray'">
              {{ r.candidate ? "候选" : r.category }}
            </span>
          </span>
          <span class="c-size">{{ fmtSize(r.bytes) }}<span class="text-3 f">{{ fmtCount(r.files) }} 文件</span></span>
          <span class="c-state">
            <span v-if="r.migrated" class="tag tag-blue">已迁移</span>
            <span v-else-if="r.running.length" class="tag tag-yellow" :title="r.running.join(', ')">
              ⚠ {{ r.running[0] }} 运行中
            </span>
            <span v-else class="tag tag-green">可迁移</span>
          </span>
        </label>
      </div>
    </div>

    <section class="ledger card">
      <div class="ledger-head">
        <div>
          <span class="lg-title">迁移账本</span>
          <span class="text-3">（记录"哪条数据搬到哪了"，支持一键还原）</span>
        </div>
        <div class="lg-actions">
          <input v-model="newRoot" class="field" placeholder="归拢到的文件夹名" />
          <button class="btn btn-ghost btn-sm" :disabled="task.running || !newRoot" @click="reorganize">整理到该目录</button>
          <button class="btn btn-ghost btn-sm" :disabled="task.running || restoring.size === 0" @click="askRestore">还原选中项</button>
          <button class="btn btn-ghost btn-sm" :disabled="task.running" @click="rebuild">重建账本</button>
        </div>
      </div>
      <div v-if="ledger.length === 0" class="lg-empty text-3">暂无迁移记录</div>
      <div v-else class="lg-body">
        <label v-for="e in ledger" :key="e.src" class="lg-row" :class="{ sel: restoring.has(e.src) }">
          <input
            class="checkbox"
            type="checkbox"
            :checked="restoring.has(e.src)"
            @change="toggleRestore(e.src)"
          />
          <span class="lg-name">{{ e.name }}</span>
          <span class="lg-src mono ellipsis" :title="e.src">{{ e.src }}</span>
          <span class="lg-arrow">→</span>
          <span class="lg-dst mono ellipsis" :title="e.dst">{{ e.dst }}</span>
          <span class="lg-size">{{ fmtSize(e.bytes) }}</span>
          <span class="lg-time text-3">{{ e.time }}</span>
        </label>
      </div>
    </section>

    <ConfirmDialog
      v-model:visible="confirmMigrate"
      :title="hasCandidates ? '确认迁移（含未验证候选）' : '确认迁移'"
      :message="
        `将迁移 ${selectedRows.length} 个目录，合计约 ${fmtSize(selectedBytes)}。\n` +
        (hasCandidates
          ? '其中包含【未验证候选】：工具无法确认它属于哪个软件、也不保证该软件支持联接迁移，请自行承担风险，并确保相关软件已完全退出。'
          : '迁移前会自动检测相关进程，运行中的条目不会被迁移。')
      "
      confirm-text="开始迁移"
      @confirm="doMigrate"
    />

    <ConfirmDialog
      v-model:visible="confirmRestore"
      title="确认还原"
      :message="`将把 ${restoring.size} 条记录的数据搬回原位置并删除联接。`"
      confirm-text="开始还原"
      @confirm="doRestore"
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
  overflow-y: auto;
}
.page-head h1 {
  font-size: 20px;
  font-weight: 650;
}
.sub {
  margin-top: 4px;
  color: var(--text-2);
  font-size: 13px;
  max-width: 720px;
}
.toolbar {
  display: flex;
  align-items: flex-end;
  gap: var(--sp-3);
  padding: var(--sp-3) var(--sp-4);
  flex-wrap: wrap;
}
.fld {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  color: var(--text-2);
}
.fld .field {
  min-width: 150px;
}
.switch {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  font-size: 13px;
  cursor: pointer;
  padding-bottom: 8px;
}
.grow {
  flex: 1;
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
  display: flex;
  flex-direction: column;
  overflow: hidden;
  flex: 1;
  min-height: 240px;
}
.thead,
.tr {
  display: grid;
  grid-template-columns: 40px 1fr 90px 120px 150px;
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
  cursor: pointer;
}
.tr:last-child {
  border-bottom: none;
}
.tr:hover {
  background: var(--hover);
}
.tr.sel {
  background: var(--brand-soft);
}
.tr.cand {
  background: var(--purple-soft);
}
.tr.blocked {
  opacity: 0.72;
  cursor: not-allowed;
}
.nm {
  font-weight: 550;
  font-size: 13.5px;
}
.pth {
  display: block;
  font-size: 11.5px;
  color: var(--text-3);
}
.dsc {
  display: block;
  font-size: 11.5px;
}
.c-size {
  font-variant-numeric: tabular-nums;
  font-weight: 600;
  display: flex;
  flex-direction: column;
}
.c-size .f {
  font-weight: 400;
  font-size: 11px;
}

.ledger {
  flex: none;
  padding: var(--sp-3) var(--sp-4) var(--sp-4);
}
.ledger-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-3);
  flex-wrap: wrap;
  padding-bottom: var(--sp-3);
  border-bottom: 1px solid var(--border);
}
.lg-title {
  font-weight: 600;
  margin-right: 6px;
}
.lg-actions {
  display: flex;
  gap: var(--sp-2);
  align-items: center;
}
.lg-actions .field {
  height: 30px;
  width: 170px;
}
.lg-empty {
  padding: var(--sp-4) 0;
  text-align: center;
  font-size: 13px;
}
.lg-body {
  max-height: 220px;
  overflow-y: auto;
}
.lg-row {
  display: grid;
  grid-template-columns: 22px 1.1fr 1.4fr 20px 1.4fr 90px 130px;
  gap: var(--sp-2);
  align-items: center;
  padding: var(--sp-2) 0;
  font-size: 12.5px;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
}
.lg-row:last-child {
  border-bottom: none;
}
.lg-row.sel {
  background: var(--brand-soft);
}
.lg-arrow {
  color: var(--text-3);
  text-align: center;
}
.lg-size {
  font-variant-numeric: tabular-nums;
}
</style>
