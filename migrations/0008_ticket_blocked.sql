-- A non-NULL blocked_reason means the ticket is blocked.
ALTER TABLE tickets ADD COLUMN blocked_reason TEXT;

-- Rebuild ticket_history to allow blocked/unblocked events.
CREATE TABLE ticket_history_new (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    ticket_id  INTEGER NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    field      TEXT NOT NULL CHECK (field IN ('status','priority','link_added','link_removed','blocked','unblocked')),
    old_value  TEXT,
    new_value  TEXT,
    changed_by INTEGER NOT NULL REFERENCES users(id),
    changed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);
INSERT INTO ticket_history_new SELECT * FROM ticket_history;
DROP TABLE ticket_history;
ALTER TABLE ticket_history_new RENAME TO ticket_history;
CREATE INDEX idx_ticket_history_ticket ON ticket_history(ticket_id, id);
