<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { buttonClass, inputClass, labelClass } from "./styles";

  let {
    value = $bindable(""),
    label = "Directory",
    required = false,
    onerror,
  }: {
    value?: string;
    label?: string;
    required?: boolean;
    onerror?: (message: string) => void;
  } = $props();

  async function pickDirectory() {
    try {
      const dir = await open({ directory: true });
      if (typeof dir === "string") value = dir;
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
      placeholder="C:\path\to\project"
      class={`flex-1 ${inputClass}`}
    />
    <button type="button" onclick={pickDirectory} class={buttonClass}>Browse…</button>
  </div>
</label>
