//! Lo que la interfaz puede preguntar sobre la persona que usa la sesión.

use serde::Serialize;
use std::process::Command;
use std::str::from_utf8;

use crate::usuario::{self, LIMITE_AVATAR};

/// El icono genérico, para cuando no hay ninguna foto.
///
/// Va empotrado y no como ruta a un archivo del tema: si el tema no lo tiene, el
/// panel del escritorio se queda con un hueco donde va la cara.
const AVATAR_DE_RESERVA: &str = "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIyNCIgaGVpZ2h0PSIyNCIgdmlld0JveD0iMCAwIDI0IDI0IiBmaWxsPSJub25lIiBzdHJva2U9ImN1cnJlbnRDb2xvciIgc3Ryb2tlLXdpZHRoPSIyIiBzdHJva2UtbGluZWNhcD0icm91bmQiIHN0cm9rZS1saW5lam9pbj0icm91bmQiPjxwYXRoIGQ9Ik0yMCAyMXYtMmE0IDQgMCAwIDAtNC00SDhhNCA0IDAgMCAwLTQgNHYyIj48L3BhdGg+PGNpcmNsZSBjeD0iMTIiIGN5PSI3IiByPSI0Ij48L2NpcmNsZT48L3N2Zz4=";

#[derive(Debug, Serialize, Clone)]
pub struct UserInfo {
    username: String,
    full_name: String,
    avatar_data: String,
    home_dir: String,
}

/// Quién es y cómo se ve.
///
/// **No falla si el entorno está incompleto.** Antes leía `USER` con `?`, así que
/// una aplicación lanzada desde una unidad de systemd —donde esa variable puede no
/// estar— se quedaba sin nombre, sin foto y sin directorio: el panel del escritorio
/// entero sin datos. Ahora el uid es la fuente de última instancia, que siempre
/// está, y el directorio sale del `passwd` con `HOME` como respaldo.
#[tauri::command]
pub fn get_user_info() -> Result<UserInfo, String> {
    // Siempre devuelve algo: `USER`, el `passwd`, o el uid como número.
    let username = usuario::nombre_de_la_cuenta();

    // La línea de `passwd`. Si no se puede leer, se sigue con lo que haya: quedarse
    // sin panel por no saber el nombre completo sería peor que mostrar la cuenta.
    let linea = Command::new("getent")
        .args(["passwd", &username])
        .output()
        .ok()
        .and_then(|salida| from_utf8(&salida.stdout).ok().map(|t| t.to_string()))
        .unwrap_or_default();

    let full_name = usuario::nombre_completo_de(&linea, &username);
    let home_dir = usuario::hogar_de(&linea)
        .or_else(|| std::env::var("HOME").ok())
        .unwrap_or_default();

    let avatar_data = usuario::avatar_de(
        &usuario::rutas_de_avatar(&home_dir, &username),
        LIMITE_AVATAR,
    )
    .unwrap_or_else(|| AVATAR_DE_RESERVA.to_string());

    Ok(UserInfo {
        username,
        full_name,
        avatar_data,
        home_dir,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn siempre_se_devuelve_algo_utilizable() {
        // El panel del escritorio pide esto al arrancar: si falla, no muestra nada.
        let info = get_user_info().expect("tiene que resolver");
        assert!(!info.username.is_empty());
        assert!(!info.full_name.is_empty(), "al menos el nombre de la cuenta");
        assert!(info.avatar_data.starts_with("data:image/"), "{}", info.avatar_data);
        assert!(!info.home_dir.is_empty());
    }

    #[test]
    fn el_avatar_de_reserva_es_una_imagen_valida() {
        // Si estuviera mal, todo el mundo sin foto vería un hueco.
        assert!(AVATAR_DE_RESERVA.starts_with("data:image/svg+xml;base64,"));
        let carga = AVATAR_DE_RESERVA.trim_start_matches("data:image/svg+xml;base64,");
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, carga)
            .expect("base64 válido");
        let texto = String::from_utf8(bytes).expect("utf-8");
        assert!(texto.contains("<svg"), "{texto}");
        assert!(texto.contains("</svg>"));
    }
}
