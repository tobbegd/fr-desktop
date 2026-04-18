<script lang="ts">
  import { onMount, onDestroy, untrack } from "svelte";
  import { EditorView, keymap, placeholder as cmPlaceholder } from "@codemirror/view";
  import { EditorState } from "@codemirror/state";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { autocompletion, completionKeymap, type CompletionContext } from "@codemirror/autocomplete";
  import { sql, SQLite } from "@codemirror/lang-sql";
  import { oneDark } from "@codemirror/theme-one-dark";

  type Props = {
    value: string;
    schema: Record<string, string[]>;
    onchange: (val: string) => void;
    onrun: () => void;
  };

  let { value = $bindable(), schema, onchange, onrun }: Props = $props();

  let container: HTMLDivElement;
  let view: EditorView | null = null;

  function buildEditor(currentSchema: Record<string, string[]>) {
    view?.destroy();

    const tables = Object.keys(currentSchema);
    const placeholderText = tables.length
      ? `SELECT * FROM ${tables.join("|")} LIMIT 10`
      : "SELECT * FROM bolag LIMIT 10";

    const allColumns = [...new Set(Object.values(currentSchema).flat())].map((col) => ({
      label: col,
      type: "property",
    }));

    function columnFallback(ctx: CompletionContext) {
      const word = ctx.matchBefore(/\w*/);
      if (!word || (word.from === word.to && !ctx.explicit)) return null;
      return { from: word.from, options: allColumns };
    }

    const startState = EditorState.create({
      doc: value,
      extensions: [
        history(),
        keymap.of([
          { key: "Ctrl-Enter", run: () => { onrun(); return true; } },
          { key: "Mod-Enter", run: () => { onrun(); return true; } },
          ...completionKeymap,
          ...defaultKeymap,
          ...historyKeymap,
        ]),
        autocompletion(),
        sql({ dialect: SQLite, schema: currentSchema, upperCaseKeywords: true }),
        EditorState.languageData.of(() => [{ autocomplete: columnFallback }]),
        oneDark,
        EditorView.theme({
          "&": { borderRadius: "0.5rem", fontSize: "0.875rem" },
          ".cm-editor": { borderRadius: "0.5rem" },
          ".cm-scroller": { fontFamily: "monospace", borderRadius: "0.5rem" },
          ".cm-content": { padding: "8px 4px", minHeight: "72px" },
          "&.cm-focused": { outline: "none" },
          ".cm-focused .cm-cursor": { borderLeftColor: "#fff" },
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            const newVal = update.state.doc.toString();
            value = newVal;
            onchange(newVal);
          }
        }),
        cmPlaceholder(placeholderText),
      ],
    });

    view = new EditorView({ state: startState, parent: container });
  }

  onMount(() => buildEditor(schema));
  onDestroy(() => view?.destroy());

  $effect(() => {
    if (Object.keys(schema).length > 0) untrack(() => buildEditor(schema));
  });

  $effect(() => {
    if (view && value !== view.state.doc.toString()) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: value },
      });
    }
  });
</script>

<div bind:this={container} class="rounded-lg overflow-hidden border border-zinc-700 focus-within:border-zinc-500 transition-colors"></div>
