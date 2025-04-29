use serde::Serialize;
use std::process::Command;
use std::str::from_utf8;
use std::env;
use std::fs;

#[derive(Debug, Serialize, Clone)]
pub struct UserInfo {
    username: String,
    full_name: String,
    avatar_data: String,
    home_dir: String,
}

#[tauri::command]
pub fn get_user_info() -> Result<UserInfo, String> {
    // Obtener nombre de usuario actual
    let username = env::var("USER")
        .map_err(|e| format!("Error getting username: {}", e))?;

    // Obtener nombre completo desde el comando getent
    let output = Command::new("getent")
        .args(["passwd", &username])
        .output()
        .map_err(|e| format!("Error executing getent: {}", e))?;

    let passwd_entry = from_utf8(&output.stdout)
        .map_err(|e| format!("Invalid UTF-8 in output: {}", e))?;

    let full_name = passwd_entry
        .split(':')
        .nth(4)
        .unwrap_or(&username)
        .split(',')
        .next()
        .unwrap_or(&username)
        .to_string();

    // Intentar leer el avatar del usuario
    let avatar_data = get_user_avatar(&username)?;
    let home_dir = env::var("HOME").unwrap_or_default();

    Ok(UserInfo {
        username,
        full_name,
        avatar_data,
        home_dir,
    })
}

fn get_user_avatar(username: &str) -> Result<String, String> {
    let home_dir = env::var("HOME").unwrap_or_default();
    
    // Lista de posibles ubicaciones del avatar
    let avatar_paths = vec![
        format!("{}/.face", home_dir),
        format!("/var/lib/AccountsService/icons/{}", username),
        "/usr/share/icons/default/user.png".to_string(),
        "/usr/share/icons/gnome/scalable/apps/system-users-symbolic.svg".to_string(),
    ];

    // Intentar leer la primera imagen que exista
    for path in avatar_paths {
        if let Ok(data) = fs::read(&path) {
            // Detectar el tipo MIME
            let mime_type = match path.split('.').last() {
                Some("png") => "image/png",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("svg") => "image/svg+xml",
                _ => "image/png", // default
            };

            // Convertir a base64
            let base64 = base64::encode(&data);
            return Ok(format!("data:{};base64,{}", mime_type, base64));
        }
    }

    // Si no se encuentra ninguna imagen, usar un SVG por defecto
    Ok("data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIyNCIgaGVpZ2h0PSIyNCIgdmlld0JveD0iMCAwIDI0IDI0IiBmaWxsPSJub25lIiBzdHJva2U9ImN1cnJlbnRDb2xvciIgc3Ryb2tlLXdpZHRoPSIyIiBzdHJva2UtbGluZWNhcD0icm91bmQiIHN0cm9rZS1saW5lam9pbj0icm91bmQiPjxwYXRoIGQ9Ik0yMCAyMXYtMmE0IDQgMCAwIDAtNC00SDhhNCA0IDAgMCAwLTQgNHYyIj48L3BhdGg+PGNpcmNsZSBjeD0iMTIiIGN5PSI3IiByPSI0Ij48L2NpcmNsZT48L3N2Zz4=".to_string())
}