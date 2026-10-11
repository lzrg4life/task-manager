CREATE TABLE ticket_relations (
    ticket_id         INTEGER NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    related_ticket_id INTEGER NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    created_by        INTEGER NOT NULL REFERENCES users(id),
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    PRIMARY KEY (ticket_id, related_ticket_id),
    CHECK (ticket_id < related_ticket_id)
);
