import random
import os
import pathlib
from datetime import datetime

num_rows = 100000
# Prepare migration folder and filename
migration_dir = pathlib.Path().resolve().as_posix() + '/migrations'
os.makedirs(migration_dir, exist_ok=True)
migration_name = f"{datetime.now().strftime('%Y%m%d%H%M%S')}_create_{num_rows}_songs.sql"
filename = os.path.join(migration_dir, migration_name)

artists = ["Metallica", "Nirvana", "Queen", "Radiohead", "Adele", "Coldplay", "U2", "Pink Floyd"]
albums = ["Greatest Hits", "Live Album", "Studio Sessions", "Acoustic", "Remastered", "Unplugged"]
words = ["love", "dream", "fire", "night", "moon", "star", "rock", "sound", "metal", "song", "boomb", "powpow", "skkrrt", "amazing"]

with open(filename, mode='w', encoding='utf-8') as f:

    for _ in range(num_rows):
        title = ' '.join(random.choices(words, k=3))
        artist = random.choice(artists)
        album = random.choice(albums)
        remix = f"{random.choice(words)} Remix" if random.random() < 0.2 else ''
        release_year = random.randint(1970, 2025)

        search_blob = f"{title} {artist} {release_year} {album} {remix}".lower() if remix else f"{title} {artist} {release_year} {album}".lower()
        file_path = f"/home/Music/{title.replace(' ', '_')}"
        duration = random.randint(60, 600)

        # Escape single quotes
        title_esc = title.replace("'", "''")
        artist_esc = artist.replace("'", "''")
        album_esc = album.replace("'", "''")
        remix_esc = remix.replace("'", "''") if remix else ''
        search_blob_esc = search_blob.replace("'", "''")
        file_path_esc = file_path.replace("'", "''")

        f.write(f"INSERT OR IGNORE INTO song (title, artist, release_year, album, remix, search_blob, file_path, duration) VALUES ('{title_esc}', '{artist_esc}', {release_year}, '{album_esc}', '{remix_esc}', '{search_blob_esc}', '{file_path_esc}', '{duration}');\n")

print(f"SQL migration file '{filename}' with {num_rows} INSERT statements created.")
