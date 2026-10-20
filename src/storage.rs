//! Local catalog, immutable match rosters, and append-only match commands.
use crate::{
    Game, Play, Player, Side, Stat, Team,
    clock::ClockState,
    profile::{self, PlayerDetails, Position},
    sport::{Sport, State},
};
use chrono::NaiveDate;
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::{
    fmt,
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;
#[path = "lineup_storage.rs"]
mod lineups;
#[path = "sport_storage.rs"]
mod sports;
pub use sports::{SportEvent, SportMatchDraft};
const APPLICATION_ID: i64 = 0x48584f53;

#[derive(Debug)]
pub enum Error {
    Validation(String),
    Conflict,
    UnsupportedSchema,
    Sql(rusqlite::Error),
    Io(std::io::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => f.write_str(message),
            Self::Conflict => f.write_str("This match changed in another window. Reopen it from Matches before recording another play."),
            Self::UnsupportedSchema => f.write_str("This database belongs to another app or a newer version of HustlX OSS. It has not been reset. Use a compatible app version."),
            Self::Sql(_) => f.write_str("Local storage could not complete the operation. Check folder access and free space, then try again. Another window may have the database busy."),
            Self::Io(error) => write!(f, "Cannot access the data folder: {error}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: i64,
    pub name: String,
    pub archived: bool,
}
impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (#{})", self.name, self.id)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterEntry {
    pub player_id: i64,
    pub name: String,
    pub jersey: String,
    pub positions: Vec<Position>,
}
#[derive(Debug, Clone)]
pub struct MatchDraft {
    pub name: String,
    pub date: String,
    pub venue: String,
    pub home_team_id: i64,
    pub away_team_id: i64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchSummary {
    pub sport: Sport,
    pub id: i64,
    pub name: String,
    pub date: String,
    pub venue: String,
    pub home_name: String,
    pub away_name: String,
    pub revision: i64,
}
#[derive(Debug, Clone)]
pub struct SavedMatch {
    pub sport_state: Option<State>,
    pub sport_history: Vec<SportEvent>,
    pub summary: MatchSummary,
    pub game: Game,
    pub clock: ClockState,
    pub lineups: Vec<crate::lineup::LineupChange>,
}
#[derive(Debug)]
pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn default_path() -> Result<PathBuf> {
        let dir = match std::env::var_os("HUSTLX_OSS_DATA_DIR") {
            Some(dir) => {
                let path = PathBuf::from(dir);
                if !path.is_absolute() {
                    return invalid("HUSTLX_OSS_DATA_DIR must be an absolute directory path.");
                }
                path
            }
            None => ProjectDirs::from("com", "HustlX", "hustlx-oss")
                .ok_or_else(|| {
                    Error::Validation("Cannot find your application data folder.".into())
                })?
                .data_local_dir()
                .to_path_buf(),
        };
        Ok(dir.join("hustlx.sqlite3"))
    }
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_millis(250))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let app: i64 = tx.pragma_query_value(None, "application_id", |r| r.get(0))?;
        match version {
            0 => {
                let tables: i64 = tx.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",[],|r|r.get(0))?;
                if tables != 0 || (app != 0 && app != APPLICATION_ID) {
                    return Err(Error::UnsupportedSchema);
                }
                tx.execute_batch(include_str!("schema.sql"))?;
                tx.pragma_update(None, "application_id", APPLICATION_ID)?;
                tx.pragma_update(None, "user_version", 1)?;
            }
            1..=5 if app == APPLICATION_ID => {}
            _ => return Err(Error::UnsupportedSchema),
        }
        if version < 2 {
            tx.execute_batch(include_str!("migration_v2.sql"))?;
            tx.pragma_update(None, "user_version", 2)?;
        }
        if version < 3 {
            tx.execute_batch(include_str!("migration_v3.sql"))?;
            tx.pragma_update(None, "user_version", 3)?;
        }
        if version < 4 {
            tx.execute_batch(include_str!("migration_v4.sql"))?;
            tx.pragma_update(None, "user_version", 4)?;
        }
        if version < 5 {
            tx.execute_batch(include_str!("migration_v5.sql"))?;
            tx.pragma_update(None, "user_version", 5)?;
        }
        tx.commit()?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
        Ok(Self { connection })
    }
    pub fn create_team(&self, name: &str) -> Result<i64> {
        self.create_entry("teams", name)
    }
    pub fn create_player(&self, name: &str) -> Result<i64> {
        self.create_entry("players", name)
    }
    pub fn player_details(&self, id: i64) -> Result<PlayerDetails> {
        Ok(self.connection.query_row(
            "SELECT jersey,positions FROM players WHERE id=?1",
            [id],
            |r| {
                Ok(PlayerDetails {
                    jersey: r.get(0)?,
                    positions: profile::decode(&r.get::<_, String>(1)?),
                })
            },
        )?)
    }
    pub fn save_player(
        &mut self,
        id: Option<i64>,
        name: &str,
        jersey: &str,
        positions: &[Position],
    ) -> Result<i64> {
        let name = required(name, "Name", 200)?;
        let jersey = bounded(jersey, "Jersey", 12)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let positions = profile::encode(positions);
        let id = if let Some(id) = id {
            changed(tx.execute(
                "UPDATE players SET name=?1,jersey=?2,positions=?3 WHERE id=?4",
                params![name, jersey, positions, id],
            )?)?;
            id
        } else {
            tx.execute(
                "INSERT INTO players(name,jersey,positions) VALUES(?1,?2,?3)",
                params![name, jersey, positions],
            )?;
            tx.last_insert_rowid()
        };
        tx.commit()?;
        Ok(id)
    }
    pub fn save_player_details(
        &mut self,
        id: i64,
        jersey: &str,
        positions: &[Position],
    ) -> Result<()> {
        let jersey = bounded(jersey, "Jersey", 12)?;
        changed(self.connection.execute(
            "UPDATE players SET jersey=?1,positions=?2 WHERE id=?3",
            params![jersey, profile::encode(positions), id],
        )?)
    }
    pub fn rename_team(&self, id: i64, name: &str) -> Result<()> {
        self.rename("teams", id, name)
    }
    pub fn rename_player(&self, id: i64, name: &str) -> Result<()> {
        self.rename("players", id, name)
    }
    pub fn archive_team(&self, id: i64, archived: bool) -> Result<()> {
        self.archive("teams", id, archived)
    }
    pub fn archive_player(&self, id: i64, archived: bool) -> Result<()> {
        self.archive("players", id, archived)
    }
    pub fn teams(&self, include_archived: bool) -> Result<Vec<Entry>> {
        self.entries("teams", include_archived)
    }
    pub fn players(&self, include_archived: bool) -> Result<Vec<Entry>> {
        self.entries("players", include_archived)
    }
    // Table identifiers are private static literals, never form values.
    fn create_entry(&self, table: &str, name: &str) -> Result<i64> {
        let name = required(name, "Name", 200)?;
        self.connection
            .execute(&format!("INSERT INTO {table}(name) VALUES (?1)"), [name])?;
        Ok(self.connection.last_insert_rowid())
    }
    fn rename(&self, table: &str, id: i64, name: &str) -> Result<()> {
        let name = required(name, "Name", 200)?;
        changed(self.connection.execute(
            &format!("UPDATE {table} SET name=?1 WHERE id=?2"),
            params![name, id],
        )?)
    }
    fn archive(&self, table: &str, id: i64, archived: bool) -> Result<()> {
        changed(self.connection.execute(
            &format!("UPDATE {table} SET archived=?1 WHERE id=?2"),
            params![archived, id],
        )?)
    }
    fn entries(&self, table: &str, include_archived: bool) -> Result<Vec<Entry>> {
        let mut stmt=self.connection.prepare(&format!("SELECT id,name,archived FROM {table} WHERE archived=0 OR ?1 ORDER BY archived,name COLLATE NOCASE,id"))?;
        Ok(stmt
            .query_map([include_archived], |r| {
                Ok(Entry {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    archived: r.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?)
    }
    pub fn roster(&self, team_id: i64) -> Result<Vec<RosterEntry>> {
        roster(&self.connection, team_id)
    }
    pub fn add_to_team(&self, team: i64, player: i64, jersey: &str) -> Result<()> {
        active_name(&self.connection, "teams", team)?;
        active_name(&self.connection, "players", player)?;
        let jersey = bounded(jersey, "Jersey", 12)?;
        self.connection.execute("INSERT INTO team_players(team_id,player_id,jersey) VALUES (?1,?2,?3) ON CONFLICT(team_id,player_id) DO UPDATE SET jersey=excluded.jersey",params![team,player,jersey])?;
        Ok(())
    }
    pub fn remove_from_team(&self, team: i64, player: i64) -> Result<()> {
        changed(self.connection.execute(
            "DELETE FROM team_players WHERE team_id=?1 AND player_id=?2",
            params![team, player],
        )?)
    }
    pub fn create_match(&mut self, draft: &MatchDraft) -> Result<SavedMatch> {
        let name = required(&draft.name, "Match name", 200)?;
        let date = draft.date.trim();
        if date.len() != 10 || NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
            return invalid("Enter a real match date as YYYY-MM-DD.");
        }
        let venue = bounded(&draft.venue, "Venue", 300)?;
        if draft.home_team_id == draft.away_team_id {
            return invalid("Choose two different teams.");
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let home = active_name(&tx, "teams", draft.home_team_id)?;
        let away = active_name(&tx, "teams", draft.away_team_id)?;
        tx.execute("INSERT INTO matches(name,date,venue,home_team_id,away_team_id,home_name,away_name) VALUES (?1,?2,?3,?4,?5,?6,?7)",params![name,date,venue,draft.home_team_id,draft.away_team_id,home,away])?;
        let id = tx.last_insert_rowid();
        tx.execute("INSERT INTO match_clocks(match_id) VALUES(?1)", [id])?;
        for (side, team_id) in [("home", draft.home_team_id), ("away", draft.away_team_id)] {
            for (order, p) in roster(&tx, team_id)?.into_iter().enumerate() {
                tx.execute("INSERT INTO match_players(match_id,side,player_id,name,jersey,on_court,sort_order,positions) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",params![id,side,p.player_id,p.name,p.jersey,false,order as i64,profile::encode(&p.positions)])?;
            }
        }
        let saved = load(&tx, id)?;
        tx.commit()?;
        Ok(saved)
    }
    pub fn matches(&self) -> Result<Vec<MatchSummary>> {
        let mut stmt=self.connection.prepare("SELECT id,name,date,venue,home_name,away_name,revision,sport FROM matches ORDER BY date DESC,id DESC")?;
        Ok(stmt
            .query_map([], summary)?
            .collect::<rusqlite::Result<_>>()?)
    }
    pub fn load_match(&self, id: i64) -> Result<SavedMatch> {
        // A read transaction keeps metadata, rosters and plays at the same revision.
        let tx = self.connection.unchecked_transaction()?;
        let saved = load(&tx, id)?;
        tx.commit()?;
        Ok(saved)
    }
    pub fn record(
        &mut self,
        id: i64,
        revision: i64,
        command: Uuid,
        side: Side,
        player: i64,
        stat: Stat,
    ) -> Result<SavedMatch> {
        self.record_with_clock(id, revision, command, side, player, stat, None)
    }
    #[allow(clippy::too_many_arguments)]
    /// Record with the pre-action live clock. None uses the saved baseline;
    /// callers operating a running clock must supply its current projection.
    pub fn record_with_clock(
        &mut self,
        id: i64,
        revision: i64,
        command: Uuid,
        side: Side,
        player: i64,
        stat: Stat,
        clock: Option<ClockState>,
    ) -> Result<SavedMatch> {
        if let Some(clock) = clock {
            clock.validate()?;
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        event_command_available(&tx, command)?;
        let side = side_key(side);
        let tag = stat_key(stat);
        let existing = tx
            .query_row(
                "SELECT match_id,kind,side,player_id,stat,game_ms,shot_ms,period,running,shot_running FROM match_events WHERE command_id=?1",
                [command.to_string()],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, Option<i64>>(3)?,
                        r.get::<_, Option<String>>(4)?,
                        event_clock_row(r, 5)?,
                    ))
                },
            )
            .optional()?;
        if let Some(e) = existing {
            let expected = (
                id,
                "record".into(),
                Some(side.into()),
                Some(player),
                Some(tag.into()),
                e.5,
            );
            if e != expected || clock.is_some_and(|stamp| e.5 != Some(stamp)) {
                return invalid(
                    "This save identifier was already used for a different action. Reopen the match.",
                );
            }
            return load(&tx, id);
        }
        let current = load(&tx, id)?;
        basketball_only(&current)?;
        if current.summary.revision != revision {
            return Err(Error::Conflict);
        }
        if current.game.player(parse_side(side)?, player).is_none() {
            return invalid("Select a player from this match's roster.");
        }
        let stamp = clock.unwrap_or(current.clock);
        tx.execute("INSERT INTO match_events(match_id,command_id,kind,side,player_id,stat,game_ms,shot_ms,period,running,shot_running) VALUES (?1,?2,'record',?3,?4,?5,?6,?7,?8,?9,?10)",params![id,command.to_string(),side,player,tag,stamp.game_ms as i64,stamp.shot_ms as i64,stamp.period,stamp.running,stamp.shot_running])?;
        if let Some(clock) = clock {
            write_clock(&tx, id, clock.for_stat(stat))?;
        }
        tx.execute("UPDATE matches SET revision=revision+1 WHERE id=?1", [id])?;
        let saved = load(&tx, id)?;
        tx.commit()?;
        Ok(saved)
    }
    pub fn undo(&mut self, id: i64, revision: i64, command: Uuid) -> Result<SavedMatch> {
        self.undo_with_clock(id, revision, command, None)
    }
    pub fn undo_with_clock(
        &mut self,
        id: i64,
        revision: i64,
        command: Uuid,
        clock: Option<ClockState>,
    ) -> Result<SavedMatch> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        event_command_available(&tx, command)?;
        let existing = tx
            .query_row(
                "SELECT match_id,kind FROM match_events WHERE command_id=?1",
                [command.to_string()],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?;
        if let Some(existing) = existing {
            if existing != (id, "undo".into()) {
                return invalid(
                    "This save identifier was already used for a different action. Reopen the match.",
                );
            }
            return load(&tx, id);
        }
        let current = load(&tx, id)?;
        basketball_only(&current)?;
        if current.summary.revision != revision {
            return Err(Error::Conflict);
        }
        let target=tx.query_row("SELECT e.id FROM match_events e WHERE e.match_id=?1 AND e.kind='record' AND NOT EXISTS(SELECT 1 FROM match_events u WHERE u.target_id=e.id) ORDER BY e.id DESC LIMIT 1",[id],|r|r.get::<_,i64>(0)).optional()?;
        let Some(target) = target else {
            return invalid("There is no play to undo.");
        };
        tx.execute(
            "INSERT INTO match_events(match_id,command_id,kind,target_id) VALUES (?1,?2,'undo',?3)",
            params![id, command.to_string(), target],
        )?;
        if let Some(clock) = clock {
            write_clock(&tx, id, clock)?;
        }
        tx.execute("UPDATE matches SET revision=revision+1 WHERE id=?1", [id])?;
        let saved = load(&tx, id)?;
        tx.commit()?;
        Ok(saved)
    }
    pub fn save_clock(
        &mut self,
        id: i64,
        revision: i64,
        command: Uuid,
        clock: ClockState,
    ) -> Result<SavedMatch> {
        clock.validate()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        lineups::command_available(&tx, command)?;
        sports::command_available(&tx, command)?;
        let existing=tx.query_row("SELECT match_id,game_ms,shot_ms,period,running,shot_running FROM clock_commands WHERE command_id=?1",[command.to_string()],|r|Ok((r.get::<_,i64>(0)?,clock_row(r,1)?))).optional()?;
        if let Some(existing) = existing {
            if existing != (id, clock) {
                return invalid("This save identifier already belongs to another clock command.");
            }
            return load(&tx, id);
        }
        let used: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM match_events WHERE command_id=?1)",
            [command.to_string()],
            |r| r.get(0),
        )?;
        if used {
            return invalid("This save identifier already belongs to a stat command.");
        }
        let current = load(&tx, id)?;
        basketball_only(&current)?;
        if current.summary.revision != revision {
            return Err(Error::Conflict);
        }
        write_clock(&tx, id, clock)?;
        tx.execute("INSERT INTO clock_commands(command_id,match_id,game_ms,shot_ms,period,running,shot_running) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![command.to_string(),id,clock.game_ms as i64,clock.shot_ms as i64,clock.period,clock.running,clock.shot_running])?;
        tx.execute("UPDATE matches SET revision=revision+1 WHERE id=?1", [id])?;
        let saved = load(&tx, id)?;
        tx.commit()?;
        Ok(saved)
    }
}
fn required<'a>(value: &'a str, label: &str, limit: usize) -> Result<&'a str> {
    let value = bounded(value, label, limit)?;
    if value.is_empty() {
        return invalid(&format!("{label} is required."));
    }
    Ok(value)
}
fn bounded<'a>(value: &'a str, label: &str, limit: usize) -> Result<&'a str> {
    let value = value.trim();
    if value.chars().count() > limit {
        return invalid(&format!("{label} must be {limit} characters or fewer."));
    }
    Ok(value)
}
fn invalid<T>(message: &str) -> Result<T> {
    Err(Error::Validation(message.into()))
}
fn changed(count: usize) -> Result<()> {
    if count == 1 {
        Ok(())
    } else {
        invalid("This item no longer exists. Refresh the library.")
    }
}
fn active_name(db: &Connection, table: &str, id: i64) -> Result<String> {
    db.query_row(
        &format!("SELECT name FROM {table} WHERE id=?1 AND archived=0"),
        [id],
        |r| r.get(0),
    )
    .optional()?
    .ok_or_else(|| Error::Validation("Choose an existing, active team or player.".into()))
}
fn roster(db: &Connection, id: i64) -> Result<Vec<RosterEntry>> {
    let mut stmt=db.prepare("SELECT p.id,p.name,CASE WHEN tp.jersey='' THEN p.jersey ELSE tp.jersey END,p.positions FROM team_players tp JOIN players p ON p.id=tp.player_id WHERE tp.team_id=?1 AND p.archived=0 ORDER BY tp.rowid")?;
    Ok(stmt
        .query_map([id], |r| {
            Ok(RosterEntry {
                player_id: r.get(0)?,
                name: r.get(1)?,
                jersey: r.get(2)?,
                positions: profile::decode(&r.get::<_, String>(3)?),
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
}
fn summary(r: &rusqlite::Row<'_>) -> rusqlite::Result<MatchSummary> {
    Ok(MatchSummary {
        id: r.get(0)?,
        name: r.get(1)?,
        date: r.get(2)?,
        venue: r.get(3)?,
        home_name: r.get(4)?,
        away_name: r.get(5)?,
        revision: r.get(6)?,
        sport: sports::parse_sport(r.get::<_, String>(7)?.as_str()).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(e))
        })?,
    })
}
fn load(db: &Connection, id: i64) -> Result<SavedMatch> {
    let summary = db.query_row(
        "SELECT id,name,date,venue,home_name,away_name,revision,sport FROM matches WHERE id=?1",
        [id],
        summary,
    )?;
    let teams = [
        Team {
            name: summary.home_name.clone(),
            players: load_players(db, id, "home")?,
        },
        Team {
            name: summary.away_name.clone(),
            players: load_players(db, id, "away")?,
        },
    ];
    let mut stmt=db.prepare("SELECT e.side,e.player_id,e.stat,e.game_ms,e.shot_ms,e.period,e.running,e.shot_running FROM match_events e WHERE e.match_id=?1 AND e.kind='record' AND NOT EXISTS(SELECT 1 FROM match_events u WHERE u.target_id=e.id) ORDER BY e.id")?;
    let rows = stmt.query_map([id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, String>(2)?,
            event_clock_row(r, 3)?,
        ))
    })?;
    let mut plays = Vec::new();
    for row in rows {
        let (side, player_id, stat, clock) = row?;
        if let Some(stamp) = clock {
            stamp.validate()?;
        }
        plays.push(Play {
            side: parse_side(&side)?,
            player_id,
            stat: parse_stat(&stat)?,
            clock,
        });
    }
    let clock = db.query_row(
        "SELECT game_ms,shot_ms,period,running,shot_running FROM match_clocks WHERE match_id=?1",
        [id],
        |r| clock_row(r, 0),
    )?;
    clock.validate()?;
    let (sport_state, sport_history) = sports::projection(db, id, summary.sport)?;
    Ok(SavedMatch {
        sport_state,
        sport_history,
        summary,
        clock,
        lineups: lineups::history(db, id)?,
        game: Game::from_saved(teams, plays),
    })
}
fn load_players(db: &Connection, id: i64, side: &str) -> Result<Vec<Player>> {
    let mut stmt=db.prepare("SELECT player_id,name,jersey,on_court,positions FROM match_players WHERE match_id=?1 AND side=?2 ORDER BY sort_order")?;
    Ok(stmt
        .query_map(params![id, side], |r| {
            Ok(Player {
                id: r.get(0)?,
                name: r.get(1)?,
                jersey: r.get(2)?,
                on_court: r.get(3)?,
                positions: profile::decode(&r.get::<_, String>(4)?),
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
}
fn side_key(side: Side) -> &'static str {
    match side {
        Side::Home => "home",
        Side::Away => "away",
    }
}
fn parse_side(value: &str) -> Result<Side> {
    match value {
        "home" => Ok(Side::Home),
        "away" => Ok(Side::Away),
        _ => invalid("Unsupported side in saved match."),
    }
}
fn stat_key(stat: Stat) -> &'static str {
    match stat {
        Stat::FreeThrowMade => "ft_made",
        Stat::TwoMade => "two_made",
        Stat::ThreeMade => "three_made",
        Stat::FreeThrowMiss => "ft_miss",
        Stat::TwoMiss => "two_miss",
        Stat::ThreeMiss => "three_miss",
        Stat::OffensiveRebound => "orb",
        Stat::DefensiveRebound => "drb",
        Stat::Assist => "assist",
        Stat::Turnover => "turnover",
        Stat::Steal => "steal",
        Stat::Block => "block",
        Stat::Foul => "foul",
        Stat::Technical => "technical",
        Stat::Unsportsmanlike => "unsportsmanlike",
        Stat::Disqualifying => "disqualifying",
        Stat::Warning => "warning",
    }
}
fn parse_stat(value: &str) -> Result<Stat> {
    match value {
        "ft_made" => Ok(Stat::FreeThrowMade),
        "two_made" => Ok(Stat::TwoMade),
        "three_made" => Ok(Stat::ThreeMade),
        "ft_miss" => Ok(Stat::FreeThrowMiss),
        "two_miss" => Ok(Stat::TwoMiss),
        "three_miss" => Ok(Stat::ThreeMiss),
        "orb" => Ok(Stat::OffensiveRebound),
        "drb" => Ok(Stat::DefensiveRebound),
        "assist" => Ok(Stat::Assist),
        "turnover" => Ok(Stat::Turnover),
        "steal" => Ok(Stat::Steal),
        "block" => Ok(Stat::Block),
        "foul" => Ok(Stat::Foul),
        "technical" => Ok(Stat::Technical),
        "unsportsmanlike" => Ok(Stat::Unsportsmanlike),
        "disqualifying" => Ok(Stat::Disqualifying),
        "warning" => Ok(Stat::Warning),
        _ => invalid("Unsupported stat in saved match. Use a compatible application version."),
    }
}

fn event_clock_row(r: &rusqlite::Row<'_>, offset: usize) -> rusqlite::Result<Option<ClockState>> {
    let fields = (offset..offset + 5)
        .map(|index| r.get::<_, Option<i64>>(index))
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if fields.iter().all(Option::is_none) {
        Ok(None)
    } else if fields.iter().all(Option::is_some) {
        clock_row(r, offset).map(Some)
    } else {
        Err(rusqlite::Error::InvalidQuery)
    }
}

fn clock_row(r: &rusqlite::Row<'_>, offset: usize) -> rusqlite::Result<ClockState> {
    Ok(ClockState {
        game_ms: r.get::<_, i64>(offset)? as u64,
        shot_ms: r.get::<_, i64>(offset + 1)? as u64,
        period: r.get(offset + 2)?,
        running: r.get(offset + 3)?,
        shot_running: r.get(offset + 4)?,
    })
}
fn write_clock(db: &Connection, id: i64, clock: ClockState) -> Result<()> {
    clock.validate()?;
    changed(db.execute("UPDATE match_clocks SET game_ms=?1,shot_ms=?2,period=?3,running=?4,shot_running=?5 WHERE match_id=?6",params![clock.game_ms as i64,clock.shot_ms as i64,clock.period,clock.running,clock.shot_running,id])?)
}
fn event_command_available(db: &Connection, command: Uuid) -> Result<()> {
    lineups::command_available(db, command)?;
    sports::command_available(db, command)?;
    let used: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM clock_commands WHERE command_id=?1)",
        [command.to_string()],
        |r| r.get(0),
    )?;
    if used {
        return invalid("This save identifier already belongs to a clock command.");
    }
    Ok(())
}

fn basketball_only(saved: &SavedMatch) -> Result<()> {
    if saved.summary.sport != Sport::Basketball {
        return invalid(
            "Use this sport's score controls. Basketball commands do not apply to this match.",
        );
    }
    Ok(())
}
