//! Durable versioned sport commands. Rules and side names are match-time snapshots.
use super::*;
use crate::sport::{Command, Rules};

#[derive(Debug, Clone)]
pub struct SportMatchDraft {
    pub name: String,
    pub date: String,
    pub venue: String,
    pub home_name: String,
    pub away_name: String,
    pub sport: Sport,
    pub rules: Rules,
}

#[derive(Debug, Clone)]
pub struct SportEvent {
    pub id: i64,
    pub command: Option<Command>,
    pub target_id: Option<i64>,
    pub undone: bool,
}
impl SportEvent {
    pub fn label(&self) -> String {
        let side_name = |side| match side {
            Side::Home => "A",
            Side::Away => "B",
        };
        match self.command {
            Some(Command::Rally(side)) => format!("Rally won by {}", side_name(side)),
            Some(Command::SetServer(side)) => format!("Server corrected to {}", side_name(side)),
            Some(Command::Complete) => "Match completed".into(),
            None => format!("Undo action #{}", self.target_id.unwrap_or_default()),
        }
    }
}

impl Store {
    pub fn create_sport_match(&mut self, draft: &SportMatchDraft) -> Result<SavedMatch> {
        State::new(draft.sport, draft.rules.clone()).map_err(engine_error)?;
        let name = required(&draft.name, "Match name", 200)?;
        let date = draft.date.trim();
        if date.len() != 10 || NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
            return invalid("Enter a real match date as YYYY-MM-DD.");
        }
        let venue = bounded(&draft.venue, "Venue", 300)?;
        let home = required(&draft.home_name, "Side A name", 200)?;
        let away = required(&draft.away_name, "Side B name", 200)?;
        let rules = serde_json::to_string(&draft.rules).map_err(json_error)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Never match identities by name. These new local sides have independent IDs,
        // including when both display names are the same.
        tx.execute("INSERT INTO teams(name) VALUES(?1)", [home])?;
        let home_id = tx.last_insert_rowid();
        tx.execute("INSERT INTO teams(name) VALUES(?1)", [away])?;
        let away_id = tx.last_insert_rowid();
        tx.execute("INSERT INTO matches(name,date,venue,home_team_id,away_team_id,home_name,away_name,sport) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)", params![name,date,venue,home_id,away_id,home,away,draft.sport.slug()])?;
        let id = tx.last_insert_rowid();
        // Retained for structural compatibility; sport commands never run these clocks.
        tx.execute("INSERT INTO match_clocks(match_id) VALUES(?1)", [id])?;
        tx.execute("INSERT INTO sport_matches(match_id,local_id,rules_version,rules_json) VALUES(?1,?2,1,?3)", params![id,Uuid::new_v4().to_string(),rules])?;
        let saved = load(&tx, id)?;
        tx.commit()?;
        Ok(saved)
    }

    pub fn record_sport(
        &mut self,
        id: i64,
        revision: i64,
        command_id: Uuid,
        command: Command,
    ) -> Result<SavedMatch> {
        let payload = serde_json::to_string(&command).map_err(json_error)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        other_command_available(&tx, command_id)?;
        if let Some((match_id, kind, old_payload)) = receipt(&tx, command_id)? {
            if (match_id, kind.as_str(), old_payload.as_deref())
                != (id, "record", Some(payload.as_str()))
            {
                return invalid(
                    "This save identifier already belongs to a different action. Reopen the match.",
                );
            }
            return load(&tx, id);
        }
        let current = load(&tx, id)?;
        if current.summary.revision != revision {
            return Err(Error::Conflict);
        }
        let mut state = current.sport_state.ok_or_else(|| {
            Error::Validation("Use the basketball controls for this match.".into())
        })?;
        state.apply(command).map_err(engine_error)?;
        tx.execute("INSERT INTO sport_events(match_id,command_id,payload_version,kind,payload) VALUES(?1,?2,1,'record',?3)", params![id,command_id.to_string(),payload])?;
        tx.execute("UPDATE matches SET revision=revision+1 WHERE id=?1", [id])?;
        let saved = load(&tx, id)?;
        tx.commit()?;
        Ok(saved)
    }

    pub fn undo_sport(&mut self, id: i64, revision: i64, command_id: Uuid) -> Result<SavedMatch> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        other_command_available(&tx, command_id)?;
        if let Some((match_id, kind, _)) = receipt(&tx, command_id)? {
            if (match_id, kind.as_str()) != (id, "undo") {
                return invalid(
                    "This save identifier already belongs to a different action. Reopen the match.",
                );
            }
            return load(&tx, id);
        }
        let current = load(&tx, id)?;
        if current.summary.revision != revision {
            return Err(Error::Conflict);
        }
        if current.sport_state.is_none() {
            return invalid("Use the basketball undo control for this match.");
        }
        let target = tx.query_row("SELECT e.id FROM sport_events e WHERE e.match_id=?1 AND e.kind='record' AND NOT EXISTS(SELECT 1 FROM sport_events u WHERE u.target_id=e.id) ORDER BY e.id DESC LIMIT 1", [id], |r| r.get::<_,i64>(0)).optional()?;
        let Some(target) = target else {
            return invalid("There is no sport action to undo.");
        };
        tx.execute("INSERT INTO sport_events(match_id,command_id,payload_version,kind,target_id) VALUES(?1,?2,1,'undo',?3)", params![id,command_id.to_string(),target])?;
        tx.execute("UPDATE matches SET revision=revision+1 WHERE id=?1", [id])?;
        let saved = load(&tx, id)?;
        tx.commit()?;
        Ok(saved)
    }
}

pub(super) fn parse_sport(slug: &str) -> Result<Sport> {
    Sport::ALL
        .into_iter()
        .find(|sport| sport.slug() == slug)
        .ok_or_else(|| {
            Error::Validation(
                "Unsupported saved sport. Preserve this library and use a compatible app version."
                    .into(),
            )
        })
}

pub(super) fn projection(
    db: &Connection,
    id: i64,
    sport: Sport,
) -> Result<(Option<State>, Vec<SportEvent>)> {
    if sport == Sport::Basketball {
        return Ok((None, vec![]));
    }
    let (version, rules) = db.query_row(
        "SELECT rules_version,rules_json FROM sport_matches WHERE match_id=?1",
        [id],
        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
    )?;
    if version != 1 {
        return invalid("Unsupported saved sport rules version. Use a compatible app version.");
    }
    let rules: Rules = serde_json::from_str(&rules).map_err(json_error)?;
    let mut state = State::new(sport, rules).map_err(engine_error)?;
    let mut stmt = db.prepare("SELECT e.id,e.payload_version,e.kind,e.payload,e.target_id,EXISTS(SELECT 1 FROM sport_events u WHERE u.target_id=e.id) FROM sport_events e WHERE e.match_id=?1 ORDER BY e.id")?;
    let rows = stmt.query_map([id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, Option<String>>(3)?,
            r.get::<_, Option<i64>>(4)?,
            r.get::<_, bool>(5)?,
        ))
    })?;
    let mut events = vec![];
    for row in rows {
        let (event_id, version, kind, payload, target_id, undone) = row?;
        if version != 1 {
            return invalid(
                "Unsupported saved sport command version. Use a compatible app version.",
            );
        }
        let command = match kind.as_str() {
            "record" => Some(
                serde_json::from_str::<Command>(
                    payload
                        .as_deref()
                        .ok_or_else(|| Error::Validation("Missing saved sport command.".into()))?,
                )
                .map_err(json_error)?,
            ),
            "undo" => None,
            _ => return invalid("Unsupported saved sport action."),
        };
        if let Some(command) = command
            && !undone
        {
            state.apply(command).map_err(engine_error)?;
        }
        events.push(SportEvent {
            id: event_id,
            command,
            target_id,
            undone,
        });
    }
    Ok((Some(state), events))
}

fn receipt(db: &Connection, command: Uuid) -> Result<Option<(i64, String, Option<String>)>> {
    Ok(db
        .query_row(
            "SELECT match_id,kind,payload FROM sport_events WHERE command_id=?1",
            [command.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?)
}

pub(super) fn command_available(db: &Connection, command: Uuid) -> Result<()> {
    let used: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sport_events WHERE command_id=?1)",
        [command.to_string()],
        |r| r.get(0),
    )?;
    if used {
        return invalid("This save identifier already belongs to a sport command.");
    }
    Ok(())
}
fn other_command_available(db: &Connection, command: Uuid) -> Result<()> {
    let used: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM match_events WHERE command_id=?1 UNION ALL SELECT 1 FROM clock_commands WHERE command_id=?1 UNION ALL SELECT 1 FROM lineup_commands WHERE command_id=?1)", [command.to_string()], |r| r.get(0))?;
    if used {
        return invalid("This save identifier already belongs to another action.");
    }
    Ok(())
}
fn engine_error(error: crate::sport::ScoreError) -> Error {
    Error::Validation(error.to_string())
}
fn json_error(_error: serde_json::Error) -> Error {
    Error::Validation(
        "Invalid saved sport data. Preserve this library and use a compatible app version.".into(),
    )
}
