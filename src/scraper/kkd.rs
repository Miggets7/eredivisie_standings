use std::sync::LazyLock;

use scraper::{ElementRef, Html, Selector};
use serde::Deserialize;
use serde_json::Value;
use tracing::warn;

use crate::models::{StandingsData, Team};
use crate::scraper::{
    cell_text, clean_team_name, fetch_html, find_rows, finish, row_cells, safe_int, split_ints,
};

const URL: &str = "https://keukenkampioendivisie.nl/klassement";
const EXPECTED_TEAMS: usize = 20;

const TABLE_SELECTORS: [&str; 2] = ["table.table-medium tbody tr", "table tbody tr"];

static NEXT_DATA: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("script#__NEXT_DATA__").unwrap());
static IMG: LazyLock<Selector> = LazyLock::new(|| Selector::parse("img").unwrap());

/// A cleaned name, if it is long enough to be a real team name.
fn valid_name(raw: &str) -> Option<String> {
    let name = clean_team_name(raw);
    (name.len() >= 2).then_some(name)
}

/// Club name from cell 3, or the logo's alt text in cell 2.
fn extract_team_name(cells: &[ElementRef]) -> Option<String> {
    valid_name(&cell_text(cells, 3)).or_else(|| {
        let alt = cells[2].select(&IMG).next()?.attr("alt")?;
        valid_name(alt)
    })
}

pub async fn scrape_kkd(client: &reqwest::Client) -> Option<StandingsData> {
    let html = fetch_html(client, URL).await?;
    finish("KKD", parse(&html), EXPECTED_TEAMS)
}

fn parse(html: &str) -> Vec<Team> {
    let document = Html::parse_document(html);
    teams_from_next_data(&document).unwrap_or_else(|| {
        warn!("KKD: no standings in __NEXT_DATA__, falling back to HTML table");
        teams_from_table(&document)
    })
}

/// One entry of `props.pageProps.rankingsData` in the page's Next.js payload.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Ranking {
    rank: i32,
    contestant_club_name: String,
    matches_played: i32,
    matches_won: i32,
    matches_drawn: i32,
    matches_lost: i32,
    goals_for: i32,
    goals_against: i32,
    points: i32,
}

impl From<Ranking> for Team {
    fn from(r: Ranking) -> Self {
        Team {
            position: r.rank,
            name: r.contestant_club_name,
            games: r.matches_played,
            wins: r.matches_won,
            draws: r.matches_drawn,
            losses: r.matches_lost,
            goals_for: r.goals_for,
            goals_against: r.goals_against,
            goal_difference: r.goals_for - r.goals_against,
            points: r.points,
        }
    }
}

/// Standings from the structured data the page embeds for Next.js hydration.
fn teams_from_next_data(document: &Html) -> Option<Vec<Team>> {
    let script = document.select(&NEXT_DATA).next()?;
    let mut data: Value = serde_json::from_str(&script.text().collect::<String>())
        .inspect_err(|e| warn!("KKD: invalid __NEXT_DATA__ JSON: {e}"))
        .ok()?;
    let rankings = data.pointer_mut("/props/pageProps/rankingsData")?.take();
    let rankings: Vec<Ranking> = serde_json::from_value(rankings)
        .inspect_err(|e| warn!("KKD: unexpected rankingsData shape: {e}"))
        .ok()?;
    (!rankings.is_empty()).then(|| rankings.into_iter().map(Team::from).collect())
}

/// Fallback: column positions in the rendered table.
/// Columns: legend | # | logo | Club | GS | W / G / V | P | DV / DT | DS
fn teams_from_table(document: &Html) -> Vec<Team> {
    let rows = find_rows(document, &TABLE_SELECTORS, 1, EXPECTED_TEAMS);

    let mut teams = Vec::new();
    for (i, row) in rows.iter().take(EXPECTED_TEAMS).enumerate() {
        let Some(cells) = row_cells(row) else {
            continue;
        };

        let position = match safe_int(&cell_text(&cells, 1)) {
            pos if (1..=EXPECTED_TEAMS as i32).contains(&pos) => pos,
            _ => (i + 1) as i32,
        };

        let Some(name) = extract_team_name(&cells) else {
            warn!("Could not extract team name from row {}", i + 1);
            continue;
        };

        // "W / G / V" in cell 5: won, drawn, lost
        let [wins, draws, losses] = split_ints(&cell_text(&cells, 5), '/');
        let [goals_for, goals_against] = split_ints(&cell_text(&cells, 7), '/');

        teams.push(Team {
            position,
            name,
            games: safe_int(&cell_text(&cells, 4)),
            wins,
            draws,
            losses,
            goals_for,
            goals_against,
            goal_difference: safe_int(&cell_text(&cells, 8)),
            points: safe_int(&cell_text(&cells, 6)),
        });
    }
    teams
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Table and `__NEXT_DATA__` script trimmed from the live page.
    const PAGE: &str = include_str!("../../tests/fixtures/kkd.html");

    #[test]
    fn parses_next_data_from_real_page() {
        let doc = Html::parse_document(PAGE);
        let teams = teams_from_next_data(&doc).expect("JSON path must succeed");

        assert_eq!(teams.len(), EXPECTED_TEAMS);
        let t = &teams[0];
        assert_eq!((t.position, t.name.as_str()), (1, "Heracles Almelo"));
        assert_eq!((t.games, t.wins, t.draws, t.losses), (14, 11, 2, 1));
        assert_eq!(
            (t.goals_for, t.goals_against, t.goal_difference),
            (44, 15, 29)
        );
        assert_eq!(t.points, 35);
    }

    #[test]
    fn html_fallback_matches_next_data() {
        let doc = Html::parse_document(PAGE);
        let from_json = teams_from_next_data(&doc).expect("JSON path must succeed");

        assert_eq!(teams_from_table(&doc), from_json);
    }
}
