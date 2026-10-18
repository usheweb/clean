import { defineStore } from "pinia";
import { ref } from "vue";

import { cancelTask, onTaskDone, onTaskLog, onTaskProgress, pauseTask, resumeTask } from "../api/ipc";
import type { TaskDone, TaskLog, TaskProgress } from "../api/types";

const MAX_LOGS = 800;

/**
 * 全局任务状态：驱动底部任务条（spinner / 进度 / 暂停 / 停止）。
 * 同一时刻只有一个任务（后端也是这么约束的）。
 */
export const useTaskStore = defineStore("task", () => {
  const taskId = ref<string | null>(null);
  const running = ref(false);
  const progress = ref<TaskProgress | null>(null);
  const logs = ref<TaskLog[]>([]);
  const lastDone = ref<TaskDone | null>(null);
  const showLogs = ref(false);

  let initialized = false;

  function init() {
    if (initialized) return;
    initialized = true;
    void onTaskProgress((p) => {
      if (p.taskId !== taskId.value) return;
      progress.value = p;
    });
    void onTaskLog((l) => {
      if (l.taskId !== taskId.value) return;
      logs.value.push(l);
      if (logs.value.length > MAX_LOGS) {
        logs.value.splice(0, logs.value.length - MAX_LOGS);
      }
    });
    void onTaskDone((d) => {
      if (d.taskId !== taskId.value) return;
      lastDone.value = d;
      running.value = false;
      progress.value = null;
    });
  }

  /** 起一个任务：fn 返回 taskId（命令本身立即返回） */
  async function start(fn: () => Promise<string>): Promise<string | null> {
    init();
    try {
      const id = await fn();
      taskId.value = id;
      running.value = true;
      progress.value = null;
      logs.value = [];
      lastDone.value = null;
      return id;
    } catch (e) {
      lastDone.value = {
        taskId: "",
        kind: "clean_scan",
        ok: 0,
        skipped: 0,
        failed: 1,
        bytes: 0,
        stopped: false,
        message: String(e),
      };
      return null;
    }
  }

  async function togglePause() {
    if (!running.value) return;
    if (progress.value?.paused) {
      await resumeTask();
      if (progress.value) progress.value = { ...progress.value, paused: false };
    } else {
      await pauseTask();
      if (progress.value) progress.value = { ...progress.value, paused: true };
    }
  }

  async function stop() {
    if (!running.value) return;
    await cancelTask();
  }

  function clearDone() {
    lastDone.value = null;
  }

  return {
    taskId,
    running,
    progress,
    logs,
    lastDone,
    showLogs,
    init,
    start,
    togglePause,
    stop,
    clearDone,
  };
});
