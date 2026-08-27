<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { buttonClass, inputClass, labelClass } from "./styles";
  import { directoryPlaceholder } from "$lib/platform";

  let {
    value = $bindable(""),
    label = "Directory",
    required = false,
    onerror,
    onPicked,
  }: {
    value?: string;
    label?: string;
    required?: boolean;
    onerror?: (message: string) => void;
    // Fired after the user picks a directory via the Browse dialog (not
    // when `value` changes by typing). Lets a caller react specifically to
    // a fresh pick, e.g. to suggest other fields from it.
    onPicked?: (directory: string) => void;
  } = $props();

  async function pickDirectory() {
    try {
      const dir = await open({ directory: true });
      if (typeof dir === "string") {
        value = dir;
        onPicked?.(dir);
      }
    } catch (err) {
      onerror?.((err as Error).message);
    }
  }
</script>

<label class={labelClass}>
  {label}
  <div class="flex gap-2">
    <input
      bind:value
      {required}
      placeholder={directoryPlaceholder()}
      class={`flex-1 ${inputClass}`}
    />
    <button type="button" onclick={pickDirectory} class={buttonClass}>Browse…</button>
  </div>
</label>
