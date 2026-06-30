import { state } from "@front-shared/state";
import type {
  CurrentQuotaResponse,
  DashboardViewModel,
  PagingInfo,
  ProfileCard,
  ProfilesSnapshotResponse,
} from "@front-shared/types";

const PROFILE_ORDER_STORAGE_KEY = "codex-switch-profile-order";

function loadProfileOrder(): string[] {
  const raw = globalThis.localStorage?.getItem(PROFILE_ORDER_STORAGE_KEY);
  if (!raw) {
    return [];
  }

  try {
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((value): value is string => typeof value === "string") : [];
  } catch {
    return [];
  }
}

export function persistProfileOrder(profiles: ProfileCard[]): void {
  globalThis.localStorage?.setItem(
    PROFILE_ORDER_STORAGE_KEY,
    JSON.stringify(profiles.map((profile) => profile.folder_name)),
  );
}

function applyProfileOrder(profiles: ProfileCard[]): ProfileCard[] {
  const order = loadProfileOrder();
  if (!order.length) {
    return profiles;
  }

  const rank = new Map(order.map((profile, index) => [profile, index]));
  return [...profiles].sort((left, right) => {
    const leftRank = rank.get(left.folder_name) ?? Number.MAX_SAFE_INTEGER;
    const rightRank = rank.get(right.folder_name) ?? Number.MAX_SAFE_INTEGER;
    return leftRank - rightRank;
  });
}

export function buildPaging(totalProfiles: number, pageSize: number, page: number): PagingInfo {
  const totalPages = Math.max(1, Math.ceil(totalProfiles / pageSize));
  const nextPage = Math.min(Math.max(1, page), totalPages);

  return {
    page: nextPage,
    page_size: pageSize,
    total_profiles: totalProfiles,
    total_pages: totalPages,
    has_previous: nextPage > 1,
    has_next: nextPage < totalPages,
  };
}

export function buildDashboardViewModel(): DashboardViewModel | null {
  if (!state.snapshot) {
    return null;
  }

  const paging = buildPaging(state.snapshot.profiles.length, state.pageSize, state.page);
  const start = (paging.page - 1) * paging.page_size;
  const end = start + paging.page_size;
  state.page = paging.page;

  return {
    paging,
    profiles: state.snapshot.profiles.slice(start, end),
    current_card: state.snapshot.current_card,
    current_quota_card: state.currentQuota ?? state.snapshot.current_quota_card,
  };
}

export function applySnapshot(snapshot: ProfilesSnapshotResponse): void {
  state.snapshot = {
    ...snapshot,
    profiles: applyProfileOrder(snapshot.profiles),
  };
  state.pageSize = snapshot.page_size;
  state.currentProfile = snapshot.current_card?.folder_name ?? null;
  state.currentQuota = snapshot.current_quota_card;
  state.page = buildPaging(snapshot.profiles.length, snapshot.page_size, state.page).page;
}

export function applyCurrentQuota(response: CurrentQuotaResponse): void {
  const currentProfile = state.snapshot?.current_card?.folder_name ?? null;

  if (!response.profile) {
    if (!currentProfile) {
      state.currentQuota = null;
    }
    return;
  }

  if (response.profile === currentProfile) {
    state.currentQuota = response.quota;
    if (state.snapshot) {
      state.snapshot = {
        ...state.snapshot,
        current_quota_card: response.quota,
        profiles: state.snapshot.profiles.map((profile) => (
          profile.folder_name === response.profile
            ? { ...profile, quota: response.quota ?? profile.quota }
            : profile
        )),
      };
    }
  }
}
