-- One row per sample. The primary key is also the idempotency rule: a sample
-- that arrives twice (a batch resent after a lost Ack) is the same row.
CREATE TABLE sample (
    agent  TEXT    NOT NULL,
    metric TEXT    NOT NULL,
    ts_ms  INTEGER NOT NULL,
    value  INTEGER NOT NULL,
    PRIMARY KEY (agent, metric, ts_ms)
) STRICT, WITHOUT ROWID;
