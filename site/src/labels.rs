//! The few texts the islands need in the browser. Kept apart from the full dictionary
//! (i18n.rs) so the WebAssembly bundle only carries these.

use crate::routes::Lang;

pub struct Labels {
    pub dust: &'static str,
    pub particles: &'static str,
    pub tabs: [&'static str; 4],
    pub title: &'static str,
    pub description: &'static str,
    pub path: &'static str,
    pub title_value: &'static str,
    pub description_value: &'static str,
    pub path_value: &'static str,
    pub output: &'static str,
    pub google: &'static str,
    pub social: &'static str,
    pub server: &'static str,
    pub browser: &'static str,
    pub search: &'static str,
    pub search_hint: &'static str,
    pub no_results: &'static str,
}

pub fn labels(lang: Lang) -> &'static Labels {
    match lang {
        Lang::En => &EN,
        Lang::Fr => &FR,
        Lang::Es => &ES,
    }
}

static EN: Labels = Labels {
    dust: "This dust is simulated in Rust",
    particles: "particles",
    tabs: ["Component", "SEO", "Island", "Server"],
    title: "Title",
    description: "Description",
    path: "Page path",
    title_value: "Handmade oak chair",
    description_value: "A solid oak chair, built to last a lifetime and shipped in 48 hours.",
    path_value: "/products/oak-chair",
    output: "Generated <head>",
    google: "Search result",
    social: "Social card",
    server: "rendered on the server",
    browser: "Rust in your browser",
    search: "Search the docs",
    search_hint: "Type to search…",
    no_results: "No results",
};

static FR: Labels = Labels {
    dust: "Cette poussière est simulée en Rust",
    particles: "particules",
    tabs: ["Composant", "SEO", "Îlot", "Serveur"],
    title: "Titre",
    description: "Description",
    path: "Chemin de la page",
    title_value: "Chaise en chêne artisanale",
    description_value: "Une chaise en chêne massif, faite pour durer toute une vie et livrée en 48 heures.",
    path_value: "/produits/chaise-chene",
    output: "<head> généré",
    google: "Résultat de recherche",
    social: "Carte sociale",
    server: "rendu sur le serveur",
    browser: "Rust dans votre navigateur",
    search: "Rechercher dans la doc",
    search_hint: "Tapez pour chercher…",
    no_results: "Aucun résultat",
};

static ES: Labels = Labels {
    dust: "Este polvo se simula en Rust",
    particles: "partículas",
    tabs: ["Componente", "SEO", "Isla", "Servidor"],
    title: "Título",
    description: "Descripción",
    path: "Ruta de la página",
    title_value: "Silla artesanal de roble",
    description_value: "Una silla de roble macizo, hecha para durar toda la vida y enviada en 48 horas.",
    path_value: "/productos/silla-roble",
    output: "<head> generado",
    google: "Resultado de búsqueda",
    social: "Tarjeta social",
    server: "renderizado en el servidor",
    browser: "Rust en tu navegador",
    search: "Buscar en la documentación",
    search_hint: "Escribe para buscar…",
    no_results: "Sin resultados",
};

/// `value` with `decimals` digits, without float formatting machinery (keeps the
/// WebAssembly bundle small): `fixed(0.745, 2) == "0.75"`.
pub fn fixed(value: f64, decimals: u32) -> String {
    let scale = 10u64.pow(decimals);
    let negative = value < 0.0;
    let n = (value.abs() * scale as f64 + 0.5) as u64;
    let (int, frac) = (n / scale, n % scale);
    let sign = if negative && n > 0 { "-" } else { "" };
    if decimals == 0 {
        format!("{sign}{int}")
    } else {
        format!("{sign}{int}.{frac:0width$}", width = decimals as usize)
    }
}

/// Parses `"12"`, `"0.74"`, `"-3.5"` without the float parser.
pub fn parse_decimal(s: &str) -> f64 {
    let (sign, s) = match s.strip_prefix('-') {
        Some(rest) => (-1.0, rest),
        None => (1.0, s),
    };
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    let int: u64 = int.parse().unwrap_or(0);
    let digits = frac.len().min(9) as u32;
    let frac: u64 = frac
        .get(..digits as usize)
        .and_then(|f| f.parse().ok())
        .unwrap_or(0);
    sign * (int as f64 + frac as f64 / 10u64.pow(digits) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_and_parses_decimals_without_floats_machinery() {
        assert_eq!(fixed(0.745, 2), "0.75");
        assert_eq!(fixed(12.0, 0), "12");
        assert_eq!(fixed(-1.05, 1), "-1.1");
        assert_eq!(fixed(0.0001, 4), "0.0001");
        assert_eq!(parse_decimal("0.74"), 0.74);
        assert_eq!(parse_decimal("-3.5"), -3.5);
        assert_eq!(parse_decimal("100"), 100.0);
    }
}
