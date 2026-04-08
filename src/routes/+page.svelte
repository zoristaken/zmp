<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, tick } from "svelte";
  import {
    Play,
    Pause,
    SkipBack,
    SkipForward,
    Shuffle,
    Repeat,
    Volume2,
    Volume1,
    VolumeX,
    Search,
  } from "lucide-svelte";

  type Song = {
    id: number;
    title: string;
    artist: string;
    release_year: number;
    album: string;
    remix: string;
    search_blob: string;
    file_path: string;
    duration: number;
    extension: string;
  };

  type TrackChangedPayload = {
    currentIndex: number | null;
  };

  let isPlaying = false;
  let isShuffle = false;
  let isRepeat = false;

  let volume = 50;
  let previousVolume = 50;
  let isMuted = false;

  let searchQuery = "";
  let searchResultCount = 0;
  let songs: Song[] = [];
  let selectedIndex: number | null = null;
  let selectedSongId: number | null = null;
  let currentSong: Song | null = null;

  let currentSeekSeconds = 0;
  let isSeeking = false;
  let isProgrammaticSeekReset = false;
  let isDraggingSeek = false;

  let volumeTimeout: ReturnType<typeof setTimeout> | undefined;
  let playbackInterval: ReturnType<typeof setInterval> | undefined;
  let searchTimeout: ReturnType<typeof setTimeout> | undefined;
  let songRowElements: Array<HTMLDivElement | null> = [];

  let hasInitialized = false;
  let lastSearchedQuery = "";

  function formatDuration(durationSeconds: number): string {
    const totalSeconds = Math.max(0, Math.floor(durationSeconds));
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return `${minutes}:${seconds.toString().padStart(2, "0")}`;
  }

  function formatDate(year: number): string {
    return year > 0 ? String(year) : "—";
  }

  function stopPlaybackTicker() {
    if (playbackInterval) {
      clearInterval(playbackInterval);
      playbackInterval = undefined;
    }
  }

  async function ensureSelectedSongIsVisible() {
    await tick();

    const index = songs.findIndex((song) => song.id === selectedSongId);
    if (index < 0) return;

    const row = songRowElements[index];
    row?.scrollIntoView({
      block: "center",
      inline: "nearest",
      behavior: "smooth",
    });
  }

  async function saveSeekProgress() {
    try {
      await invoke("save_current_song_seek", {
        seekValue: currentSeekSeconds,
      });
    } catch (err) {
      console.error("Failed to save seek position:", err);
    }
  }

  function startPlaybackTicker() {
    stopPlaybackTicker();

    playbackInterval = setInterval(() => {
      if (!isPlaying || isSeeking || !currentSong) return;

      if (currentSeekSeconds < currentSong.duration) {
        currentSeekSeconds += 1;
        void saveSeekProgress();
        return;
      }

      stopPlaybackTicker();
      resetSeekUi();
      void next();
    }, 1000);
  }

  function resetSeekUi() {
    isProgrammaticSeekReset = true;
    currentSeekSeconds = 0;

    queueMicrotask(() => {
      isProgrammaticSeekReset = false;
    });
  }

  async function refreshLoadedSongs() {
    songs = await invoke<Song[]>("get_loaded_songs");
  }

  async function refreshCurrentSong() {
    currentSong = await invoke<Song | null>("get_current_song");
  }

  async function refreshSavedSeek() {
    try {
      currentSeekSeconds = await invoke<number>("get_current_song_seek");
    } catch (err) {
      console.error("Failed to get saved seek position:", err);
    }
  }

  async function syncPlaybackState() {
    try {
      const paused = await invoke<boolean>("get_is_player_paused");
      isPlaying = !paused;

      if (isPlaying) {
        startPlaybackTicker();
      } else {
        stopPlaybackTicker();
      }
    } catch (err) {
      console.error("Failed to sync playback state:", err);
    }
  }

  async function handleTrackChange(newIndex: number | null) {
    selectedIndex = newIndex;

    stopPlaybackTicker();
    resetSeekUi();

    await refreshCurrentSong();
    await refreshLoadedSongs();
    await refreshSavedSeek();
    await syncPlaybackState();

    if (currentSong) {
      selectedSongId = currentSong.id;

      const visibleSelectedIndex = songs.findIndex(
        (song) => song.id === currentSong?.id,
      );
      selectedIndex = visibleSelectedIndex >= 0 ? visibleSelectedIndex : null;
    } else if (newIndex !== null && songs[newIndex]) {
      selectedSongId = songs[newIndex].id;
    } else {
      selectedSongId = null;
      selectedIndex = null;
    }

    await ensureSelectedSongIsVisible();
  }

  async function playSelectedSong(index: number) {
    try {
      await invoke("play_song_at", { index });
    } catch (err) {
      console.error("Failed to play selected song:", err);
    }
  }

  async function play() {
    try {
      const nextIsPlaying = !isPlaying;

      await invoke("set_play_pause", { isPlaying: nextIsPlaying });
      isPlaying = nextIsPlaying;

      if (isPlaying) {
        startPlaybackTicker();
      } else {
        stopPlaybackTicker();
        await saveSeekProgress();
      }
    } catch (err) {
      console.error("Failed to toggle play/pause:", err);
    }
  }

  async function previous() {
    try {
      await invoke("previous_song");
    } catch (err) {
      console.error("Failed to go to previous song:", err);
    }
  }

  async function next() {
    try {
      await invoke("next_song");
    } catch (err) {
      console.error("Failed to go to next song:", err);
    }
  }

  async function shuffle() {
    try {
      await invoke("set_random");
      isShuffle = !isShuffle;
    } catch (err) {
      console.error("Failed to toggle shuffle:", err);
    }
  }

  async function repeat() {
    try {
      await invoke("set_repeat");
      isRepeat = !isRepeat;
    } catch (err) {
      console.error("Failed to toggle repeat:", err);
    }
  }

  async function performSearch(autoplayFirst = false) {
    try {
      const count = await invoke<number>("search_songs", {
        query: searchQuery,
      });

      searchResultCount = count;
      lastSearchedQuery = searchQuery;
      await refreshLoadedSongs();

      if (selectedSongId !== null) {
        const visibleSelectedIndex = songs.findIndex(
          (song) => song.id === selectedSongId,
        );
        selectedIndex = visibleSelectedIndex >= 0 ? visibleSelectedIndex : null;
      } else {
        selectedIndex = null;
      }

      await ensureSelectedSongIsVisible();

      if (autoplayFirst && count > 0) {
        await playSelectedSong(0);
      }
    } catch (err) {
      console.error("Failed to search songs:", err);
    }
  }

  function handleSearchKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();

      if (searchTimeout) {
        clearTimeout(searchTimeout);
        searchTimeout = undefined;
      }

      void performSearch(true);
    }
  }

  function changeVolume(event: Event) {
    const value = Number((event.target as HTMLInputElement).value);
    volume = value;

    if (value === 0) {
      isMuted = true;
    } else {
      isMuted = false;
      previousVolume = value;
    }

    if (volumeTimeout) clearTimeout(volumeTimeout);

    volumeTimeout = setTimeout(async () => {
      try {
        await invoke("set_volume", { volume: value / 100 });
      } catch (err) {
        console.error("Failed to set volume:", err);
      }
    }, 50);
  }

  async function toggleMute() {
    try {
      if (isMuted || volume === 0) {
        volume = previousVolume || 50;
        isMuted = false;
      } else {
        previousVolume = volume || 50;
        volume = 0;
        isMuted = true;
      }

      await invoke("set_volume", { volume: volume / 100 });
    } catch (err) {
      console.error("Failed to toggle mute:", err);
    }
  }

  function onSeekInput() {
    if (isProgrammaticSeekReset || !currentSong) return;
    isSeeking = true;
    isDraggingSeek = true;
    stopPlaybackTicker();
  }

  async function commitSeek() {
    if (isProgrammaticSeekReset || !currentSong) {
      isSeeking = false;
      isDraggingSeek = false;
      return;
    }

    try {
      await invoke("set_current_song_seek", {
        seekValue: currentSeekSeconds,
      });

      await saveSeekProgress();
    } catch (err) {
      console.error("Failed to seek song:", err);
      resetSeekUi();
    } finally {
      isSeeking = false;
      isDraggingSeek = false;

      if (isPlaying) {
        startPlaybackTicker();
      }
    }
  }

  async function onSeekChange() {
    if (!isDraggingSeek) return;
    await commitSeek();
  }

  async function onSeekPointerUp() {
    if (!isDraggingSeek) return;
    await commitSeek();
  }

  $: if (hasInitialized && searchQuery !== lastSearchedQuery) {
    if (searchTimeout) clearTimeout(searchTimeout);

    searchTimeout = setTimeout(() => {
      void performSearch(false);
    }, 150);
  }

  onMount(() => {
    let unlisten: (() => void) | undefined;

    void (async () => {
      try {
        await invoke<number>("init");

        unlisten = await listen<TrackChangedPayload>(
          "track-changed",
          async (event) => {
            await handleTrackChange(event.payload.currentIndex);
          },
        );

        const savedSearch = await invoke<string>("get_saved_search_blob");
        searchQuery = savedSearch;

        const count = await invoke<number>("load");
        searchResultCount = count;
        lastSearchedQuery = searchQuery;

        const savedVolume = await invoke<number>("get_volume");
        const savedShuffle = await invoke<boolean>("get_random");
        const savedRepeat = await invoke<boolean>("get_repeat");
        const initialIndex = await invoke<number | null>("get_current_index");
        const savedIsPlaying = await invoke<boolean>("get_play_pause");

        volume = Math.round(savedVolume * 100);
        previousVolume = volume > 0 ? volume : 50;
        isMuted = volume === 0;

        isShuffle = savedShuffle;
        isRepeat = savedRepeat;
        isPlaying = savedIsPlaying;

        await refreshLoadedSongs();
        await refreshCurrentSong();
        await refreshSavedSeek();
        await handleTrackChange(initialIndex);
        await syncPlaybackState();

        hasInitialized = true;
      } catch (err) {
        console.error("Failed to initialize player:", err);
      }
    })();

    return () => {
      if (volumeTimeout) clearTimeout(volumeTimeout);
      if (searchTimeout) clearTimeout(searchTimeout);
      if (playbackInterval) clearInterval(playbackInterval);
      if (unlisten) unlisten();
    };
  });
</script>

<div class="app-shell">
  <div class="main-panel">
    <div class="search-row">
      <div class="search" data-tauri-drag-region>
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search songs, artist, album..."
          on:keydown={handleSearchKeydown}
        />
        <button class="search-button" on:click={() => performSearch(true)}>
          <Search size={16} />
          <span>Search</span>
        </button>
      </div>
    </div>

    <div class="song-list">
      <div class="song-list-body">
        <div class="song-list-header">
          <div>#</div>
          <div>Title</div>
          <div>Artist</div>
          <div>Album</div>
          <div>Date</div>
          <div>Ext</div>
          <div class="header-duration">
            {searchResultCount}
            {searchResultCount === 1 ? " song" : " songs"}
          </div>
        </div>

        {#each songs as song, i (song.id)}
          <div
            bind:this={songRowElements[i]}
            class:selected={selectedSongId === song.id}
            class="song-row"
            role="button"
            tabindex="0"
            title={`Play ${song.title}`}
            on:click={() => playSelectedSong(i)}
            on:keydown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                void playSelectedSong(i);
              }
            }}
          >
            <div class="index">{i + 1}</div>

            <div class="title-cell">
              <div class="song-title">{song.title}</div>
              {#if song.remix}
                <div class="song-subtitle">{song.remix}</div>
              {/if}
            </div>

            <div class="artist-cell">{song.artist}</div>
            <div class="album-cell">{song.album}</div>
            <div class="date-cell">{formatDate(song.release_year)}</div>
            <div class="extension-cell">{song.extension}</div>
            <div class="duration-cell">{formatDuration(song.duration)}</div>
          </div>
        {/each}
      </div>
    </div>
  </div>

  <div class="bottom-bar">
    <div class="bottom-layout">
      <div class="now-playing">
        {#if currentSong}
          <div class="now-playing-content">
            <div class="now-playing-header">
              <div class="now-playing-title">{currentSong.title}</div>
            </div>

            <div class="now-playing-meta">{currentSong.artist}</div>
            <div class="now-playing-album">{currentSong.album}</div>

            {#if currentSong.remix}
              <div class="now-playing-remix">{currentSong.remix}</div>
            {/if}
          </div>
        {:else}
          <div class="now-playing-content">
            <div class="now-playing-header">
              <div class="now-playing-title">Nothing playing</div>
            </div>
            <div class="now-playing-meta">Choose a song from the list</div>
          </div>
        {/if}
      </div>

      <div class="center-player">
        <div class="controls">
          <button
            on:click={shuffle}
            class:active={isShuffle}
            class="control-button secondary"
            title="Shuffle"
            aria-label="Shuffle"
          >
            <Shuffle size={18} strokeWidth={2.2} />
          </button>

          <button
            on:click={previous}
            class="control-button secondary"
            title="Previous"
            aria-label="Previous"
          >
            <SkipBack size={19} strokeWidth={2.2} />
          </button>

          <button
            class="control-button play"
            on:click={play}
            title={isPlaying ? "Pause" : "Play"}
            aria-label={isPlaying ? "Pause" : "Play"}
          >
            {#if isPlaying}
              <Pause size={20} strokeWidth={2.6} fill="currentColor" />
            {:else}
              <Play size={20} strokeWidth={2.6} fill="currentColor" />
            {/if}
          </button>

          <button
            on:click={next}
            class="control-button secondary"
            title="Next"
            aria-label="Next"
          >
            <SkipForward size={19} strokeWidth={2.2} />
          </button>

          <button
            on:click={repeat}
            class:active={isRepeat}
            class="control-button secondary"
            title="Repeat"
            aria-label="Repeat"
          >
            <Repeat size={18} strokeWidth={2.2} />
          </button>
        </div>

        <div class="seek-row">
          <span class="seek-time">{formatDuration(currentSeekSeconds)}</span>
          {#if currentSong}
            <input
              class="seek-slider"
              type="range"
              min="0"
              max={currentSong.duration}
              step="1"
              bind:value={currentSeekSeconds}
              on:input={onSeekInput}
              on:pointerup={onSeekPointerUp}
              aria-label="Seek"
            />
          {:else}
            <input
              class="seek-slider"
              type="range"
              min="0"
              max="0"
              value="0"
              disabled
              aria-label="Seek"
            />
          {/if}
          <span class="seek-time">
            {currentSong ? formatDuration(currentSong.duration) : "0:00"}
          </span>
        </div>
      </div>

      <div class="volume">
        <button
          class="volume-button"
          on:click={toggleMute}
          title="Mute"
          aria-label="Mute"
        >
          {#if isMuted || volume === 0}
            <VolumeX size={18} strokeWidth={2.2} />
          {:else if volume < 50}
            <Volume1 size={18} strokeWidth={2.2} />
          {:else}
            <Volume2 size={18} strokeWidth={2.2} />
          {/if}
        </button>

        <input
          type="range"
          min="0"
          max="100"
          bind:value={volume}
          on:input={changeVolume}
          aria-label="Volume"
        />

        <span>{volume}%</span>
      </div>
    </div>
  </div>
</div>

<style>
  :global(html, body) {
    margin: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: #121212;
    color: white;
    font-family: Arial, sans-serif;
  }

  :global(body) {
    overscroll-behavior: none;
  }

  :global(#app) {
    width: 100%;
    height: 100vh;
    overflow: hidden;
  }

  .app-shell {
    height: 100vh;
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    gap: 1rem;
    padding: 1rem;
    box-sizing: border-box;
    background: #121212;
    color: white;
    overflow: hidden;
  }

  .main-panel {
    min-height: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 0.75rem;
    overflow: hidden;
  }

  .search-row {
    width: 100%;
  }

  .search {
    width: 100%;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.75rem;
    align-items: center;
  }

  .search input {
    width: 100%;
    min-width: 0;
    padding: 0.85rem 1rem;
    border: 1px solid #444;
    border-radius: 999px;
    font-size: 1rem;
    outline: none;
    background: #1f1f1f;
    color: white;
    box-sizing: border-box;
  }

  .search-button {
    border: 1px solid #323232;
    border-radius: 999px;
    background: #1b1b1b;
    color: #f2f2f2;
    cursor: pointer;
    font-size: 0.95rem;
    font-weight: 600;
    padding: 0.8rem 1.1rem;
    width: auto;
    height: auto;
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    transition:
      background 0.18s ease,
      border-color 0.18s ease,
      transform 0.18s ease;
  }

  .search-button:hover {
    background: #242424;
    border-color: #454545;
  }

  .search-button:active {
    transform: scale(0.98);
  }

  .song-list {
    min-height: 0;
    width: 100%;
    background: #181818;
    border-radius: 12px;
    overflow: hidden;
  }

  .song-list-body {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .song-list-header,
  .song-row {
    display: grid;
    grid-template-columns:
      clamp(2.5rem, 4vw, 3.5rem)
      minmax(0, 2.4fr)
      minmax(0, 1.8fr)
      minmax(0, 1.8fr)
      clamp(4rem, 8vw, 6rem)
      clamp(3.5rem, 6vw, 5rem)
      clamp(3.5rem, 7vw, 4.5rem);
    gap: clamp(0.4rem, 1vw, 1rem);
    align-items: center;
    padding: 0.9rem 1rem;
    box-sizing: border-box;
    width: 100%;
  }

  .song-list-header {
    color: #b3b3b3;
    font-size: 0.9rem;
    border-bottom: 1px solid #2a2a2a;
    background: #181818;
    position: sticky;
    top: 0;
    z-index: 2;
  }

  .song-list-header > div,
  .song-row > div {
    min-width: 0;
  }

  .header-duration {
    text-align: right;
    white-space: nowrap;
  }

  .song-row {
    border-bottom: 1px solid #202020;
    background: transparent;
    color: white;
    cursor: pointer;
    outline: none;
  }

  .song-row:hover {
    background: #242424;
  }

  .song-row.selected {
    background: #1f3a2a;
  }

  .song-row:focus-visible {
    box-shadow: inset 0 0 0 2px #1db954;
  }

  .index,
  .duration-cell,
  .extension-cell,
  .date-cell {
    color: #b3b3b3;
    font-size: 0.95rem;
  }

  .duration-cell {
    text-align: right;
  }

  .title-cell,
  .album-cell,
  .artist-cell {
    min-width: 0;
  }

  .song-title,
  .album-cell,
  .artist-cell {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .song-title {
    font-weight: 600;
  }

  .song-subtitle {
    color: #b3b3b3;
    font-size: 0.85rem;
    margin-top: 0.2rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .bottom-bar {
    width: 100%;
    min-height: 130px;
    background: #181818;
    border-radius: 12px;
    padding: 1rem 1.25rem;
    box-sizing: border-box;
    overflow: hidden;
  }

  .bottom-layout {
    position: relative;
    min-height: 98px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .now-playing {
    min-width: 0;
    width: min(340px, 28vw);
    display: flex;
    align-items: center;
    overflow: hidden;
    z-index: 1;
  }

  .now-playing-content {
    width: 100%;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 0.35rem;
    overflow: hidden;
  }

  .now-playing-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    min-width: 0;
    overflow: hidden;
  }

  .now-playing-title {
    font-weight: 700;
    font-size: clamp(0.98rem, 1vw, 1.05rem);
    line-height: 1.3;
  }

  .now-playing-meta,
  .now-playing-album,
  .now-playing-remix {
    color: #b3b3b3;
    font-size: clamp(0.88rem, 0.9vw, 0.95rem);
    line-height: 1.3;
  }

  .now-playing-title,
  .now-playing-meta,
  .now-playing-album,
  .now-playing-remix {
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .center-player {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    width: min(42vw, 720px);
    min-width: 420px;
    display: grid;
    grid-template-rows: auto auto;
    justify-items: center;
    gap: 0.8rem;
    z-index: 2;
    pointer-events: none;
  }

  .center-player > * {
    pointer-events: auto;
  }

  .controls {
    display: flex;
    gap: 0.55rem;
    align-items: center;
    justify-content: center;
    flex-wrap: nowrap;
  }

  .control-button,
  .volume-button {
    appearance: none;
    border: none;
    background: transparent;
    color: #b3b3b3;
    cursor: pointer;
    display: grid;
    place-items: center;
    transition:
      color 0.16s ease,
      transform 0.16s ease,
      background-color 0.16s ease,
      opacity 0.16s ease;
  }

  .control-button.secondary {
    width: 36px;
    height: 36px;
  }

  .control-button.secondary:hover,
  .volume-button:hover {
    color: #ffffff;
    transform: scale(1.06);
  }

  .control-button.secondary.active {
    color: #1db954;
  }

  .control-button.play {
    width: 50px;
    height: 50px;
    border-radius: 999px;
    background: #ffffff;
    color: #111111;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.24);
  }

  .control-button.play:hover {
    background: #f8f8f8;
    color: #000000;
    transform: scale(1.06);
  }

  .control-button:active,
  .volume-button:active {
    transform: scale(0.96);
  }

  .seek-row {
    width: 100%;
    max-width: 560px;
    display: grid;
    grid-template-columns: 52px minmax(0, 1fr) 52px;
    align-items: center;
    gap: 0.75rem;
  }

  .seek-time {
    color: #b3b3b3;
    font-size: 0.9rem;
    text-align: center;
    white-space: nowrap;
  }

  .seek-slider {
    width: 100%;
  }

  .volume {
    width: min(260px, 22vw);
    min-width: 180px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.5rem;
    align-self: end;
    padding-bottom: 2px;
    z-index: 1;
  }

  .volume-button {
    width: 36px;
    height: 36px;
    flex: 0 0 auto;
  }

  .volume input {
    width: clamp(90px, 10vw, 120px);
    min-width: 90px;
    max-width: 120px;
  }

  .volume span {
    color: #b3b3b3;
    font-size: 0.9rem;
    min-width: 42px;
    text-align: right;
    flex: 0 0 auto;
  }

  .now-playing {
    margin-right: max(1rem, 22vw);
  }

  .volume {
    margin-left: max(1rem, 22vw);
  }

  @media (max-width: 1180px) {
    .center-player {
      width: min(46vw, 660px);
      min-width: 380px;
    }

    .now-playing {
      width: min(300px, 26vw);
    }

    .volume {
      width: min(220px, 20vw);
      min-width: 170px;
    }
  }

  @media (max-width: 1100px) {
    .song-list-header,
    .song-row {
      grid-template-columns:
        clamp(2.25rem, 4vw, 3rem)
        minmax(0, 2.6fr)
        minmax(0, 1.8fr)
        minmax(0, 1.5fr)
        clamp(3.75rem, 7vw, 5rem)
        clamp(3rem, 5vw, 4rem)
        clamp(3.25rem, 6vw, 4rem);
    }
  }

  @media (max-width: 980px) {
    .bottom-bar {
      min-height: unset;
      height: auto;
    }

    .bottom-layout {
      position: static;
      display: grid;
      grid-template-columns: 1fr;
      gap: 1rem;
    }

    .center-player {
      position: static;
      left: auto;
      top: auto;
      transform: none;
      width: 100%;
      min-width: 0;
      max-width: 100%;
      order: 1;
      pointer-events: auto;
    }

    .now-playing {
      width: 100%;
      margin-right: 0;
      order: 2;
    }

    .volume {
      width: 100%;
      min-width: 0;
      margin-left: 0;
      justify-content: center;
      align-self: center;
      padding-bottom: 0;
      order: 3;
    }

    .seek-row {
      max-width: 100%;
    }
  }

  @media (max-width: 820px) {
    .song-list-header,
    .song-row {
      grid-template-columns:
        2.25rem
        minmax(0, 2.8fr)
        minmax(0, 1.8fr)
        minmax(0, 1.2fr)
        4.25rem
        3.5rem;
    }

    .song-list-header > :nth-child(6),
    .song-row > :nth-child(6) {
      display: none;
    }
  }

  @media (max-width: 640px) {
    .song-list-header,
    .song-row {
      grid-template-columns:
        2rem
        minmax(0, 2.8fr)
        minmax(0, 1.6fr)
        3.75rem
        3.5rem;
      padding: 0.8rem 0.75rem;
    }

    .song-list-header > :nth-child(4),
    .song-row > :nth-child(4) {
      display: none;
    }
  }
</style>
