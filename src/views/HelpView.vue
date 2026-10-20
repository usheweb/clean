<script setup lang="ts">
// 使用说明 —— 改写自旧版 HELP_TEXT，按平台裁剪（macOS 不显示高级迁移）。
import { computed } from "vue";

import { useAppStore } from "../stores/app";

const app = useAppStore();

const isWin = computed(() => app.isWin);
const contactEmail = "app@usheweb.com";

async function copyEmail() {
  try {
    await navigator.clipboard.writeText(contactEmail);
  } catch {
    // 剪贴板不可用时忽略
  }
}
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div>
        <h1>使用说明</h1>
        <p class="sub">
          {{ app.info?.productName || "唐三浪清理" }} v{{ app.info?.version || "0.0.6" }} ——
          完全离线运行，不联网、不上传任何数据；所有清理项都是内置白名单，只碰清单里的目录。
        </p>
      </div>
    </header>

    <div class="doc card">
      <section class="sec">
        <h2>这是什么</h2>
        <p>
          一个完全离线的系统垃圾清理 / 搬家小工具：不联网、不上传任何数据，所有清理项都是内置白名单，
          只碰清单里的目录，不碰你的其他文件。删除默认进入
          {{ isWin ? "回收站" : "废纸篓" }}，随时可还原。
        </p>
      </section>

      <section class="sec">
        <h2>安全清理</h2>
        <ol>
          <li>点【开始扫描】：只读扫描，列出当前系统中可清理的缓存 / 日志等，此时不会删除任何东西。</li>
          <li>
            绿色 = 一定安全（纯缓存 / 日志）；黄色 = 建议先关闭对应软件再清。拿不准就只勾绿色，
            或直接点【只勾绿色】。
          </li>
          <li>
            点【清理选中项】开始处理：默认进{{ isWin ? "回收站" : "废纸篓" }}（可还原）；
            勾选「直接删除」才会立即释放空间且不可恢复，请谨慎勾选。
          </li>
          <li>处理过程中界面不会卡住：底部任务条实时显示进度，可随时【暂停 / 继续】或【停止】。</li>
        </ol>
      </section>

      <template v-if="isWin">
        <section class="sec">
          <h2>高级迁移</h2>
          <p>
            原理：把 C 盘的大目录整体搬到其他盘，原位置留一个 junction 联接，系统和软件完全无感知、
            照常使用，C 盘空间立即腾出。
          </p>
          <ol>
            <li>
              选好【目标盘】和【目标文件夹名】，点【扫描 C 盘可迁移目录】。列表按体积从大到小排，
              方便你挑大的先搬。
            </li>
            <li>有进程在运行的行不可选中；关掉对应软件后点那一行，会自动刷新为可迁移。</li>
            <li>
              点【迁移选中项】：自动执行 搬运 → 校验 → 建联接，任何一步失败都会自动回滚，不会丢数据。
            </li>
            <li>
              每次成功迁移都会记入下方【迁移账本】；想搬回 C 盘，选中记录点【还原选中项】即可。
            </li>
            <li>
              【整理到当前目录】：多次迁移数据散在多个文件夹时，用它可把已迁移数据归拢到当前填写的
              目标文件夹（同盘为瞬间改名完成）。
            </li>
          </ol>
        </section>

        <section class="sec">
          <h2>未验证候选（白名单之外的大目录）</h2>
          <p>
            白名单不可能把市面上所有软件都列全，所以工具有一层「发现」：勾选【同时发现未验证候选】后，
            它会自己去 <span class="mono">%LOCALAPPDATA%</span>、<span class="mono">%APPDATA%</span>、
            ProgramData 里找 500MB 以上的目录，白名单已覆盖的不再重复列出。
          </p>
          <ul>
            <li>结果以紫色【候选】行显示，形如 <span class="mono">Local\某软件\某模块</span>。</li>
            <li>
              候选是【未验证】的：工具不知道它属于哪个软件、无法检测那个软件是否在运行、也不保证该软件
              支持联接迁移 —— 默认不选，迁移前会二次确认并要求你自行承担风险。点确定前请先完全退出相关软件。
            </li>
            <li>
              已硬排除的：Windows/Office/Edge/OneDrive 等系统组件、商店应用（Packages）、程序本体（Programs），
              以及所有联接。
            </li>
            <li>
              目录里若还套着联接（多半是已迁移的子目录），迁移会被直接拦下 —— 那会把联接指向的数据再搬一遍
              （数据重复）。遇到就改用更精确的目录。
            </li>
          </ul>
        </section>

        <section class="sec">
          <h2>账本与备份（迁移记录）</h2>
          <p>账本记着「哪条数据搬到哪了」，一键还原全靠它。账本有多层互备，坏一层还有下一层：</p>
          <ul>
            <li>主账本：应用数据目录下的 <span class="mono">migrations.json</span></li>
            <li>同目录副本：<span class="mono">migrations.json.bak</span>（与主账本同一份内容，不会复活已还原的记录）</li>
            <li>操作日志：同目录 <span class="mono">ledger_journal.jsonl</span>（一次操作追加一行，每一步迁移 / 还原 / 整理都留痕）</li>
            <li>数据旁边镜像：目标盘上的 <span class="mono">&lt;目标文件夹名&gt;/clean_c_ledger_backup.json</span>，跟着数据走</li>
            <li>读账本时按上面顺序自动兜底，救回来会立刻回填主账本。</li>
            <li>
              账本真丢了？点【重建账本】：工具会扫 C 盘原位的联接，按联接的真实指向把记录反推出来
              （候选目录也一样能认出来；只认指向其他盘的联接）。数据一直都在。
            </li>
          </ul>
        </section>
      </template>

      <section class="sec">
        <h2>大文件（只读）</h2>
        <ol>
          <li>
            点【开始扫描】：按体积从大到小列出大文件，只读展示，不提供任何删除入口 —— 删不删由你自己在
            文件管理器里决定，天然有回收站兜底。
          </li>
          <li>
            系统目录与系统文件已硬排除，不会出现在结果里。默认只扫用户数据区
            （桌面 / 下载 / 文档 / 视频等），速度较快；勾选【包含 AppData】才会进入各软件的数据目录，会慢很多。
          </li>
          <li>
            选中一行后可以【打开所在位置】或【复制路径】，再自行判断处理。类别列只是体积参考，
            不代表可以删。
          </li>
        </ol>
      </section>

      <section class="sec">
        <h2>{{ isWin ? "C 盘已装软件（只读）" : "已装软件（只读）" }}</h2>
        <ol>
          <li>
            点【开始扫描】：{{ isWin ? "读 Windows 注册表（不联网）" : "扫描 /Applications 与 ~/Applications" }}，
            把软件连同它们的真实体积列出来，按体积从大到小排，用来回答「我的磁盘到底被谁占了」。
          </li>
          <li>
            这里列的是【程序本体】，既不能迁移也不能直接删：搬去别的盘会让软件无法修复 / 更新，
            手动删会让软件损坏。要卸载请走系统里的应用管理。
          </li>
          <li>
            本页只是盘点；真正能安全迁移的数据 / 缓存目录，仍由清理与{{ isWin ? "迁移" : "清理" }}功能负责。
          </li>
        </ol>
      </section>

      <section class="sec">
        <h2>常见问题</h2>
        <ul>
          <li>会删掉我的软件吗？ —— 不会。清理项都是缓存 / 日志类，删除默认可还原。</li>
          <li v-if="isWin">迁移中断电 / 报错怎么办？ —— 程序会自动回滚到迁移前的状态。</li>
          <li>扫描很慢？ —— 正在统计文件大小，耐心等待即可，也可点【停止】中断。</li>
          <li>建议先关闭微信 / QQ / 浏览器等再清理对应项，避免文件占用。</li>
        </ul>
      </section>

      <section class="sec contact">
        <h2>联系方式</h2>
        <p>
          遇到问题或有建议，欢迎发邮件：
          <a href="#" @click.prevent="copyEmail">{{ contactEmail }}</a>
          <span class="text-3">（点击复制）</span>
        </p>
      </section>
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
.page-head h1 {
  font-size: 20px;
  font-weight: 650;
}
.sub {
  margin-top: 4px;
  color: var(--text-2);
  font-size: 13px;
  max-width: 760px;
}
.doc {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: var(--sp-5) var(--sp-6);
  user-select: text;
}
.sec {
  padding-bottom: var(--sp-5);
}
.sec + .sec {
  border-top: 1px solid var(--border);
  padding-top: var(--sp-5);
}
.sec h2 {
  font-size: 15px;
  font-weight: 600;
  margin-bottom: var(--sp-2);
  color: var(--text);
}
.sec p,
.sec li {
  font-size: 13px;
  color: var(--text-2);
  line-height: 1.85;
}
.sec ol,
.sec ul {
  margin: 0;
  padding-left: 20px;
}
.sec li {
  margin-bottom: 4px;
}
.contact a {
  font-weight: 600;
}
</style>
