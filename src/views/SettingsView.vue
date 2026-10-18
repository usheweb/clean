<script setup lang="ts">
import { computed, ref } from "vue";

import { relaunchAsAdmin } from "../api/client";
import { useAppStore } from "../stores/app";
import { useSettingsStore } from "../stores/settings";
import { useUiStore } from "../stores/ui";

const app = useAppStore();
const cfg = useSettingsStore();
const ui = useUiStore();

const rootName = ref(cfg.settings.targetRootName);
const saving = ref(false);

const themeLabel = computed(() => (cfg.settings.theme === "dark" ? "深色" : "浅色"));

async function saveRootName() {
  saving.value = true;
  await cfg.update({ targetRootName: rootName.value.trim() || "clean_c_migrated" });
  rootName.value = cfg.settings.targetRootName;
  saving.value = false;
  ui.push("设置已保存", "ok");
}

async function elevate() {
  try {
    await relaunchAsAdmin();
    ui.push("已请求提权，请在系统弹窗中确认", "info");
  } catch (e) {
    ui.push(String(e), "error");
  }
}
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div>
        <h1>设置</h1>
        <p class="sub">界面主题、迁移默认参数与应用信息。</p>
      </div>
    </header>

    <section class="card sec">
      <h2 class="sec-title">外观</h2>
      <div class="sec-row">
        <div>
          <div class="k">主题</div>
          <div class="text-3 v">当前：{{ themeLabel }}</div>
        </div>
        <button class="btn btn-ghost btn-sm" @click="cfg.toggleTheme()">
          切换到{{ cfg.settings.theme === "dark" ? "浅色" : "深色" }}
        </button>
      </div>
    </section>

    <section v-if="app.isWin" class="card sec">
      <h2 class="sec-title">高级迁移</h2>
      <div class="sec-row">
        <div>
          <div class="k">默认目标文件夹名</div>
          <div class="text-3 v">迁移数据在目标盘上的存放目录</div>
        </div>
        <div class="inline">
          <input v-model="rootName" class="field" :disabled="saving" />
          <button class="btn btn-primary btn-sm" :disabled="saving" @click="saveRootName">保存</button>
        </div>
      </div>
    </section>

    <section class="card sec">
      <h2 class="sec-title">权限</h2>
      <div class="sec-row">
        <div>
          <div class="k">管理员权限</div>
          <div class="text-3 v">
            {{
              app.isAdmin
                ? "当前已以管理员身份运行，可以清理系统级目录（系统临时文件、Windows 更新缓存等）。"
                : app.isMac
                  ? "当前为普通用户。系统级目录（/Library/Caches、/Library/Logs）需要 sudo，会被跳过或失败。"
                  : "当前为普通用户。标记「需管理员」的条目会被跳过或部分失败。"
            }}
          </div>
        </div>
        <button v-if="app.isWin && !app.isAdmin" class="btn btn-ghost btn-sm" @click="elevate">
          以管理员身份重启
        </button>
      </div>
    </section>

    <section class="card sec">
      <h2 class="sec-title">关于</h2>
      <div class="about">
        <div class="about-row"><span class="k">产品名称</span><span>{{ app.info?.productName || "唐三浪清理" }}</span></div>
        <div class="about-row"><span class="k">版本</span><span>v{{ app.info?.version || "0.0.1" }}</span></div>
        <div class="about-row"><span class="k">平台</span><span>{{ app.info?.platform || "—" }}</span></div>
        <div class="about-row"><span class="k">用户目录</span><span class="mono">{{ app.info?.homeDir || "—" }}</span></div>
      </div>
    </section>
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
}
.sec {
  padding: var(--sp-4) var(--sp-5) var(--sp-5);
}
.sec-title {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: var(--sp-3);
}
.sec-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-4);
}
.k {
  font-weight: 550;
  font-size: 13.5px;
}
.v {
  font-size: 12.5px;
  margin-top: 2px;
  max-width: 560px;
}
.inline {
  display: flex;
  gap: var(--sp-2);
}
.about {
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
}
.about-row {
  display: flex;
  gap: var(--sp-3);
  font-size: 13px;
}
.about-row .k {
  width: 90px;
  color: var(--text-3);
  font-weight: 400;
}
</style>
