<script lang="ts">
  import App from "./App.svelte";
  import { client } from "#lib/api-client.ts";
  import { browser } from "$app/env";

  const browserPromise = new Promise<void>((resolve) => {
    if (browser) {
      resolve();
    }
  });

  const languages = browserPromise
    .then(() => client.GET("/api/languages"))
    .then((res) => {
      if ("error" in res) {
        throw new Error(res.error);
      }
      return res.data;
    });
</script>

{#await languages}
  Loading...
{:then languages}
  <App {languages} />
{:catch error}
  <p>Something went wrong: {error.message}</p>
{/await}
