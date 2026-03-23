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

CREATE TABLE IF NOT EXISTS filters
(
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    name                    TEXT,
    UNIQUE(name)
);

CREATE TABLE IF NOT EXISTS song_filters
(
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    song_id                 INT,
    filter_id               INT,
    FOREIGN KEY (song_id)   REFERENCES song (id) ON UPDATE SET NULL ON DELETE SET NULL,
    FOREIGN KEY (filter_id) REFERENCES filters (id) ON UPDATE SET NULL ON DELETE SET NULL,
    UNIQUE(song_id, filter_id)
);