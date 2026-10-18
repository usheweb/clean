<script setup lang="ts">
import { computed } from "vue";

import { useTaskStore } from "../stores/task";
import { fmtCount, fmtDuration, fmtEta, fmtSize, fmtSpeed } from "../utils/format";

const task = useTaskStore();

const p = computed(() => task.progress);
const ratio = computed(() => p.value?.ratio ?? null);
const pct = computed(() => (ratio.value == null ? null : Math.round(ratio.value * 100)));
const bytesLeft = computed(() => {
  const x = p.value;
  if (!x || !x.bytesTotal) return 0;
  return Math.max(0, x.bytesTotal - x.bytesDone);
});
</script>

<template>
  <div v-if="task.running || task.showLogs" class="taskbar">
    <div v-if="task.showLogs" class="log-panel">
      <div class="log-head">
        <span>处理日志</span>
        <button class="log-clear" @click="task.showLogs = false">收起</button>
      </div>
      <div class="log-body">
        <div v-for="(l, i) in task.logs" :key="i" class="log-line" :class="`lvl-${l.level}`">
          {{ l.line }}
        </div>
        <div v-if="task.logs.length === 0" class="log-empty">暂无日志</div>
      </div>
    </div>

    <div class="bar-head">
      <!-- 「处理中的角度显示」：纯 CSS 环形旋转，永远在动 -->
      <span class="spinner" :class="{ paused: p?.paused }" />
      <div class="head-text">
        <div class="head-title">
          <span class="phase" v-if="p">{{ p.phase }}</span>
          <span class="title">{{ p?.title || "处理中…" }}</span>
          <span v-if="p?.paused" class="tag tag-yellow">已暂停</span>
        </div>
        <div class="head-path mono ellipsis" :title="p?.current || ''">
          {{ p?.current || "正在准备…" }}
        </div>
      </div>

      <div class="head-stats" v-if="p">
        <div class="stat">
          <span class="k">已处理</span>
          <span class="v">{{ fmtCount(p.itemsDone) }}/{{ p.itemsTotal || "?" }} 项</span>
        </div>
        <div class="stat">
          <span class="k">文件</span>
          <span class="v">{{ fmtCount(p.filesDone) }}/{{ p.filesTotal ? fmtCount(p.filesTotal) : "?" }}</span>
        </div>
        <div class="stat">
          <span class="k">{{ p.bytesTotal ? "已处理体积" : "已发现" }}</span>
          <span class="v">{{ fmtSize(p.bytesDone) }}</span>
        </div>
        <div class="stat" v-if="p.bytesTotal">
          <span class="k">速率</span>
          <span class="v">{{ fmtSpeed(p.speedBps) }}</span>
        </div>
        <div class="stat" v-if="p.bytesTotal">
          <span class="k">剩余</span>
          <span class="v">{{ fmtEta(bytesLeft, p.speedBps) }}</span>
        </div>
        <div class="stat">
          <span class="k">耗时</span>
          <span class="v">{{ fmtDuration(p.elapsedMs) }}</span>
        </div>
      </div>

      <div class="head-actions">
        <button class="btn btn-ghost btn-sm" @click="task.showLogs = !task.showLogs">
          日志<span v-if="task.logs.length" class="badge">{{ task.logs.length }}</span>
        </button>
        <button
          v-if="task.running && (p?.canPause ?? true)"
          class="btn btn-soft btn-sm"
          @click="task.togglePause()"
        >
          {{ p?.paused ? "继续" : "暂停" }}
        </button>
        <button v-if="task.running" class="btn btn-danger btn-sm" @click="task.stop()">停止</button>
      </div>
    </div>

    <div class="track" :class="{ indet: ratio == null }">
      <div
        v-if="ratio != null"
        class="fill"
        :style="{ width: pct + '%' }"
      />
      <div v-else class="fill-indet" />
    </div>

    <div class="bar-foot">
      <span v-if="pct != null">{{ pct }}%</span>
      <span v-else>正在处理，请稍候…（可随时暂停或停止）</span>
    </div>
  </div>
</template>

<style scoped>
.taskbar {
  position: relative;
  border-top: 1px solid var(--border);
  background: var(--bg-elevated);
  box-shadow: 0 -4px 16px rgba(16, 24, 40, 0.06);
  flex: none;
}

.bar-head {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  padding: var(--sp-3) var(--sp-4) var(--sp-2);
}

.spinner.paused {
  border-top-color: var(--yellow);
}

.head-text {
  flex: 1;
  min-width: 0;
}

.head-title {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  font-weight: 600;
  font-size: 13.5px;
}
.phase {
  font-size: 11px;
  color: var(--brand);
  background: var(--brand-soft);
  border-radius: var(--r-sm);
  padding: 1px 6px;
  font-weight: 500;
}
.head-path {
  font-size: 12px;
  color: var(--text-3);
  max-width: 100%;
}

.head-stats {
  display: flex;
  gap: var(--sp-4);
  flex: none;
}
.stat {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  line-height: 1.3;
}
.stat .k {
  font-size: 10.5px;
  color: var(--text-3);
}
.stat .v {
  font-size: 12.5px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.head-actions {
  display: flex;
  gap: var(--sp-2);
  flex: none;
}
.badge {
  margin-left: 5px;
  background: var(--brand);
  color: #fff;
  border-radius: var(--r-full);
  font-size: 10px;
  padding: 0 5px;
  line-height: 15px;
}

.track {
  position: relative;
  height: 5px;
  margin: 0 var(--sp-4);
  background: var(--surface-3);
  border-radius: var(--r-full);
  overflow: hidden;
}
.fill {
  height: 100%;
  background: linear-gradient(90deg, var(--brand), var(--brand-hover));
  border-radius: var(--r-full);
  transition: width 180ms linear;
}
.fill-indet {
  position: absolute;
  top: 0;
  height: 100%;
  background: linear-gradient(90deg, var(--brand), var(--brand-hover));
  border-radius: var(--r-full);
  animation: indeterminate 1.4s ease-in-out infinite;
}

.bar-foot {
  padding: var(--sp-1) var(--sp-4) var(--sp-2);
  font-size: 11.5px;
  color: var(--text-3);
  font-variant-numeric: tabular-nums;
}

.log-panel {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 100%;
  max-height: 260px;
  display: flex;
  flex-direction: column;
  background: var(--surface);
  border-top: 1px solid var(--border);
  box-shadow: var(--shadow-lg);
}
.log-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--sp-2) var(--sp-4);
  font-size: 12px;
  font-weight: 600;
  color: var(--text-2);
  border-bottom: 1px solid var(--border);
}
.log-clear {
  font-size: 12px;
  color: var(--brand);
}
.log-body {
  overflow-y: auto;
  padding: var(--sp-2) var(--sp-4);
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.7;
  user-select: text;
}
.log-line {
  white-space: pre-wrap;
  word-break: break-all;
}
.lvl-ok {
  color: var(--green);
}
.lvl-warn {
  color: var(--yellow);
}
.lvl-error {
  color: var(--red);
}
.lvl-info {
  color: var(--text-2);
}
.log-empty {
  color: var(--text-3);
  text-align: center;
  padding: var(--sp-4);
}
</style>
