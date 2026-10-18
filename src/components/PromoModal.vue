<script setup lang="ts">
// 启动推广弹窗：展示「唐三浪学习时钟」。
// 倒计时结束前不能关闭；勾选「7 天内不再提示」后记下到期日，到期后再弹一次。
// 因为勾选偏好会一直保留，等价于「每周一次」。
import { computed, onBeforeUnmount, ref, watch } from "vue";

import { useSettingsStore } from "../stores/settings";

const props = defineProps<{ visible: boolean }>();
const emit = defineEmits<{
  (e: "update:visible", v: boolean): void;
}>();

const cfg = useSettingsStore();

const COUNTDOWN = 3;
const SKIP_DAYS = 7;

const skip = ref(false);
const remain = ref(COUNTDOWN);

let endAt = 0;
let timer: number | null = null;

const canClose = computed(() => remain.value <= 0);

/** 今天往后 days 天的日期键（YYYY-MM-DD），字符串比较即时间序 */
function dayKeyAfter(days: number): string {
  const d = new Date();
  d.setDate(d.getDate() + days);
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

function stopTimer() {
  if (timer != null) {
    window.clearInterval(timer);
    timer = null;
  }
}

/** 按截止时间戳重算，不受窗口卡顿/后台节流影响 */
function sync() {
  if (!endAt) return;
  remain.value = Math.max(0, Math.ceil((endAt - Date.now()) / 1000));
  if (remain.value <= 0) {
    endAt = 0;
    stopTimer();
  }
}

function tick() {
  sync();
}

watch(
  () => props.visible,
  (v) => {
    if (!v) {
      stopTimer();
      endAt = 0;
      return;
    }
    // 勾选偏好持久化：以前勾过就一直勾着
    skip.value = cfg.settings.lastPromoDay !== "";
    endAt = Date.now() + COUNTDOWN * 1000;
    remain.value = COUNTDOWN;
    if (timer == null) timer = window.setInterval(tick, 300);
    window.addEventListener("focus", sync);
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  stopTimer();
  window.removeEventListener("focus", sync);
});

async function close() {
  // 倒计时没走完，点了也关不掉
  if (!canClose.value) return;
  if (skip.value) {
    await cfg.update({ lastPromoDay: dayKeyAfter(SKIP_DAYS) });
  } else {
    await cfg.update({ lastPromoDay: "" });
  }
  emit("update:visible", false);
}
</script>

<template>
  <Teleport to="body">
    <Transition name="promo">
      <div v-if="visible" class="mask">
        <div class="promo">
          <img class="promo-img" src="/tslclock.png" alt="唐三浪学习时钟" draggable="false" />
          <div class="promo-actions">
            <label class="skip">
              <input v-model="skip" class="checkbox" type="checkbox" />
              <span>7 天内不再提示</span>
            </label>
            <button class="btn btn-sm promo-btn" :class="{ ready: canClose }" :disabled="!canClose" @click="close">
              {{ canClose ? "关闭" : `${remain} 秒后可关闭` }}
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
  z-index: 1800;
  background: rgba(10, 15, 22, 0.42);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--sp-6);
}
.promo {
  width: 100%;
  max-width: 604px;
  background: #fdfefe;
  border: 1px solid #7fc6d4;
  border-radius: var(--r-lg);
  box-shadow: var(--shadow-lg);
  padding: var(--sp-5);
}
.promo-img {
  display: block;
  width: 100%;
  height: auto;
  border-radius: var(--r);
  user-select: none;
}
.promo-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-4);
  margin-top: var(--sp-4);
}
.skip {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  font-size: 13px;
  color: #5a6f86;
  cursor: pointer;
}
.promo-btn {
  background: #f2f6f8;
  color: #a8bccb;
  min-width: 130px;
}
.promo-btn.ready {
  background: #e3f3f7;
  color: #008da8;
}
.promo-btn.ready:hover {
  background: #c9e9f1;
  color: #04252e;
}

.promo-enter-active,
.promo-leave-active {
  transition: opacity 180ms ease;
}
.promo-enter-active .promo,
.promo-leave-active .promo {
  transition: transform 180ms ease;
}
.promo-enter-from,
.promo-leave-to {
  opacity: 0;
}
.promo-enter-from .promo,
.promo-leave-to .promo {
  transform: scale(0.97);
}
</style>
