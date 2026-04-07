CREATE TABLE IF NOT EXISTS song (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    title        TEXT NOT NULL,
    artist       TEXT NOT NULL,
    release_year INTEGER,
    album        TEXT,
    remix        TEXT,
    search_blob  TEXT,
    file_path    TEXT,
    extension    TEXT,
    duration     INTEGER,
    UNIQUE(title, artist, remix)
);

CREATE TABLE IF NOT EXISTS filter
(
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    name                    TEXT NOT NULL,
    UNIQUE(name)
);

CREATE TABLE IF NOT EXISTS song_filter
(
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    song_id                 INT NOT NULL,
    filter_id               INT NOT NULL,
    FOREIGN KEY (song_id)   REFERENCES song (id) ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY (filter_id) REFERENCES filter (id) ON UPDATE CASCADE ON DELETE CASCADE,
    UNIQUE(song_id, filter_id)
);

CREATE TABLE IF NOT EXISTS setting
(
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    key     TEXT NOT NULL,
    value   TEXT NOT NULL,
    UNIQUE(key)
);
