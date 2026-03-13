use chrono::Utc;
use scraper::{Html, Selector};
use tracing::{error, info};

use crate::models::{StandingsData, Team};
use crate::scraper::{clean_team_name, fetch_html, safe_int};

const URL: &str = "https://eredivisie.nl/competitie/stand/";
const EXPECTED_TEAMS: usize = 18;

pub async fn scrape_eredivisie(client: &reqwest::Client) -> Option<StandingsData> {
    let html = match fetch_html(client, URL).await {
        Ok(html) => html,
        Err(e) => {
            error!("Error fetching {URL}: {e}");
            return None;
        }
    };

    let document = Html::parse_document(&html);

    // Try multiple table selectors
    let table_selectors = [
        "table.standings tbody tr",
        ".standings-table tbody tr",
        "table tbody tr",
        ".table tbody tr",
    ];

    let mut rows: Vec<_> = Vec::new();
    for selector_str in &table_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            let found: Vec<_> = document.select(&selector).collect();
            if found.len() >= EXPECTED_TEAMS {
                rows = found;
                break;
            }
        }
    }

    // Fallback: look for any table rows with numeric data
    if rows.is_empty() {
        if let Ok(tr_selector) = Selector::parse("tr") {
            let td_selector = Selector::parse("td, th").unwrap();
            rows = document
                .select(&tr_selector)
                .filter(|row| {
                    let cells: Vec<_> = row.select(&td_selector).collect();
                    if cells.len() < 7 {
                        return false;
                    }
                    let first_text = cells[0].text().collect::<String>().trim().to_string();
                    first_text
                        .parse::<u32>()
                        .is_ok_and(|n| n >= 1 && n <= EXPECTED_TEAMS as u32)
                })
                .collect();
        }
    }

    info!("Found {} table rows", rows.len());

    let td_selector = Selector::parse("td, th").unwrap();
    let mut teams = Vec::new();

    for (i, row) in rows.iter().take(EXPECTED_TEAMS).enumerate() {
        let cells: Vec<_> = row.select(&td_selector).collect();
        if cells.len() < 7 {
            continue;
        }

        let pos_text = cells[0].text().collect::<String>().trim().to_string();
        let position = pos_text.parse::<i32>().unwrap_or((i + 1) as i32);

        let team_name = clean_team_name(&cells[1].text().collect::<String>());
        if team_name.len() < 2 {
            continue;
        }

        let games = safe_int(&cells[2].text().collect::<String>());

        // W|L|D from cell 3
        let results_text = cells[3].text().collect::<String>();
        let results_parts: Vec<&str> = results_text.split('|').collect();
        let wins = results_parts.first().map_or(0, |v| safe_int(v));
        let losses = results_parts.get(1).map_or(0, |v| safe_int(v));
        let draws = results_parts.get(2).map_or(0, |v| safe_int(v));

        // Goals for-against from cell 4
        let goals_text = cells[4].text().collect::<String>();
        let goals_parts: Vec<&str> = goals_text.split('-').collect();
        let goals_for = goals_parts.first().map_or(0, |v| safe_int(v));
        let goals_against = goals_parts.get(1).map_or(0, |v| safe_int(v));

        let goal_difference = if cells.len() > 5 {
            safe_int(&cells[5].text().collect::<String>())
        } else {
            0
        };
        let points = if cells.len() > 6 {
            safe_int(&cells[6].text().collect::<String>())
        } else {
            0
        };

        teams.push(Team {
            position,
            name: team_name,
            games,
            wins,
            losses,
            draws,
            goals_for,
            goals_against,
            goal_difference,
            points,
        });
    }

    if teams.len() >= 10 {
        teams.sort_by_key(|t| t.position);
        info!("Successfully scraped {} teams", teams.len());
        Some(StandingsData {
            teams,
            last_updated: Utc::now().to_rfc3339(),
        })
    } else {
        error!(
            "Only found {} teams, expected {}",
            teams.len(),
            EXPECTED_TEAMS
        );
        None
    }
}
