-- Lesson 03, migration 2: a later change arrives as a new file.
-- The foreign key column is read by every "items of this shelf" query.
CREATE INDEX item_by_shelf ON item (shelf_id);
