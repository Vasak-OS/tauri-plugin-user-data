use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

pub fn init<R: Runtime, C: DeserializeOwned>(
  app: &AppHandle<R>,
  _api: PluginApi<R, C>,
) -> crate::Result<UserData<R>> {
  Ok(UserData(app.clone()))
}

/// Access to the user-data APIs.
pub struct UserData<R: Runtime>(AppHandle<R>);
