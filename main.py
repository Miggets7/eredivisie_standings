#!/usr/bin/env python3
"""
Eredivisie TRMNL Service
A FastAPI service that scrapes Eredivisie standings and serves them for TRMNL devices
"""

import aiohttp
import logging
import os
from datetime import datetime
from typing import List, Optional
from dataclasses import dataclass, asdict
import re

from fastapi import FastAPI, HTTPException, Query, Depends, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.security.api_key import APIKeyQuery
from bs4 import BeautifulSoup
import uvicorn
from apscheduler.schedulers.asyncio import AsyncIOScheduler

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Get API key from environment variable
API_KEY = os.environ.get("API_KEY", "")
if not API_KEY:
    logger.warning("API_KEY environment variable not set. API will be accessible without authentication.")

# API key security scheme
api_key_query = APIKeyQuery(name="api_key", auto_error=False)

@dataclass
class Team:
    position: int
    name: str
    games: int
    wins: int
    losses: int
    draws: int
    goals_for: int
    goals_against: int
    goal_difference: int
    points: int

@dataclass
class StandingsData:
    teams: List[Team]
    last_updated: str

class EredivisieScraper:
    def __init__(self):
        self.url = "https://eredivisie.nl/competitie/stand/"
        self.data: Optional[StandingsData] = None
        self.session: Optional[aiohttp.ClientSession] = None
        
    async def __aenter__(self):
        self.session = aiohttp.ClientSession(
            headers={'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'},
            timeout=aiohttp.ClientTimeout(total=30)
        )
        return self
        
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        if self.session:
            await self.session.close()
    
    def _clean_team_name(self, name: str) -> str:
        """Clean team name for display"""
        # Remove extra whitespace
        name = re.sub(r'\s+', ' ', name.strip())
        return name.replace("*", "").strip()
    
    def _safe_int(self, value: str) -> int:
        """Safely convert string to int"""
        try:
            clean_value = re.sub(r'[^\d-]', '', str(value))
            return int(clean_value) if clean_value else 0
        except (ValueError, TypeError):
            return 0
    
    async def scrape_standings(self) -> Optional[StandingsData]:
        """Scrape current standings from eredivisie.nl"""
        try:
            async with self.session.get(self.url) as response:
                if response.status != 200:
                    logger.error(f"HTTP {response.status} when fetching {self.url}")
                    return None
                
                html = await response.text()
                soup = BeautifulSoup(html, 'html.parser')
                
                teams = []
                
                # Try multiple table selectors
                table_selectors = [
                    'table.standings tbody tr',
                    '.standings-table tbody tr',
                    'table tbody tr',
                    '.table tbody tr'
                ]
                
                rows = []
                for selector in table_selectors:
                    rows = soup.select(selector)
                    if len(rows) >= 18:  # Eredivisie has 18 teams
                        break
                
                if not rows:
                    # Fallback: look for any table rows with numeric data
                    all_rows = soup.find_all('tr')
                    rows = []
                    for row in all_rows:
                        cells = row.find_all(['td', 'th'])
                        if len(cells) >= 7:
                            # Check if first cell looks like a position
                            first_cell = cells[0].get_text().strip()
                            if first_cell.isdigit() and int(first_cell) <= 18:
                                rows.append(row)
                
                logger.info(f"Found {len(rows)} table rows")
                
                for i, row in enumerate(rows[:18]):  # Limit to 18 teams
                    try:
                        cells = row.find_all(['td', 'th'])
                        if len(cells) < 7:
                            continue
                            
                        # Extract position (either from first cell or infer from order)
                        pos_text = cells[0].get_text().strip()
                        position = int(pos_text) if pos_text.isdigit() else i + 1
                        
                        # Extract team name
                        team_name = self._clean_team_name(cells[1].get_text())
                        if len(team_name) < 2:
                            continue
                        
                        # Extract stats (adjust indices based on table structure)
                        games = self._safe_int(cells[2].get_text()) if len(cells) > 2 else 0
                        game_results = cells[3].get_text().split('|')
                        wins = self._safe_int(game_results[0]) if len(game_results) > 0 else 0
                        losses = self._safe_int(game_results[1]) if len(game_results) > 1 else 0
                        draws = self._safe_int(game_results[2]) if len(game_results) > 2 else 0

                        goals_for_against = cells[4].get_text().split('-')
                        goals_for = self._safe_int(goals_for_against[0]) if len(goals_for_against) > 0 else 0
                        goals_against = self._safe_int(goals_for_against[1]) if len(goals_for_against) > 1 else 0
                        
                        # Goal difference and points
                        goal_diff = self._safe_int(cells[5].get_text()) if len(cells) > 5 else 0
                        points = self._safe_int(cells[6].get_text()) if len(cells) > 6 else 0
                        
                        team = Team(
                            position=position,
                            name=team_name,
                            games=games,
                            wins=wins,
                            losses=losses,
                            draws=draws,
                            goals_for=goals_for,
                            goals_against=goals_against,
                            goal_difference=goal_diff,
                            points=points
                        )
                        teams.append(team)
                        
                    except Exception as e:
                        logger.warning(f"Error parsing row {i}: {e}")
                        continue
                
                if len(teams) >= 10:  # We got reasonable data
                    # Sort by position to ensure correct order
                    teams.sort(key=lambda x: x.position)
                    
                    self.data = StandingsData(
                        teams=teams,
                        last_updated=datetime.now().isoformat()
                    )
                    logger.info(f"Successfully scraped {len(teams)} teams")
                    return self.data
                else:
                    logger.error(f"Only found {len(teams)} teams, expected 18")
                    return None
                    
        except Exception as e:
            logger.error(f"Error scraping standings: {e}")
            return None
        
class KeukenKampioenDivisieScraper(EredivisieScraper):
    def __init__(self):
        super().__init__()
        self.url = "https://keukenkampioendivisie.nl/klassement"
    
    async def scrape_standings(self) -> Optional[StandingsData]:
        """Scrape current standings from keukenkampioendivisie.nl"""
        try:
            async with self.session.get(self.url) as response:
                if response.status != 200:
                    logger.error(f"HTTP {response.status} when fetching {self.url}")
                    return None
                
                html = await response.text()
                soup = BeautifulSoup(html, 'html.parser')
                
                teams = []
                
                # Try multiple table selectors for KKD website
                table_selectors = [
                    'table.table-medium tbody tr',
                    'table.table tbody tr',
                    '.standings-table tbody tr',
                    'table tbody tr',
                    '.table tbody tr'
                ]
                
                rows = []
                for selector in table_selectors:
                    rows = soup.select(selector)
                    if len(rows) >= 20:  # KKD has 20 teams
                        logger.info(f"Found KKD table using selector: {selector}")
                        break
                
                if not rows:
                    # Fallback: look for any table rows with numeric data
                    all_rows = soup.find_all('tr')
                    rows = []
                    for row in all_rows:
                        cells = row.find_all(['td', 'th'])
                        if len(cells) >= 7:
                            # Check if second cell looks like a position
                            if len(cells) > 1:
                                pos_cell = cells[1].get_text().strip()
                                if pos_cell.isdigit() and 1 <= int(pos_cell) <= 20:
                                    rows.append(row)
                
                logger.info(f"Found {len(rows)} table rows for KKD")
                
                for i, row in enumerate(rows[:20]):  # Limit to 20 teams
                    try:
                        cells = row.find_all(['td', 'th'])
                        if len(cells) < 7:  # Need at least position, team, and stats
                            continue
                            
                        # Get position from the second cell (index 1)
                        position = self._safe_int(cells[1].get_text().strip())
                        if position <= 0 or position > 20:
                            position = i + 1  # Fallback to row index
                        
                        # Get team name - check both possible locations
                        team_name = None
                        
                        # First try the hidden lg:table-cell with the team name
                        team_cell = row.select_one('td.font-bold.hidden.lg\:table-cell')
                        if team_cell and team_cell.find('a'):
                            team_name = self._clean_team_name(team_cell.find('a').get_text())
                        
                        # If that didn't work, try the image alt text
                        if not team_name and cells[2].find('img'):
                            img = cells[2].find('img')
                            if img.get('alt'):
                                team_name = self._clean_team_name(img.get('alt'))
                        
                        # Last resort, try the fourth cell
                        if not team_name and len(cells) > 3:
                            team_name = self._clean_team_name(cells[3].get_text())
                        
                        if not team_name or len(team_name) < 2:
                            logger.warning(f"Could not extract team name from row {i+1}")
                            continue
                        
                        # Get games played
                        games = self._safe_int(cells[4].get_text() if len(cells) > 4 else "0")
                        
                        # Parse W/G/V (wins, draws, losses)
                        wgv_text = cells[5].get_text().strip() if len(cells) > 5 else "0/0/0"
                        wgv_parts = wgv_text.split('/')
                        wins = self._safe_int(wgv_parts[0].strip()) if len(wgv_parts) > 0 else 0
                        draws = self._safe_int(wgv_parts[1].strip()) if len(wgv_parts) > 1 else 0
                        losses = self._safe_int(wgv_parts[2].strip()) if len(wgv_parts) > 2 else 0
                        
                        # Get points
                        points = self._safe_int(cells[6].get_text() if len(cells) > 6 else "0")
                        
                        # Parse DV/DT (goals for/against)
                        dvdt_text = cells[7].get_text().strip() if len(cells) > 7 else "0/0"
                        dvdt_parts = dvdt_text.split('/')
                        goals_for = self._safe_int(dvdt_parts[0].strip()) if len(dvdt_parts) > 0 else 0
                        goals_against = self._safe_int(dvdt_parts[1].strip()) if len(dvdt_parts) > 1 else 0
                        
                        # Get goal difference
                        goal_diff = self._safe_int(cells[8].get_text() if len(cells) > 8 else "0")
                        
                        # Handle special case for Vitesse with negative points
                        if "vitesse" in team_name.lower() and points < 0:
                            logger.info(f"Found Vitesse with {points} points")
                        
                        teams.append(Team(
                            position=position,
                            name=team_name,
                            games=games,
                            wins=wins,
                            draws=draws,
                            losses=losses,
                            goals_for=goals_for,
                            goals_against=goals_against,
                            goal_difference=goal_diff,
                            points=points
                        ))
                        
                    except Exception as e:
                        logger.warning(f"Error parsing KKD row {i}: {e}")
                        continue
                
                if len(teams) >= 10:  # We got reasonable data
                    # Sort by position to ensure correct order
                    teams.sort(key=lambda x: x.position)
                    
                    self.data = StandingsData(
                        teams=teams,
                        last_updated=datetime.now().isoformat()
                    )
                    logger.info(f"Successfully scraped {len(teams)} KKD teams")
                    return self.data
                else:
                    logger.error(f"Only found {len(teams)} teams, expected 20")
                    return None
                    
        except Exception as e:
            logger.error(f"Error scraping KKD standings: {e}")
            return None
# Global scraper instances
scraper = EredivisieScraper()
kkd_scraper = KeukenKampioenDivisieScraper()

# FastAPI app
app = FastAPI(
    title="Eredivisie & KKD TRMNL Service",
    description="Eredivisie and Keuken Kampioen Divisie standings data for TRMNL devices",
    version="1.1.0"
)

# API key dependency
async def verify_api_key(request: Request, api_key: str = Depends(api_key_query)):
    """Verify that the request has a valid API key"""
    # If no API_KEY is set in environment, skip verification (development mode)
    if not API_KEY:
        return True
    
    # Check if API key matches
    if api_key != API_KEY:
        raise HTTPException(
            status_code=403,
            detail="Invalid API key"
        )
    return True

# Enable CORS for TRMNL
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Scheduler for periodic updates
scheduler = AsyncIOScheduler()

async def update_standings():
    """Periodic task to update standings"""
    # Update Eredivisie standings
    logger.info("Updating Eredivisie standings...")
    async with EredivisieScraper() as scraper_instance:
        await scraper_instance.scrape_standings()
        if scraper_instance.data:
            scraper.data = scraper_instance.data
            logger.info("Eredivisie standings updated successfully")
        else:
            logger.error("Failed to update Eredivisie standings")
    
    # Update KKD standings
    logger.info("Updating Keuken Kampioen Divisie standings...")
    async with KeukenKampioenDivisieScraper() as kkd_scraper_instance:
        await kkd_scraper_instance.scrape_standings()
        if kkd_scraper_instance.data:
            kkd_scraper.data = kkd_scraper_instance.data
            logger.info("KKD standings updated successfully")
        else:
            logger.error("Failed to update KKD standings")

@app.on_event("startup")
async def startup_event():
    """Initialize the service"""
    logger.info("Starting Eredivisie TRMNL Service...")
    
    # Initial data fetch
    await update_standings()
    
    # Schedule updates every 60 minutes
    scheduler.add_job(
        update_standings,
        'interval',
        minutes=60,
        id='update_standings'
    )
    scheduler.start()
    logger.info("Service started successfully")

@app.on_event("shutdown")
async def shutdown_event():
    """Cleanup on shutdown"""
    scheduler.shutdown()
    logger.info("Service stopped")

@app.get("/")
async def root(authorized: bool = Depends(verify_api_key)):
    """Health check endpoint"""
    return {
        "service": "Eredivisie & KKD TRMNL Service",
        "status": "running",
        "eredivisie_last_updated": scraper.data.last_updated if scraper.data else None,
        "eredivisie_teams_count": len(scraper.data.teams) if scraper.data else 0,
        "kkd_last_updated": kkd_scraper.data.last_updated if kkd_scraper.data else None,
        "kkd_teams_count": len(kkd_scraper.data.teams) if kkd_scraper.data else 0
    }

@app.get("/standings")
async def get_standings(
    authorized: bool = Depends(verify_api_key),
    top: Optional[int] = Query(None, description="Number of top teams to return")
):
    """Get current Eredivisie standings"""
    if not scraper.data:
        raise HTTPException(status_code=503, detail="Eredivisie standings data not available")
    
    teams = scraper.data.teams
    if top:
        teams = teams[:top]
    
    return {
        "standings": [asdict(team) for team in teams],
        "last_updated": scraper.data.last_updated
    }

@app.get("/kkd-standings")
async def get_kkd_standings(
    authorized: bool = Depends(verify_api_key),
    top: Optional[int] = Query(None, description="Number of top teams to return")
):
    """Get current Keuken Kampioen Divisie standings"""
    if not kkd_scraper.data:
        raise HTTPException(status_code=503, detail="Keuken Kampioen Divisie standings data not available")
    
    teams = kkd_scraper.data.teams
    if top:
        teams = teams[:top]
    
    return {
        "standings": [asdict(team) for team in teams],
        "last_updated": kkd_scraper.data.last_updated
    }

if __name__ == "__main__":
    uvicorn.run(
        "main:app",
        host="0.0.0.0",
        port=8000,
        reload=True,
        log_level="info"
    )
