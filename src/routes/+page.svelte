<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let isPlaying = false;
  let isShuffle = false;
  let isRepeat = false;

  onMount(async () => {
    try {
      await invoke("load");
      console.log("Songs loaded");
    } catch (err) {
      console.error("Failed to load songs:", err);
    }
  });

  async function play() {
    isPlaying = !isPlaying;
    await invoke("play_pause");
  }

  async function previous() {
    await invoke("previous_song");
  }

  async function next() {
    await invoke("next_song");
  }

  async function shuffle() {
    isShuffle = !isShuffle;
    await invoke("set_random");
  }

  async function repeat() {
    isRepeat = !isRepeat;
    await invoke("set_repeat");
  }
</script>

<div class="container">
  <h1>Music Player</h1>

  <div class="controls">
    <button on:click={previous}>Previous</button>
    <button on:click={play}>{isPlaying ? "Pause" : "Play"}</button>
    <button on:click={next}>Next</button>
    <button on:click={shuffle} class:active={isShuffle}>Shuffle</button>
    <button on:click={repeat} class:active={isRepeat}>Repeat</button>
  </div>
</div>

<style>
  .container {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2rem;
    font-family: Arial, sans-serif;
  }

  .controls {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
    justify-content: center;
  }

  button {
    padding: 0.75rem 1.25rem;
    border: none;
    border-radius: 0.5rem;
    background: #222;
    color: white;
    cursor: pointer;
    font-size: 1rem;
  }

  button.active {
    background: #0a84ff;
  }
</style>
