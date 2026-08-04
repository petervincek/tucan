-- Add migration script here

-- 1. Create the COLUMNS table, a table that will contain all the columns of
-- kanban board that the user would like to have present on the board
CREATE TABLE columns (
    id TEXT PRIMARY KEY,                 -- UUID or slug (e.g., "in_progress")
    name TEXT NOT NULL UNIQUE,           -- Display name (e.g., "In Progress")
    wip_limit INTEGER,                   -- NULL means no limit
    position INTEGER NOT NULL UNIQUE,    -- Left-to-right order (0, 1, 2...)
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 2. CREATE the CARDS table, a table that will contain all the cards of
-- kanban board that the suer would like to create, track and manage on the board
CREATE TABLE cards (
    id TEXT PRIMARY KEY,                 -- UUID
    column_id TEXT NOT NULL,             -- Foreign key linking to table COLUMNS
    title TEXT NOT NULL,
    description TEXT,                    -- this will be markup text
    status TEXT NOT NULL CHECK (status IN ('Active', 'Blocked')),
    blocked_reason TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    started_at DATETIME,
    completed_at DATETIME,
    FOREIGN KEY(column_id) REFERENCES columns(id) ON DELETE CASCADE
);