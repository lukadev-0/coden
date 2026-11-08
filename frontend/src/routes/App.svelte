<script lang="ts">
  import { basicSetup } from "codemirror";
  import { EditorView, keymap } from "@codemirror/view";
  import { editorTheme } from "./editor-theme";
  import { indentWithTab } from "@codemirror/commands";
  import { languages as cmLanguages } from "@codemirror/language-data";
  import { Compartment } from "@codemirror/state";
  import type { components } from "#lib/api-schema.ts";
  import { client } from "#lib/api-client.ts";

  type Props = {
    languages: components["schemas"]["Language"][];
  };

  const { languages }: Props = $props();
  let selectedLanguage = $derived(languages[0]);

  let languageCompartment = new Compartment();
  const editorView = new EditorView({
    extensions: [basicSetup, keymap.of([indentWithTab]), editorTheme, languageCompartment.of([])],
  });
  let editorParent: HTMLElement;

  let runPending = $state(false);
  let runResponse: components["schemas"]["RunResponse"] | null = $state(null);
  let runError: string | null = $state(null);

  $effect(() => {
    editorParent.appendChild(editorView.dom);
  });

  $effect(() => {
    const lang = cmLanguages.find(
      (l) => l.name.toLowerCase() == selectedLanguage.label.toLowerCase(),
    );

    if (!lang) {
      editorView.dispatch({
        effects: languageCompartment.reconfigure([]),
      });
      return;
    }

    const controller = new AbortController();
    lang.load().then((lang) => {
      editorView.dispatch({
        effects: languageCompartment.reconfigure(lang),
      });
    });

    return () => controller.abort();
  });

  const handleClickRun = async () => {
    const code = editorView.state.doc.toString();

    runPending = true;
    runError = null;
    runResponse = null;

    const res = await client.POST("/api/run", {
      body: {
        code,
        language: selectedLanguage.name,
      },
    });

    runPending = false;

    if (res.error !== undefined) {
      runError = res.error.message;
    } else if (res.data) {
      runResponse = res.data;
    }
  };

  const formatProgramStatus = (status: components["schemas"]["ExecResponseStatus"]) => {
    if (status === "other") return "";
    else if ("code" in status) return `code ${status.code}`;
    else if ("signal" in status) return `signal ${status.signal}`;
  };

  const formatProgramResponse = (name: string, response: components["schemas"]["ExecResponse"]) => {
    return (
      `--- ${name} (${response.time}ms, ${formatProgramStatus(response.status)}) ---\n` +
      response.output.trimEnd()
    );
  };
</script>

<main class="app-main">
  <div class="editor">
    <div>
      <div class="toolbar">
        <select
          class="language-select"
          name="language"
          aria-label="language"
          bind:value={
            () => selectedLanguage.name,
            (name) => (selectedLanguage = languages.find((language) => language.name == name)!)
          }
        >
          {#each languages as language}
            <option value={language.name}>{language.label}</option>
          {/each}
        </select>
      </div>
      <div class="editor-container" bind:this={editorParent}></div>
    </div>
  </div>
  <div class="output">
    <div>
      <div class="toolbar">
        <button class="run-button" onclick={handleClickRun} disabled={runPending}>
          <span>Run</span>
        </button>
        {#if runResponse !== null}
          {@const totalTime = (runResponse.compiler?.time ?? 0) + (runResponse.program?.time ?? 0)}
          <div class="time-took">
            Took {totalTime}ms.
          </div>
        {/if}
      </div>
      <output>
        {#if runPending}
          Running...
        {:else if runError !== null}
          <p>Error: {runError}</p>
        {:else if runResponse !== null}
          {@const compilerText = runResponse.compiler
            ? formatProgramResponse("COMPILER", runResponse.compiler) + "\n\n"
            : ""}
          {@const programText = runResponse.program
            ? formatProgramResponse("PROGRAM", runResponse.program)
            : ""}
          <pre>{compilerText}{programText}</pre>
        {/if}
      </output>
    </div>
  </div>
</main>

<style>
  .app-main {
    display: flex;
    flex-grow: 1;
    min-height: 0;
    padding: 1rem;
    padding-top: 0;
    gap: 1rem;

    --section-padding: 0.5rem;

    > * {
      display: flex;
      width: 100%;
      flex-direction: column;
      min-height: 0;
      min-width: 0;
      overflow: clip;
      padding: 1px;
      background-color: var(--color-surface);
      border-radius: calc(var(--border-radius) + var(--section-padding) + 1px);
      box-shadow: var(--shadow-inset-subtle), var(--shadow-sm);
      > * {
        border-radius: calc(var(--border-radius) + var(--section-padding));
        overflow-y: auto;
        min-height: 0;
        flex-grow: 1;
      }
    }
  }

  .toolbar {
    position: sticky;
    top: 0;
    left: 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: var(--section-padding);
    z-index: 2;

    &::before {
      content: "";
      position: absolute;
      inset: 0;
      background-color: var(--color-surface);
      mask-image: linear-gradient(to bottom, black, hsl(0deg 0% 0% / 0.6) 80%, transparent);
      z-index: -1;
    }
  }

  .editor > * {
    display: flex;
    flex-direction: column;
  }

  .editor-container {
    display: flex;
    flex-grow: 1;
    > :global(*) {
      flex-grow: 1;
    }
  }

  .run-button {
    display: inline-flex;
    align-items: center;
    padding: 0 0.5rem;
    height: 1.75rem;
    background-color: var(--color-accent);
    color: var(--color-on-accent);
    font-weight: 600;
    border-radius: var(--border-radius);
    box-shadow: var(--shadow-sm);

    > * {
      text-box: trim-both cap alphabetic;
    }

    &[disabled] {
      opacity: 0.5;
    }
  }

  output {
    display: block;
    padding: var(--section-padding);
    padding-top: 0;
  }

  .time-took {
    font-style: italic;
    font-size: var(--font-size-sm);
    color: var(--color-on-surface-dim);
    text-shadow: var(--shadow-sm);
  }
</style>
