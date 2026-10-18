import { defineStore } from "pinia";
import { ref } from "vue";

export interface Toast {
  id: number;
  text: string;
  level: "info" | "ok" | "warn" | "error";
}

/** 轻提示 */
export const useUiStore = defineStore("ui", () => {
  const toasts = ref<Toast[]>([]);
  let seq = 0;

  function push(text: string, level: Toast["level"] = "info", ms = 4200) {
    const id = ++seq;
    toasts.value.push({ id, text, level });
    window.setTimeout(() => {
      toasts.value = toasts.value.filter((t) => t.id !== id);
    }, ms);
  }

  function dismiss(id: number) {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }

  return { toasts, push, dismiss };
});
