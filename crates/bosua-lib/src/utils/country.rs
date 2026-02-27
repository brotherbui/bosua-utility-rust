use std::collections::HashMap;
use std::sync::LazyLock;

static COUNTRY_CODES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("US", "United States");
    m.insert("GB", "United Kingdom");
    m.insert("VN", "Vietnam");
    m.insert("JP", "Japan");
    m.insert("KR", "South Korea");
    m.insert("CN", "China");
    m.insert("TH", "Thailand");
    m.insert("SG", "Singapore");
    m.insert("HK", "Hong Kong");
    m.insert("TW", "Taiwan");
    m.insert("DE", "Germany");
    m.insert("FR", "France");
    m.insert("AU", "Australia");
    m.insert("CA", "Canada");
    m.insert("IN", "India");
    m
});

/// Look up a country name by ISO 3166-1 alpha-2 code.
///
/// The lookup is case-insensitive: both "us" and "US" will match.
/// Returns `None` for unrecognized codes.
pub fn lookup_country(code: &str) -> Option<&'static str> {
    COUNTRY_CODES.get(code.to_uppercase().as_str()).copied()
}

/// Get the public IP address by querying an external API.
///
/// Uses <https://api.ipify.org> to resolve the caller's public IP.
pub async fn get_public_ip() -> crate::errors::Result<String> {
    let client = reqwest::Client::new();
    let resp = client.get("https://api.ipify.org").send().await?;
    Ok(resp.text().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_known_codes() {
        assert_eq!(lookup_country("US"), Some("United States"));
        assert_eq!(lookup_country("GB"), Some("United Kingdom"));
        assert_eq!(lookup_country("VN"), Some("Vietnam"));
        assert_eq!(lookup_country("JP"), Some("Japan"));
        assert_eq!(lookup_country("KR"), Some("South Korea"));
        assert_eq!(lookup_country("CN"), Some("China"));
        assert_eq!(lookup_country("TH"), Some("Thailand"));
        assert_eq!(lookup_country("SG"), Some("Singapore"));
        assert_eq!(lookup_country("HK"), Some("Hong Kong"));
        assert_eq!(lookup_country("TW"), Some("Taiwan"));
        assert_eq!(lookup_country("DE"), Some("Germany"));
        assert_eq!(lookup_country("FR"), Some("France"));
        assert_eq!(lookup_country("AU"), Some("Australia"));
        assert_eq!(lookup_country("CA"), Some("Canada"));
        assert_eq!(lookup_country("IN"), Some("India"));
    }

    #[test]
    fn lookup_case_insensitive() {
        assert_eq!(lookup_country("us"), Some("United States"));
        assert_eq!(lookup_country("vn"), Some("Vietnam"));
        assert_eq!(lookup_country("Jp"), Some("Japan"));
    }

    #[test]
    fn lookup_unknown_code_returns_none() {
        assert_eq!(lookup_country("XX"), None);
        assert_eq!(lookup_country("ZZ"), None);
        assert_eq!(lookup_country(""), None);
    }
}
