# Tauri Plugin user-data

A simple plugin that allows you to get the operating system icons by name (only works on Linux) by getting it in base64 ready to use the src of any image

## Installation

```bash
bun add @vasakgroup/plugin-user-data
```

Add in `cargo.toml`

```toml
[dependencies]
tauri-plugin-user-data = { git = "https://github.com/Vasak-OS/tauri-plugin-user-data", branch = "v2" }
```
In `main.rs` or `lib.rs`, add the following to your `tauri::Builder`:

```rust
use tauri_plugin_user_data::UserDataPlugin;
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_user_data::init()) // this line
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

And add in `src-tauri/compatibilites/default.json`

```json
{
  "permissions": [
    ...
    "user-data:default",
  ]
}

```

## Usage

```ts
import { getUserData } from '@vasakgroup/plugin-user-data';

const icon = await getUserData('folder');
```

in vue

```vue
<script setup lang="ts">
import { getUserData } from '@vasakgroup/plugin-user-data';
import { ref } from 'vue';
const user = ref('');
const getUserInfo = async () => {
  user.value = await getUserData();
};
getUserInfo();
</script>

<template>
  {{ user }}
</template>
```