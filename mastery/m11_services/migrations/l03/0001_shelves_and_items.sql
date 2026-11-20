-- Lesson 03, migration 1: the first version of the schema.
-- A migration file is applied once, in file-name order, and never edited afterwards.
CREATE TABLE shelf (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
) STRICT;

CREATE TABLE item (
    id       INTEGER PRIMARY KEY,
    shelf_id INTEGER NOT NULL REFERENCES shelf (id),
    label    TEXT NOT NULL,
    qty      INTEGER NOT NULL CHECK (qty >= 0)
) STRICT;
