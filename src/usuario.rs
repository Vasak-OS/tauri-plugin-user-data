//! Quién es la persona que usa la sesión.
//!
//! Todo lo que hay acá es puro o casi puro, para poder probarlo: son las partes
//! que se equivocan **en silencio**. Un nombre mal parseado sale en el panel del
//! escritorio, y un avatar con el tipo MIME equivocado no se dibuja y deja un
//! hueco donde va la foto.

use std::ffi::CStr;
use std::path::PathBuf;

/// Hasta qué tamaño se lee un avatar.
///
/// El avatar viaja al WebView en base64, que agrega un tercio, dentro de un solo
/// mensaje de IPC. Un `.face` de cincuenta megas —nada impide ponerlo— serían
/// sesenta y siete de mensaje para una foto de treinta y dos píxeles.
pub const LIMITE_AVATAR: u64 = 4 * 1024 * 1024;

/// El nombre completo, sacado de una línea de `passwd`.
///
/// El quinto campo es GECOS, y por convención lleva varios valores separados por
/// comas: nombre, oficina, teléfono. Sólo el primero es el nombre. Si no hay nada
/// útil se cae al nombre de la cuenta, porque un panel que dice «pato» es mejor que
/// uno que no dice nada.
pub fn nombre_completo_de(linea_de_passwd: &str, usuario: &str) -> String {
    let gecos = linea_de_passwd.split(':').nth(4).unwrap_or("");
    let nombre = gecos.split(',').next().unwrap_or("").trim();
    if nombre.is_empty() {
        usuario.to_string()
    } else {
        nombre.to_string()
    }
}

/// El directorio personal según una línea de `passwd`.
pub fn hogar_de(linea_de_passwd: &str) -> Option<String> {
    let hogar = linea_de_passwd.split(':').nth(5).unwrap_or("").trim();
    (!hogar.is_empty()).then(|| hogar.to_string())
}

/// El tipo MIME de una imagen, por sus bytes.
///
/// Por contenido y no por la extensión. Dos de las cuatro rutas de avatar no
/// tienen extensión —`~/.face` y el icono de AccountsService, que se llama como la
/// cuenta— así que mirando el nombre siempre salía «image/png», y un `.face` que
/// en realidad es JPEG no se dibujaba: el hueco de la foto quedaba vacío.
pub fn mime_de(bytes: &[u8]) -> &'static str {
    const PNG: &[u8] = &[0x89, b'P', b'N', b'G'];
    if bytes.starts_with(PNG) {
        return "image/png";
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return "image/jpeg";
    }
    if bytes.starts_with(b"GIF8") {
        return "image/gif";
    }
    if bytes.starts_with(b"BM") {
        return "image/bmp";
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return "image/webp";
    }
    // El SVG es texto y puede empezar con la declaración XML, con un comentario o
    // con espacios, así que se busca la etiqueta en el principio del archivo.
    let principio = &bytes[..bytes.len().min(512)];
    if let Ok(texto) = std::str::from_utf8(principio) {
        if texto.contains("<svg") {
            return "image/svg+xml";
        }
    }
    // Lo que no se reconoce se declara como PNG, que es lo que había antes: el
    // motor igual olfatea el contenido, y declarar algo es mejor que nada.
    "image/png"
}

/// Dónde puede estar la foto de la cuenta, en orden.
pub fn rutas_de_avatar(hogar: &str, usuario: &str) -> Vec<PathBuf> {
    let mut rutas = Vec::new();
    if !hogar.is_empty() {
        rutas.push(PathBuf::from(hogar).join(".face"));
        rutas.push(PathBuf::from(hogar).join(".face.icon"));
    }
    rutas.push(PathBuf::from("/var/lib/AccountsService/icons").join(usuario));
    rutas.push(PathBuf::from("/usr/share/icons/default/user.png"));
    rutas
}

/// El avatar como `data:` URL, de la primera ruta que sirva.
///
/// Una ruta que no se puede leer, que pasa el límite o que está vacía se **salta**
/// y se sigue con la siguiente: perder la foto por un archivo roto cuando hay otro
/// bueno más abajo sería tonto. `None` cuando no sirvió ninguna, y ahí quien llama
/// pone el de reserva.
pub fn avatar_de(rutas: &[PathBuf], limite: u64) -> Option<String> {
    for ruta in rutas {
        let cabe = std::fs::metadata(ruta)
            .map(|m| m.is_file() && m.len() > 0 && m.len() <= limite)
            .unwrap_or(false);
        if !cabe {
            continue;
        }
        let Ok(datos) = std::fs::read(ruta) else {
            continue;
        };
        let mime = mime_de(&datos);
        let base64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &datos);
        return Some(format!("data:{mime};base64,{base64}"));
    }
    None
}

/// El nombre de la cuenta, sin depender del entorno.
///
/// `USER` primero porque es lo normal, pero **con salida**: una aplicación lanzada
/// desde una unidad de systemd puede no tenerla puesta, y ahí la versión anterior
/// fallaba entera y el panel quedaba sin nombre, sin foto y sin nada. El uid
/// siempre está.
pub fn nombre_de_la_cuenta() -> Option<String> {
    if let Ok(usuario) = std::env::var("USER") {
        if !usuario.trim().is_empty() {
            return Some(usuario);
        }
    }

    // `getpwuid` no es reentrante, pero esto corre una vez al arrancar y desde un
    // solo hilo.
    unsafe {
        let entrada = libc::getpwuid(libc::getuid());
        if entrada.is_null() {
            return None;
        }
        CStr::from_ptr((*entrada).pw_name)
            .to_str()
            .ok()
            .map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Una línea de `passwd` de verdad, con GECOS de varios campos.
    const LINEA: &str = "pato:x:1000:1000:Joaquin Decima,Oficina 3,555-1234:/home/pato:/usr/bin/zsh";

    #[test]
    fn del_gecos_se_toma_solo_el_nombre() {
        // GECOS lleva nombre, oficina y teléfono separados por comas. Sin cortar en
        // la primera, el panel del escritorio mostraría el teléfono al lado del
        // nombre.
        assert_eq!(nombre_completo_de(LINEA, "pato"), "Joaquin Decima");
    }

    #[test]
    fn sin_nombre_se_usa_el_de_la_cuenta() {
        // Un panel que dice «pato» es mejor que uno que no dice nada.
        assert_eq!(nombre_completo_de("pato:x:1000:1000::/home/pato:/bin/sh", "pato"), "pato");
        assert_eq!(nombre_completo_de("", "pato"), "pato");
        assert_eq!(nombre_completo_de("pato:x:1000:1000:   :/home/pato:/bin/sh", "pato"), "pato");
    }

    #[test]
    fn una_linea_incompleta_no_rompe_nada() {
        // Pasa cuando `getent` no encuentra la cuenta y devuelve vacío.
        assert_eq!(nombre_completo_de("pato:x:1000", "pato"), "pato");
        assert_eq!(hogar_de("pato:x:1000"), None);
    }

    #[test]
    fn el_hogar_sale_del_passwd() {
        // Y no de `HOME`, que una unidad de systemd puede no tener puesta.
        assert_eq!(hogar_de(LINEA), Some("/home/pato".to_string()));
        assert_eq!(hogar_de("pato:x:1000:1000:n::/bin/sh"), None);
    }

    #[test]
    fn el_tipo_se_saca_del_contenido_y_no_del_nombre() {
        // Dos de las cuatro rutas de avatar no tienen extensión, así que mirando el
        // nombre siempre salía «image/png» y un JPEG no se dibujaba.
        assert_eq!(mime_de(&[0x89, b'P', b'N', b'G', 0x0D]), "image/png");
        assert_eq!(mime_de(&[0xFF, 0xD8, 0xFF, 0xE0]), "image/jpeg");
        assert_eq!(mime_de(b"GIF89a..."), "image/gif");
        assert_eq!(mime_de(b"BM..."), "image/bmp");
        assert_eq!(mime_de(b"RIFF\0\0\0\0WEBPVP8 "), "image/webp");
    }

    #[test]
    fn un_svg_se_reconoce_con_y_sin_declaracion_xml() {
        assert_eq!(mime_de(br#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#), "image/svg+xml");
        assert_eq!(
            mime_de(br#"<?xml version="1.0"?><svg xmlns="x"></svg>"#),
            "image/svg+xml"
        );
        assert_eq!(mime_de(b"\n  <!-- una foto --> <svg></svg>"), "image/svg+xml");
    }

    #[test]
    fn un_riff_que_no_es_webp_no_se_declara_webp() {
        // Un WAV empieza con RIFF y no es una imagen.
        assert_eq!(mime_de(b"RIFF\0\0\0\0WAVEfmt "), "image/png");
    }

    #[test]
    fn unos_bytes_sueltos_no_hacen_panicar_al_olfateo() {
        // El archivo puede estar truncado o vacío, y esto corre al arrancar el
        // escritorio: un panic acá es un panel que no abre.
        assert_eq!(mime_de(b""), "image/png");
        assert_eq!(mime_de(b"R"), "image/png");
        assert_eq!(mime_de(&[0x89]), "image/png");
        assert_eq!(mime_de(&[0xFF, 0xD8]), "image/png");
        assert_eq!(mime_de(&[0xFF; 3]), "image/png");
    }

    #[test]
    fn el_avatar_de_la_persona_le_gana_al_del_sistema() {
        // Si no, todo el mundo aparece con el icono genérico aunque tenga foto.
        let rutas = rutas_de_avatar("/home/pato", "pato");
        assert_eq!(rutas[0], PathBuf::from("/home/pato/.face"));
        let generico = rutas
            .iter()
            .position(|r| r.starts_with("/usr/share"))
            .expect("hay uno genérico");
        assert!(generico > 0, "el genérico va último");
        assert_eq!(generico, rutas.len() - 1);
    }

    #[test]
    fn sin_hogar_igual_se_busca_el_del_sistema() {
        // Sin esto quedarían rutas como `/.face`, que no es de nadie.
        let rutas = rutas_de_avatar("", "pato");
        assert!(!rutas.iter().any(|r| r == &PathBuf::from("/.face")), "{rutas:?}");
        assert!(rutas.iter().any(|r| r.ends_with("icons/pato")));
    }

    #[test]
    fn siempre_hay_un_nombre_de_cuenta() {
        // Aunque `USER` no esté: una aplicación lanzada desde una unidad de systemd
        // puede no tenerla, y antes fallaba entera y el panel quedaba sin nada.
        assert!(nombre_de_la_cuenta().is_some_and(|n| !n.is_empty()));
    }

    /// Un PNG de 1x1 de verdad.
    const PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89,
    ];

    fn escenario(quien: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("user-data-{}-{quien}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn se_lee_el_primero_que_exista_y_con_su_tipo() {
        let base = escenario("orden");
        let segunda = base.join("segunda.jpg");
        std::fs::write(&segunda, [0xFF, 0xD8, 0xFF, 0xE0]).unwrap();

        let url = avatar_de(&[base.join("no-existe"), segunda], LIMITE_AVATAR).expect("hay avatar");
        assert!(url.starts_with("data:image/jpeg;base64,"), "{url}");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn un_avatar_enorme_se_salta_en_lugar_de_cargarse() {
        // Viaja al WebView en base64, que agrega un tercio, en un solo mensaje de
        // IPC: un `.face` de cincuenta megas serían sesenta y siete de mensaje para
        // una foto de treinta y dos píxeles.
        let base = escenario("enorme");
        let gordo = base.join("gordo.png");
        let f = std::fs::File::create(&gordo).unwrap();
        f.set_len(LIMITE_AVATAR + 1).unwrap();

        let chico = base.join("chico.png");
        std::fs::write(&chico, PNG).unwrap();

        // El gordo se salta y se sigue con el siguiente, que es lo que hace que la
        // persona igual tenga foto.
        let url = avatar_de(&[gordo, chico], LIMITE_AVATAR).expect("cae al siguiente");
        assert!(url.starts_with("data:image/png;base64,"), "{url}");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn sin_ninguna_ruta_legible_no_hay_avatar() {
        // Quien llama pone el de reserva; acá no se inventa uno.
        assert_eq!(avatar_de(&[PathBuf::from("/no/existe.png")], LIMITE_AVATAR), None);
        assert_eq!(avatar_de(&[], LIMITE_AVATAR), None);
    }

    #[test]
    fn un_avatar_vacio_no_pasa_por_avatar() {
        // Un archivo de cero bytes daría un `data:` URL sin datos, y el hueco de la
        // foto queda vacío igual: mejor seguir buscando.
        let base = escenario("vacio");
        let vacio = base.join("vacio.png");
        std::fs::write(&vacio, b"").unwrap();
        let chico = base.join("chico.png");
        std::fs::write(&chico, PNG).unwrap();

        let url = avatar_de(&[vacio, chico], LIMITE_AVATAR).expect("cae al siguiente");
        assert!(url.len() > "data:image/png;base64,".len());
        let _ = std::fs::remove_dir_all(&base);
    }
}
