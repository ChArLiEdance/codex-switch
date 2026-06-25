const ACCOUNT_DETAIL_STORAGE_KEY = "codex-switch-show-account-detail";

export function resolveInitialShowAccountDetail(): boolean {
  const stored = globalThis.localStorage?.getItem(ACCOUNT_DETAIL_STORAGE_KEY);
  return stored === null ? true : stored !== "false";
}

export function persistShowAccountDetail(showDetail: boolean): void {
  globalThis.localStorage?.setItem(ACCOUNT_DETAIL_STORAGE_KEY, String(showDetail));
}
