-- Earlier stat events have no trustworthy clock context. Keep their stamps NULL.
ALTER TABLE match_events ADD COLUMN game_ms INTEGER CHECK(game_ms BETWEEN 0 AND 86400000);
ALTER TABLE match_events ADD COLUMN shot_ms INTEGER CHECK(shot_ms BETWEEN 0 AND 120000);
ALTER TABLE match_events ADD COLUMN period INTEGER CHECK(period BETWEEN 1 AND 999);
ALTER TABLE match_events ADD COLUMN running INTEGER CHECK(running IN (0,1));
ALTER TABLE match_events ADD COLUMN shot_running INTEGER CHECK(shot_running IN (0,1))
    CHECK(
        (game_ms IS NULL AND shot_ms IS NULL AND period IS NULL AND running IS NULL AND shot_running IS NULL)
        OR
        (kind='record' AND game_ms IS NOT NULL AND shot_ms IS NOT NULL AND period IS NOT NULL
            AND running IS NOT NULL AND shot_running IS NOT NULL
            AND (shot_running=0 OR (running=1 AND shot_ms>0))
            AND (running=0 OR game_ms>0))
    );
