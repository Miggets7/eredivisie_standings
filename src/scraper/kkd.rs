use chrono::Utc;
use scraper::{ElementRef, Html, Selector};
use tracing::{error, info};

use crate::models::{StandingsData, Team};
use crate::scraper::{clean_team_name, fetch_html, safe_int};

fn extract_team_name(
    row: &ElementRef,
    cells: &[ElementRef],
    team_name_selector: &Option<Selector>,
    img_selector: &Option<Selector>,
) -> Option<String> {
    // Try the hidden lg:table-cell with team name link
    if let Some(sel) = team_name_selector {
        if let Some(el) = row.select(sel).next() {
            let name = clean_team_name(&el.text().collect::<String>());
            if name.len() >= 2 {
                return Some(name);
            }
        }
    }

    // Try image alt text in cell 2
    if let Some(sel) = img_selector {
        if let Some(img) = cells[2].select(sel).next() {
            if let Some(alt) = img.attr("alt") {
                let name = clean_team_name(alt);
                if name.len() >= 2 {
                    return Some(name);
                }
            }
        }
    }

    // Last resort: cell 3
    if cells.len() > 3 {
        let name = clean_team_name(&cells[3].text().collect::<String>());
        if name.len() >= 2 {
            return Some(name);
        }
    }

    None
}

const URL: &str = "https://keukenkampioendivisie.nl/klassement";
const EXPECTED_TEAMS: usize = 20;

pub async fn scrape_kkd(client: &reqwest::Client) -> Option<StandingsData> {
    let html = match fetch_html(client, URL).await {
        Ok(html) => html,
        Err(e) => {
            error!("Error fetching {URL}: {e}");
            return None;
        }
    };

    let document = Html::parse_document(&html);

    // Try multiple table selectors for KKD website
    let table_selectors = [
        "table.table-medium tbody tr",
        "table.table tbody tr",
        ".standings-table tbody tr",
        "table tbody tr",
        ".table tbody tr",
    ];

    let mut rows: Vec<_> = Vec::new();
    for selector_str in &table_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            let found: Vec<_> = document.select(&selector).collect();
            if found.len() >= EXPECTED_TEAMS {
                info!("Found KKD table using selector: {selector_str}");
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
                    if cells.len() <= 1 {
                        return false;
                    }
                    let pos_text = cells[1].text().collect::<String>().trim().to_string();
                    pos_text
                        .parse::<u32>()
                        .is_ok_and(|n| n >= 1 && n <= EXPECTED_TEAMS as u32)
                })
                .collect();
        }
    }

    info!("Found {} table rows for KKD", rows.len());

    let td_selector = Selector::parse("td, th").unwrap();
    let team_name_selector = Selector::parse("td.font-bold.hidden a").ok();
    let img_selector = Selector::parse("img").ok();
    let mut teams = Vec::new();

    for (i, row) in rows.iter().take(EXPECTED_TEAMS).enumerate() {
        let cells: Vec<_> = row.select(&td_selector).collect();
        if cells.len() < 7 {
            continue;
        }

        // Position from cell 1
        let position = {
            let pos = safe_int(&cells[1].text().collect::<String>());
            if pos >= 1 && pos <= EXPECTED_TEAMS as i32 {
                pos
            } else {
                (i + 1) as i32
            }
        };

        // Team name: try multiple strategies
        let team_name = extract_team_name(row, &cells, &team_name_selector, &img_selector);
        let Some(team_name) = team_name else {
            tracing::warn!("Could not extract team name from row {}", i + 1);
            continue;
        };

        // Games played from cell 4
        let games = if cells.len() > 4 {
            safe_int(&cells[4].text().collect::<String>())
        } else {
            0
        };

        // W/D/L from cell 5
        let wdl_text = if cells.len() > 5 {
            cells[5].text().collect::<String>()
        } else {
            String::new()
        };
        let wdl_parts: Vec<&str> = wdl_text.split('/').collect();
        let wins = wdl_parts.first().map_or(0, |v| safe_int(v.trim()));
        let draws = wdl_parts.get(1).map_or(0, |v| safe_int(v.trim()));
        let losses = wdl_parts.get(2).map_or(0, |v| safe_int(v.trim()));

        // Points from cell 6
        let points = if cells.len() > 6 {
            safe_int(&cells[6].text().collect::<String>())
        } else {
            0
        };

        // Goals for/against from cell 7
        let goals_text = if cells.len() > 7 {
            cells[7].text().collect::<String>()
        } else {
            String::new()
        };
        let goals_parts: Vec<&str> = goals_text.split('/').collect();
        let goals_for = goals_parts.first().map_or(0, |v| safe_int(v.trim()));
        let goals_against = goals_parts.get(1).map_or(0, |v| safe_int(v.trim()));

        // Goal difference from cell 8
        let goal_difference = if cells.len() > 8 {
            safe_int(&cells[8].text().collect::<String>())
        } else {
            0
        };

        if team_name.to_lowercase().contains("vitesse") && points < 0 {
            info!("Found Vitesse with {} points", points);
        }

        teams.push(Team {
            position,
            name: team_name,
            games,
            wins,
            draws,
            losses,
            goals_for,
            goals_against,
            goal_difference,
            points,
        });
    }

    if teams.len() >= 10 {
        teams.sort_by_key(|t| t.position);
        info!("Successfully scraped {} KKD teams", teams.len());
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
