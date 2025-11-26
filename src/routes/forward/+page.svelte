<script lang="ts">
    import { listen, emit } from "@tauri-apps/api/event"
    import { invoke } from "@tauri-apps/api/core";

    import { persisted } from 'svelte-persisted-store'
	import { PaneGroup, Pane, PaneResizer } from "paneforge";

    import { EditorView, keymap } from "@codemirror/view";
    import CodeMirror from "svelte-codemirror-editor";
    import ResizableTable from "../components/ResizableTable.svelte";
    import { select } from "three/tsl";

    let requests = persisted("requests", []);
    let responses = persisted("responses", []);

    let http_editor_text = $state("");
    let response_editor_text = $state("");
    let current = $state();
    let forwarded_requests = persisted("forwarded_requests", []);
    let forward_responses = $state([]);
    let current_response = $state({});

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

    function get_text(id: number) {
        current = {index: id, request: $forwarded_requests[id]};
        http_editor_text = current.request.raw;
        if (forward_responses[id]) {
            response_editor_text = construct_response_packet(current_response);
        } else {
            response_editor_text = "";
        }
    }

    function forward_request() {
        if (current) {
            invoke("send_request", {raw: http_editor_text, id: current.request.entry.id});
        }
    }

    function construct_response_packet(response): string {
        if (!response) { return "" };

        let text = `HTTP/1.1 ${response.status}\r\n`;

        for (const header in response.headers) {
            text += header + ": " + response.headers[header] + "\r\n";
        }
        text += "\r\n";
        text += response.body ?? "";
        return text;
    }

    listen<object>("forwarded-request-received", (event) => {
        current_response = event.payload;
        response_editor_text = construct_response_packet(current_response);
        forward_responses[current.index] = current_response;
        forward_responses = forward_responses;
    });
</script>
<div class="pr-2 pl-4 pb-2 w-full h-full grid grid-rows-[4em_auto]">
    <div class="w-full h-full flex flex-col justify-center">
        <div class="">
            {#each $forwarded_requests as req, i}
                <button class="border rounded w-10 hover:cursor-pointer" onclick={() => get_text(i)}>
                    {i + 1}
                </button>
            {/each}
        </div>
    </div>
    <PaneGroup direction="horizontal" class="">
        <Pane class="bg-[#2F323A] rounded flex flex-col">
            <div class="text-md w-full h-12 flex flex-row pl-3 items-center justify-between pr-5">
                <div class="w-full h-full flex flex-row gap-4 justify-between items-center">
                    <p class="">{current ? current.request.entry.destination : ""}</p>
                    <button class="bg-[#25272D] p-1 h-2/3 rounded hover:cursor-pointer" onclick={() => forward_request()}>
                        Forward →
                    </button>
                </div>
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <CodeMirror bind:value={http_editor_text} class="w-full h-full flex-1 text-md min-h-0" {extensions}/>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="w-full h-10">
            </div>
        </Pane>
        <PaneResizer class="w-1 cursor-col-resize" />
        <Pane class="bg-[#2F323A] rounded flex flex-col">
            <div class="text-md w-full h-12 flex flex-row pl-3 items-center justify-between pr-5" >
                <p>Reponse</p>
                <p>0 bytes</p>
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <CodeMirror bind:value={response_editor_text} class="w-full h-full flex-1 text-md" {extensions}/>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="w-full h-10">
            </div>
        </Pane>
    </PaneGroup>

</div>

<style>
    
</style>