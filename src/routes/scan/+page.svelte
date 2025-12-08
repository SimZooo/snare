<script lang="ts">
    import { Pane, PaneGroup, PaneResizer } from "paneforge";
    import { EditorView } from "@codemirror/view";
    import CodeMirror from "svelte-codemirror-editor";
    import { scan_requests } from "$lib/store";
    import { construct_response_packet } from "$lib/network";
    import { onMount, tick } from "svelte";

    let http_editor_text = $state("");
    let current = $state();

    function set_request(id: number) {
        current = {index: id, request: $scan_requests[id]};
        http_editor_text = current.request.raw;
    }

    function get_token() {
        const re = /^Authorization:\s*Bearer\s+(.+)$/m;
        let matches = http_editor_text.match(re);

        if (matches) {
            console.log(matches[0])
        }
    }

    onMount(async () => {
        if ($scan_requests.length > 0) {
            await tick();
            set_request(0)
        }
    })

    const editor_theme = EditorView.theme({
        "&": { backgroundColor: "#2F323A", color: "#FFFFFF", height: "100%" },
        ".cm-content": {
            fontFamily: "JetBrains Mono, monospace",
            caretColor: "#FFCC00",
            fontSize: "0.8rem",
            lineHeight: "1.8"
        },
        ".cm-cursor, .cm-dropCursor": { borderLeft: "2px solid #FFCC00" },
        ".cm-selection": { backgroundColor: "#555555" },
        ".cm-gutters": { backgroundColor: "transparent", border: "none", padding: "0" },
        ".cm-lineNumbers": { padding: "0 4px 0 0", color: "#EDE9E7" },
        ".cm-activeLineGutter": { backgroundColor: "transparent" },
        ".cm-activeLine": { backgroundColor: "#3B3E46" }
    });

    const extensions = [editor_theme];
</script>

<div class="w-full h-full grid grid-rows-[4em_auto]">
    <div class="w-full h-full flex flex-col justify-center">
        <div class="flex flex-row gap-2">
            {#each $scan_requests as req, i}
                <button class="border rounded w-10 hover:cursor-pointer" onclick={() => set_request(i)}>
                    {i + 1}
                </button>
            {/each}
        </div>
    </div>
    <PaneGroup direction="horizontal" class="flex flex-col">
        <Pane class="bg-[#2F323A] rounded flex flex-col">
            <div class="text-md w-full h-12 flex flex-row pl-3 items-center justify-between pr-5">
                <p>Request</p>
                <button class="bg-[#25272D] p-1 h-2/3 rounded hover:cursor-pointer" onclick={() => get_token()}>
                    Start Scan
                </button>
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <CodeMirror bind:value={http_editor_text} class="w-full h-full flex-1 text-md min-h-0" {extensions}/>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="w-full h-10 text-md">
            </div>
        </Pane>
        <PaneResizer class="h-full w-1 cursor-col-resize bg-[#25272D]" />
        <Pane class="bg-[#2F323A] rounded flex flex-col">
            <div class="text-md w-full h-12 flex flex-row pl-3 items-center justify-between pr-5">
                <p>Vulnerabilities</p>
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="w-full h-full flex-1 flex flex-col text-md p-2">
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="w-full h-10 text-md">
            </div>
        </Pane>
    </PaneGroup>
</div>

<style>
</style>