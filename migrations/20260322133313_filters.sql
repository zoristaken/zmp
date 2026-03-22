CREATE TABLE IF NOT EXISTS filters
(
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    name                    TEXT,
    UNIQUE(name)
);