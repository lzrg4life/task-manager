CREATE TABLE tickets (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    title                  TEXT NOT NULL,
    details                TEXT NOT NULL DEFAULT '',
    business_requirements  TEXT NOT NULL DEFAULT '',
    technical_requirements TEXT NOT NULL DEFAULT '',
    implementation_notes   TEXT NOT NULL DEFAULT '',
    status                 TEXT NOT NULL DEFAULT 'backlog'
        CHECK (status IN ('archived','completed','in_review','active','up_next','backlog','removed')),
    priority               TEXT NOT NULL DEFAULT 'normal'
        CHECK (priority IN ('normal','high','low')),
    created_by             INTEGER NOT NULL REFERENCES users(id),
    reviewer_id            INTEGER REFERENCES users(id),
    implementer_id         INTEGER REFERENCES users(id),
    created_at             TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    updated_at             TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);
CREATE INDEX idx_tickets_status ON tickets(status, id);

CREATE TABLE ticket_history (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    ticket_id  INTEGER NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    field      TEXT NOT NULL CHECK (field IN ('status','priority')),
    old_value  TEXT,
    new_value  TEXT NOT NULL,
    changed_by INTEGER NOT NULL REFERENCES users(id),
    changed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);
CREATE INDEX idx_ticket_history_ticket ON ticket_history(ticket_id, id);
