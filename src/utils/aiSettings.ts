import { invoke } from '@tauri-apps/api/core';

export type AiConfigMode = 'none' | 'personalKey' | 'invite';

export interface AiConfigStatus {
  mode: AiConfigMode;
  hasPersonalKey: boolean;
  hasInviteCode: boolean;
  proxyUrl: string | null;
}

interface RawAiConfigStatus {
  mode?: unknown;
  hasPersonalKey?: unknown;
  hasInviteCode?: unknown;
  proxyUrl?: unknown;
}

export const emptyAiConfigStatus: AiConfigStatus = {
  mode: 'none',
  hasPersonalKey: false,
  hasInviteCode: false,
  proxyUrl: null,
};

function normalizeAiConfigStatus(value: unknown): AiConfigStatus {
  if (!value || typeof value !== 'object') {
    return emptyAiConfigStatus;
  }

  const status = value as RawAiConfigStatus;
  const mode = status.mode === 'personalKey' || status.mode === 'invite' ? status.mode : 'none';

  return {
    mode,
    hasPersonalKey: status.hasPersonalKey === true,
    hasInviteCode: status.hasInviteCode === true,
    proxyUrl: typeof status.proxyUrl === 'string' && status.proxyUrl.trim() ? status.proxyUrl : null,
  };
}

export async function loadAiConfigStatus(): Promise<AiConfigStatus> {
  return normalizeAiConfigStatus(await invoke('load_ai_config_status'));
}

export async function savePersonalAiConfig(deepseekApiKey: string): Promise<AiConfigStatus> {
  return normalizeAiConfigStatus(await invoke('save_personal_ai_config', { request: { deepseekApiKey } }));
}

export async function saveInviteAiConfig(inviteCode: string, proxyUrl: string): Promise<AiConfigStatus> {
  return normalizeAiConfigStatus(await invoke('save_invite_ai_config', { request: { inviteCode, proxyUrl } }));
}

export async function clearAiConfig(): Promise<AiConfigStatus> {
  return normalizeAiConfigStatus(await invoke('clear_ai_config'));
}

export async function getAppLocalDataDir(): Promise<string> {
  const path = await invoke('get_app_local_data_dir');
  return typeof path === 'string' ? path : '';
}
