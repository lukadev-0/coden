<script lang="ts">
  import favicon from "#lib/assets/favicon.svg";
  import type { LayoutProps } from "./$types";

  import "./global.css";

  let { children }: LayoutProps = $props();
</script>

<svelte:head>
  <link rel="icon" href={favicon} />
</svelte:head>

<div class="root">
  <header>
    <h1>Coden</h1>
    <a href="https://github.com/lukadev-0/coden">GitHub</a>
  </header>
  {@render children()}
</div>

<style>
  .root {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  header {
    display: flex;
    flex-shrink: 0;
    align-items: center;
    height: 3rem;
    padding: 0 1rem;
    gap: 0.5rem;
  }

  h1 {
    font-weight: 600;
    text-box: trim-both cap alphabetic;
  }

  a {
    margin-left: auto;
    color: var(--color-on-base-dim);
    font-weight: 600;
    text-decoration: none;
  }
</style>
