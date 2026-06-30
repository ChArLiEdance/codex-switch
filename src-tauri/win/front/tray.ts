import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import "./tray.css";

type QuotaWindow = {
  remaining_percent: number | null;
  refresh_at: string | null;
  reset_at_timestamp: number | null;
};

type QuotaSummary = {
  five_hour: QuotaWindow;
  weekly: QuotaWindow;
};

type TrayProfileEntry = {
  folder_name: string;
  display_title: string;
  nickname: string;
  plan_name: string | null;
  quota: QuotaSummary;
  status: string;
  auth_present: boolean;
};

type TrayStatePayload = {
  locale: string;
  current_profile: string | null;
  current_title: string | null;
  current_quota: QuotaSummary | null;
  profiles: TrayProfileEntry[];
};

type Labels = {
  settings: string;
  accountQuota: string;
  noAccount: string;
  noQuota: string;
  used: string;
  left: string;
  resets: string;
};

const root = document.querySelector<HTMLElement>("#tray-root");

const fallbackState: TrayStatePayload = {
  locale: "zh-CN",
  current_profile: "hester",
  current_title: "hesterteazhang@gmail.com",
  current_quota: {
    five_hour: { remaining_percent: 89, refresh_at: "2026-06-28 17:26", reset_at_timestamp: null },
    weekly: { remaining_percent: 19, refresh_at: "2026-06-30 15:59", reset_at_timestamp: null },
  },
  profiles: [
    {
      folder_name: "hester",
      display_title: "hesterteazhang@gmail.com",
      nickname: "hester",
      plan_name: "Pro",
      quota: {
        five_hour: { remaining_percent: 89, refresh_at: "2026-06-28 17:26", reset_at_timestamp: null },
        weekly: { remaining_percent: 19, refresh_at: "2026-06-30 15:59", reset_at_timestamp: null },
      },
      status: "available",
      auth_present: true,
    },
  ],
};

function labels(locale: string): Labels {
  if (locale.startsWith("zh")) {
    return {
      settings: "设置",
      accountQuota: "当前额度",
      noAccount: "暂无当前账号",
      noQuota: "暂无额度数据",
      used: "已用",
      left: "剩余",
      resets: "重置",
    };
  }
  return {
    settings: "Settings",
    accountQuota: "Current Quota",
    noAccount: "No active account",
    noQuota: "No quota data",
    used: "Used",
    left: "Left",
    resets: "Resets",
  };
}

function hasTauriRuntime(): boolean {
  return Boolean((globalThis as typeof globalThis & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
}

function htmlEscape(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function currentTitle(state: TrayStatePayload, label: Labels): string {
  const current = state.profiles.find((profile) => profile.folder_name === state.current_profile);
  const nickname = current?.nickname.trim();
  if (nickname) {
    return nickname;
  }
  const title = state.current_title?.trim();
  return title || label.noAccount;
}

function percent(window: QuotaWindow): number | null {
  if (window.remaining_percent == null) {
    return null;
  }
  return Math.max(0, Math.min(100, window.remaining_percent));
}

function quotaTone(value: number | null): string {
  if (value == null) {
    return "muted";
  }
  if (value > 60) {
    return "green";
  }
  if (value >= 20) {
    return "orange";
  }
  return "red";
}

function quotaRow(title: string, window: QuotaWindow, label: Labels): string {
  const left = percent(window);
  const used = left == null ? null : 100 - left;
  const tone = quotaTone(left);
  const width = `${left ?? 0}%`;
  const reset = window.refresh_at?.trim() || "--";
  return `
    <div class="tray-quota-row">
      <div class="tray-quota-label">${title}</div>
      <div class="tray-quota-meter">
        <div class="tray-progress">
          <div class="tray-progress-fill tray-progress-fill--${tone}" style="width: ${width}"></div>
        </div>
        <div class="tray-quota-meta">
          <span>${label.used} ${used == null ? "--" : `${used.toFixed(1)}%`}</span>
          <span class="tray-quota-left tray-quota-left--${tone}">${label.left} ${left == null ? "--" : `${left.toFixed(1)}%`}</span>
          <span>${label.resets} ${htmlEscape(reset)}</span>
        </div>
      </div>
    </div>
  `;
}

function settingsButton(title: string): string {
  return `
    <button class="tray-settings-button" type="button" data-action="settings" title="${htmlEscape(title)}" aria-label="${htmlEscape(title)}">
      <img class="tray-settings-icon" src="/ccswitch-icons/settings.svg" alt="" draggable="false" />
    </button>
  `;
}

function render(state: TrayStatePayload): void {
  if (!root) {
    return;
  }
  const label = labels(state.locale || "en");
  const title = currentTitle(state, label);
  const quota = state.current_quota;
  root.innerHTML = `
    <section class="tray-status-panel">
      ${settingsButton(label.settings)}
      <div class="tray-status-head">
        <div class="tray-app-mark" aria-hidden="true">
          <img src="/ccswitch-icons/codex-switch-app-icon.png" alt="" draggable="false" />
        </div>
        <div class="tray-heading">
          <div class="tray-status-title">Codex Switch</div>
          <div class="tray-account-title">${htmlEscape(label.accountQuota)} · ${htmlEscape(title)}</div>
        </div>
      </div>
      <div class="tray-status-content">
        ${
          quota
            ? `<div class="tray-quota-stack">
                ${quotaRow("5h", quota.five_hour, label)}
                ${quotaRow("7d", quota.weekly, label)}
              </div>`
            : `<p class="tray-empty">${htmlEscape(label.noQuota)}</p>`
        }
      </div>
    </section>
  `;
}

async function refresh(): Promise<void> {
  if (!hasTauriRuntime()) {
    render(fallbackState);
    return;
  }
  render(await invoke<TrayStatePayload>("get_tray_state"));
}

async function handleAction(action: string): Promise<void> {
  if (!hasTauriRuntime()) {
    return;
  }
  if (action === "settings") {
    await invoke("open_tray_route", { route: "settings" });
    return;
  }
}

root?.addEventListener("click", (event) => {
  const button = (event.target as HTMLElement).closest<HTMLButtonElement>("[data-action]");
  const action = button?.dataset.action;
  if (action) {
    void handleAction(action);
  }
});

void listen<TrayStatePayload>("codex-switch://tray-state-updated", (event) => {
  render(event.payload);
});

void refresh();
