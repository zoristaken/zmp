CREATE TABLE IF NOT EXISTS song (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    title        TEXT NOT NULL,
    artist       TEXT,
    release_year INTEGER,
    album        TEXT,
    remix        TEXT,
    search_blob  TEXT,
    UNIQUE(title, artist, remix)
);