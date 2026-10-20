<script setup lang="ts">
import { useUiStore } from "../stores/ui";

const ui = useUiStore();
</script>

<template>
  <div class="toast-host">
    <TransitionGroup name="toast">
      <div
        v-for="t in ui.toasts"
        :key="t.id"
        class="toast"
        :class="`t-${t.level}`"
        @click="ui.dismiss(t.id)"
      >
        <span class="dot" />
        <span class="text">{{ t.text }}</span>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.toast-host {
  position: fixed;
  top: var(--sp-4);
  right: var(--sp-4);
  z-index: 2000;
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
  pointer-events: none;
}
.toast {
  pointer-events: auto;
  display: flex;
  align-items: flex-start;
  gap: var(--sp-2);
  max-width: 380px;
  padding: 10px 14px;
  border-radius: var(--r);
  background: var(--panel-solid);
  border: 1px solid var(--line);
  box-shadow: var(--shadow-md);
  font-size: 13px;
  cursor: pointer;
  animation: fade-in 180ms ease-out;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-top: 6px;
  flex: none;
  background: var(--blue);
}
.t-ok .dot {
  background: var(--green);
}
.t-warn .dot {
  background: var(--yellow);
}
.t-error .dot {
  background: var(--red);
}
.text {
  word-break: break-word;
}

.toast-enter-active,
.toast-leave-active {
  transition: all 200ms ease;
}
.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateX(16px);
}
</style>
