pub mod eredivisie;
pub mod kkd;

use std::sync::LazyLock;

use chrono::Utc;
use scraper::{ElementRef, Html, Selector};
use tracing::{error, info};

use crate::models::{StandingsData, Team};

/// Minimum number of cells a standings row must have.
const MIN_CELLS: usize = 7;
/// Below this many parsed teams the scrape is treated as failed.
const MIN_TEAMS: usize = 10;

static CELL: LazyLock<Selector> = LazyLock::new(|| Selector::parse("td, th").unwrap());
static ROW: LazyLock<Selector> = LazyLock::new(|| Selector::parse("tr").unwrap());

pub fn safe_int(value: &str) -> i32 {
    let cleaned: String = value
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    cleaned.parse::<i32>().unwrap_or(0)
}

/// Collapses whitespace and strips the legend markers (`*`, `+`) sites append to names.
pub fn clean_team_name(name: &str) -> String {
    name.replace(['*', '+'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Splits `text` on `sep` into `N` integers, e.g. "7 | 4 | 1" -> [7, 4, 1]. Missing parts become 0.
pub fn split_ints<const N: usize>(text: &str, sep: char) -> [i32; N] {
    let mut parts = text.split(sep).map(safe_int);
    std::array::from_fn(|_| parts.next().unwrap_or(0))
}

pub async fn fetch_html(client: &reqwest::Client, url: &str) -> Option<String> {
    let result = async {
        client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await
    };
    result
        .await
        .inspect_err(|e| error!("Error fetching {url}: {e}"))
        .ok()
}

/// All cells of a row, or `None` when the row is too short to be a standings row.
pub fn row_cells<'a>(row: &ElementRef<'a>) -> Option<Vec<ElementRef<'a>>> {
    let cells: Vec<_> = row.select(&CELL).collect();
    (cells.len() >= MIN_CELLS).then_some(cells)
}

/// Text of cell `i`, or an empty string when the cell is missing.
pub fn cell_text(cells: &[ElementRef], i: usize) -> String {
    cells.get(i).map(|c| c.text().collect()).unwrap_or_default()
}

/// Finds the standings rows. Tries each selector in order and takes the first that
/// yields at least `expected` rows. Falls back to any row whose cell at `pos_cell`
/// holds a position between 1 and `expected`.
pub fn find_rows<'a>(
    document: &'a Html,
    selectors: &[&str],
    pos_cell: usize,
    expected: usize,
) -> Vec<ElementRef<'a>> {
    for selector_str in selectors {
        let selector = Selector::parse(selector_str).unwrap();
        let found: Vec<_> = document.select(&selector).collect();
        if found.len() >= expected {
            info!("Found table using selector: {selector_str}");
            return found;
        }
    }

    document
        .select(&ROW)
        .filter(|row| {
            row_cells(row).is_some_and(|cells| {
                cell_text(&cells, pos_cell)
                    .trim()
                    .parse::<usize>()
                    .is_ok_and(|n| (1..=expected).contains(&n))
            })
        })
        .collect()
}

/// Validates the scraped teams and wraps them in `StandingsData`.
pub fn finish(league: &str, mut teams: Vec<Team>, expected: usize) -> Option<StandingsData> {
    if teams.len() < MIN_TEAMS {
        error!(
            "{league}: only found {} teams, expected {expected}",
            teams.len()
        );
        return None;
    }
    teams.sort_by_key(|t| t.position);
    info!("{league}: scraped {} teams", teams.len());
    Some(StandingsData {
        teams,
        last_updated: Utc::now().to_rfc3339(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_int_parses_negative_and_ignores_junk() {
        assert_eq!(safe_int(" -3 "), -3);
        assert_eq!(safe_int("12pt"), 12);
        assert_eq!(safe_int(""), 0);
        assert_eq!(safe_int("n/a"), 0);
    }

    #[test]
    fn clean_team_name_strips_asterisks_and_collapses_whitespace() {
        assert_eq!(clean_team_name("  Ajax \n  Amsterdam* "), "Ajax Amsterdam");
        assert_eq!(clean_team_name("Go * Ahead"), "Go Ahead");
        assert_eq!(clean_team_name("Heracles Almelo +"), "Heracles Almelo");
    }
}
