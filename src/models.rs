use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Team {
    pub position: i32,
    pub name: String,
    pub games: i32,
    pub wins: i32,
    pub losses: i32,
    pub draws: i32,
    pub goals_for: i32,
    pub goals_against: i32,
    pub goal_difference: i32,
    pub points: i32,
    /// Club crest image URL from the source site.
    pub logo: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StandingsData {
    pub teams: Vec<Team>,
    pub last_updated: String,
    /// Season the standings belong to, e.g. "2026/2027".
    pub season: String,
}
