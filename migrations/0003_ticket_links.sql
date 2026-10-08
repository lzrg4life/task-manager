CREATE TABLE ticket_links (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    ticket_id  INTEGER NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    url        TEXT NOT NULL,
    label      TEXT NOT NULL DEFAULT '',
    created_by INTEGER NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    UNIQUE (ticket_id, url)
);

-- Rebuild ticket_history to allow link events and a NULL new_value (link removal).
CREATE TABLE ticket_history_new (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    ticket_id  INTEGER NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    field      TEXT NOT NULL CHECK (field IN ('status','priority','link_added','link_removed')),
    old_value  TEXT,
    new_value  TEXT,
    changed_by INTEGER NOT NULL REFERENCES users(id),
    changed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);
INSERT INTO ticket_history_new SELECT * FROM ticket_history;
DROP TABLE ticket_history;
ALTER TABLE ticket_history_new RENAME TO ticket_history;
CREATE INDEX idx_ticket_history_ticket ON ticket_history(ticket_id, id);
