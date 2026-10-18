<script setup lang="ts">
withDefaults(
  defineProps<{
    visible: boolean;
    title: string;
    message?: string;
    confirmText?: string;
    cancelText?: string;
    danger?: boolean;
    busy?: boolean;
  }>(),
  {
    message: "",
    confirmText: "确定",
    cancelText: "取消",
    danger: false,
    busy: false,
  },
);

const emit = defineEmits<{
  (e: "confirm"): void;
  (e: "cancel"): void;
  (e: "update:visible", v: boolean): void;
}>();

function close() {
  emit("update:visible", false);
  emit("cancel");
}
</script>

<template>
  <Teleport to="body">
    <Transition name="dlg">
      <div v-if="visible" class="mask" @click.self="close">
        <div class="dlg card">
          <h3 class="dlg-title">{{ title }}</h3>
          <p v-if="message" class="dlg-msg">{{ message }}</p>
          <div class="dlg-body">
            <slot />
          </div>
          <div class="dlg-actions">
            <button class="btn btn-ghost" :disabled="busy" @click="close">
              {{ cancelText }}
            </button>
            <button
              class="btn"
              :class="danger ? 'btn-danger' : 'btn-primary'"
              :disabled="busy"
              @click="emit('confirm')"
            >
              {{ confirmText }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.mask {
  position: fixed;
  inset: 0;
  z-index: 1500;
  background: rgba(10, 15, 22, 0.42);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--sp-6);
}
.dlg {
  width: 100%;
  max-width: 520px;
  padding: var(--sp-5);
  box-shadow: var(--shadow-lg);
}
.dlg-title {
  font-size: 16px;
  font-weight: 600;
}
.dlg-msg {
  margin-top: var(--sp-2);
  color: var(--text-2);
  font-size: 13px;
  line-height: 1.7;
  white-space: pre-wrap;
  max-height: 320px;
  overflow-y: auto;
  user-select: text;
}
.dlg-body:not(:empty) {
  margin-top: var(--sp-4);
}
.dlg-actions {
  margin-top: var(--sp-5);
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-2);
}

.dlg-enter-active,
.dlg-leave-active {
  transition: opacity 160ms ease;
}
.dlg-enter-active .dlg,
.dlg-leave-active .dlg {
  transition: transform 160ms ease;
}
.dlg-enter-from,
.dlg-leave-to {
  opacity: 0;
}
.dlg-enter-from .dlg,
.dlg-leave-to .dlg {
  transform: scale(0.97);
}
</style>
