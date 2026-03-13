pub mod eredivisie;
pub mod kkd;

use std::sync::OnceLock;

use regex::Regex;

fn whitespace_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s+").unwrap())
}

pub fn safe_int(value: &str) -> i32 {
    let cleaned: String = value
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    cleaned.parse::<i32>().unwrap_or(0)
}

pub fn clean_team_name(name: &str) -> String {
    whitespace_regex()
        .replace_all(name.trim(), " ")
        .replace('*', "")
        .trim()
        .to_string()
}

pub async fn fetch_html(client: &reqwest::Client, url: &str) -> Result<String, reqwest::Error> {
    let resp = client.get(url).send().await?.error_for_status()?;
    resp.text().await
}
