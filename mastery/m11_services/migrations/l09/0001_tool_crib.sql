-- Lesson 09: the tool crib. Supplied.
CREATE TABLE tool (
    id       INTEGER PRIMARY KEY,
    name     TEXT NOT NULL UNIQUE,
    borrower TEXT             -- NULL while the tool is on its hook
) STRICT;

-- One row per borrowing, kept as history.
CREATE TABLE loan (
    id      INTEGER PRIMARY KEY,
    tool_id INTEGER NOT NULL REFERENCES tool (id),
    who     TEXT NOT NULL
) STRICT;
