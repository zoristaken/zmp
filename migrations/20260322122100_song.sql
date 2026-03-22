CREATE TABLE IF NOT EXISTS song (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    title        TEXT NOT NULL,
    artist       TEXT,
    release_year INTEGER,
    album        TEXT,
    remix        TEXT,
    search_blob  TEXT,
    file_path    TEXT,
    UNIQUE(title, artist, remix)
);