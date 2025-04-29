import { invoke } from '@tauri-apps/api/core'

export async function getUserData(value: string): Promise<UserInfo | null> {
  return await invoke<{value?: UserInfo}>('plugin:user-data|get_user_info', {
    payload: {
      value,
    },
  }).then((r) => (r.value ? r.value : null));
}

export interface UserInfo {
  username: string;
  full_name: string;
  avatar_data: string;
}