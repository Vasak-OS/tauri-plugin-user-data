use tauri::{
  plugin::{Builder, TauriPlugin},
  Manager, Runtime,
};

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::UserData;
#[cfg(mobile)]
use mobile::UserData;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the user-data APIs.
pub trait UserDataExt<R: Runtime> {
  fn user_data(&self) -> &UserData<R>;
}

impl<R: Runtime, T: Manager<R>> crate::UserDataExt<R> for T {
  fn user_data(&self) -> &UserData<R> {
    self.state::<UserData<R>>().inner()
  }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("user-data")
    .invoke_handler(tauri::generate_handler![commands::get_user_info])
    .setup(|app, api| {
      #[cfg(mobile)]
      let user_data = mobile::init(app, api)?;
      #[cfg(desktop)]
      let user_data = desktop::init(app, api)?;
      app.manage(user_data);
      Ok(())
    })
    .build()
}
