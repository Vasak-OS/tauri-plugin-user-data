import { invoke } from '@tauri-apps/api/core'

export async function getUserData(): Promise<UserInfo | null> {
  return await invoke< UserInfo >('plugin:user-data|get_user_info')
    .then((r) => (r ? r : null));
}

export interface UserInfo {
  username: string;
  full_name: string;
  avatar_data: string;
}