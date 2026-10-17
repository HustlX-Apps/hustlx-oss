use super::*;
use crate::lineup::{self, LineupChange, LineupKind};
fn encode(ids: &[i64]) -> String {
    ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",")
}
fn decode(value: &str) -> Result<Vec<i64>> {
    if value.is_empty() {
        return Ok(vec![]);
    }
    value
        .split(',')
        .map(|id| {
            id.parse().map_err(|_| {
                Error::Validation(
                    "Invalid saved lineup. Preserve the library and use a compatible version."
                        .into(),
                )
            })
        })
        .collect()
}
impl SavedMatch {
    pub fn starting_lineup(&self, side: Side) -> Option<&[i64]> {
        self.lineups
            .iter()
            .rev()
            .find(|l| l.side == side && l.kind == LineupKind::Starters)
            .map(|l| l.court.as_slice())
    }
}
impl Store {
    #[allow(clippy::too_many_arguments)]
    pub fn save_lineup(
        &mut self,
        id: i64,
        revision: i64,
        command: Uuid,
        side: Side,
        kind: LineupKind,
        court: &[i64],
        clock: ClockState,
    ) -> Result<SavedMatch> {
        clock.validate()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing=tx.query_row("SELECT match_id,side,kind,court_players,game_ms,shot_ms,period,running,shot_running FROM lineup_commands WHERE command_id=?1",[command.to_string()],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,clock_row(r,4)?))).optional()?;
        if let Some(existing) = existing {
            if existing
                != (
                    id,
                    side_key(side).into(),
                    kind.key().into(),
                    encode(court),
                    clock,
                )
            {
                return invalid(
                    "This save identifier already belongs to a different lineup command.",
                );
            }
            return load(&tx, id);
        }
        event_command_available(&tx, command)?;
        let used: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM match_events WHERE command_id=?1)",
            [command.to_string()],
            |r| r.get(0),
        )?;
        if used {
            return invalid("This save identifier already belongs to a stat command.");
        }
        let current = load(&tx, id)?;
        if current.summary.revision != revision {
            return Err(Error::Conflict);
        }
        lineup::validate(&current.game, side, court)?;
        let before = current.game.court_ids(side);
        // Put court players first in their chosen slots, then preserve bench order.
        let order: Vec<i64> = court
            .iter()
            .copied()
            .chain(
                current
                    .game
                    .team(side)
                    .players
                    .iter()
                    .filter(|p| !court.contains(&p.id))
                    .map(|p| p.id),
            )
            .collect();
        for (slot, player) in order.iter().enumerate() {
            changed(tx.execute("UPDATE match_players SET on_court=?1,sort_order=?2 WHERE match_id=?3 AND side=?4 AND player_id=?5",params![slot<court.len(),slot as i64,id,side_key(side),player])?)?;
        }
        write_clock(&tx, id, clock)?;
        tx.execute("INSERT INTO lineup_commands(command_id,match_id,side,kind,before_players,court_players,game_ms,shot_ms,period,running,shot_running) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![command.to_string(),id,side_key(side),kind.key(),encode(&before),encode(court),clock.game_ms as i64,clock.shot_ms as i64,clock.period,clock.running,clock.shot_running])?;
        tx.execute("UPDATE matches SET revision=revision+1 WHERE id=?1", [id])?;
        let saved = load(&tx, id)?;
        tx.commit()?;
        Ok(saved)
    }
}
pub(super) fn command_available(db: &Connection, command: Uuid) -> Result<()> {
    let used: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM lineup_commands WHERE command_id=?1)",
        [command.to_string()],
        |r| r.get(0),
    )?;
    if used {
        return invalid("This save identifier already belongs to a lineup command.");
    }
    Ok(())
}
pub(super) fn history(db: &Connection, id: i64) -> Result<Vec<LineupChange>> {
    let mut stmt=db.prepare("SELECT side,kind,before_players,court_players,game_ms,shot_ms,period,running,shot_running FROM lineup_commands WHERE match_id=?1 ORDER BY id")?;
    let rows = stmt.query_map([id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            clock_row(r, 4)?,
        ))
    })?;
    let mut result = vec![];
    for row in rows {
        let (side, kind, before, court, clock) = row?;
        let kind = match kind.as_str() {
            "starters" => LineupKind::Starters,
            "substitution" => LineupKind::Substitution,
            _ => return invalid("Unsupported lineup command."),
        };
        clock.validate()?;
        result.push(LineupChange {
            side: parse_side(&side)?,
            kind,
            before: decode(&before)?,
            court: decode(&court)?,
            clock,
        });
    }
    Ok(result)
}
