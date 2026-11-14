use scraper::Html;

use crate::models::{StandingsData, Team};
use crate::scraper::{
    cell_img_attr, cell_text, clean_team_name, fetch_html, find_rows, finish, row_cells, safe_int,
    split_ints,
};

const URL: &str = "https://eredivisie.nl/competitie/stand/";
const EXPECTED_TEAMS: usize = 18;

/// The site splits the standings over two `table.table` elements (1-9 and 10-18).
const TABLE_SELECTORS: [&str; 2] = ["table.table tbody tr", "table tbody tr"];

pub async fn scrape_eredivisie(client: &reqwest::Client) -> Option<StandingsData> {
    let html = fetch_html(client, URL).await?;
    finish("Eredivisie", parse(&html), EXPECTED_TEAMS)
}

/// Columns: # | Club | GS | W | G | V | DV-DT | DS | P
fn parse(html: &str) -> Vec<Team> {
    let document = Html::parse_document(html);
    let rows = find_rows(&document, &TABLE_SELECTORS, 0, EXPECTED_TEAMS);

    let mut teams = Vec::new();
    for (i, row) in rows.iter().take(EXPECTED_TEAMS).enumerate() {
        let Some(cells) = row_cells(row) else {
            continue;
        };

        let position = cell_text(&cells, 0)
            .trim()
            .parse::<i32>()
            .unwrap_or((i + 1) as i32);

        let name = clean_team_name(&cell_text(&cells, 1));
        if name.len() < 2 {
            continue;
        }

        // "W | G | V" in cell 3: won, drawn (gelijk), lost (verloren)
        let [wins, draws, losses] = split_ints(&cell_text(&cells, 3), '|');
        let [goals_for, goals_against] = split_ints(&cell_text(&cells, 4), '-');

        teams.push(Team {
            position,
            name,
            games: safe_int(&cell_text(&cells, 2)),
            wins,
            draws,
            losses,
            goals_for,
            goals_against,
            goal_difference: safe_int(&cell_text(&cells, 5)),
            points: safe_int(&cell_text(&cells, 6)),
            logo: cell_img_attr(&cells, 1, "src").map(str::to_owned),
        });
    }
    teams
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both standings tables trimmed from the live page.
    const PAGE: &str = include_str!("../../tests/fixtures/eredivisie.html");

    #[test]
    fn parses_real_page() {
        let teams = parse(PAGE);

        assert_eq!(teams.len(), EXPECTED_TEAMS);
        let t = &teams[1];
        assert_eq!((t.position, t.name.as_str()), (2, "Feyenoord"));
        assert_eq!((t.games, t.wins, t.draws, t.losses), (12, 7, 4, 1));
        assert_eq!(
            (t.goals_for, t.goals_against, t.goal_difference),
            (36, 17, 19)
        );
        assert_eq!(t.points, 25);
        assert_eq!(teams[17].goal_difference, -29);
        assert_eq!(
            teams[0].logo.as_deref(),
            Some(
                "https://eredivisie.b-cdn.net/production/clubs/psv/PSV_Logo.png?width=40&height=40&quality=85"
            )
        );
        assert!(teams.iter().all(|t| t.logo.is_some()));
        // Catches a swapped W/D/L column: every row must add up.
        for t in &teams {
            assert_eq!(t.wins * 3 + t.draws, t.points, "{}", t.name);
            assert_eq!(t.wins + t.draws + t.losses, t.games, "{}", t.name);
        }
    }
}
