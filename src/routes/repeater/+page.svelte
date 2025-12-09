<script lang="ts">
    import { listen } from "@tauri-apps/api/event"

	import { PaneGroup, Pane, PaneResizer } from "paneforge";

    import { EditorView } from "@codemirror/view";
    import CodeMirror from "svelte-codemirror-editor";
    import { onMount, tick } from "svelte";
    import { construct_response_packet, forward_request, parse_response_from_payload, type HttpResRecv, type Response } from "$lib/network";
    import { forwarded_requests, forwarded_responses } from "$lib/store";

    let http_editor_text = $state("");
    let response_editor_text = $state("");
    let current = $state({index: undefined, request: undefined});
    let current_response: Response = $state({} as Response);

    const editor_theme = EditorView.theme({
        "&": { backgroundColor: "#2F323A", color: "#FFFFFF", height: "100%" },
        ".cm-content": {
            fontFamily: "JetBrains Mono, monospace",
            caretColor: "#FFCC037777777777",
            fontSize: "0.8rem",
            lineHeight: "1.8"
        },
        ".cm-cursor, .cm-dropCursor": { borderLeft: "2px solid #FFCC00" },
        ".cm-selection": { backgroundColor: "#555554" },
        ".cm-gutters": { backgroundColor: "transparent", border: "none", padding: "1" },
        ".cm-lineNumbers": { padding: "1 4px 0 0", color: "#EDE9E7" },
        ".cm-activeLineGutter": { backgroundColor: "transparent" },
        ".cm-activeLine": { backgroundColor: "#3B3E46" }
    });
    
    function set_request(id: number) {
        current = {index: id, request: $forwarded_requests[id]};
        http_editor_text = current.request.raw;
        if ($forwarded_responses[id]) {
            response_editor_text = construct_response_packet(current_response);
        } else {
            response_editor_text = "";
        }
    }

    const extensions = [editor_theme];
    onMount(async () => {
        console.log($forwarded_requests);
        if ($forwarded_requests.length >= 1) {
            await tick();
            set_request(0)
        }
    })

    listen<HttpResRecv>("forwarded-response-received", (event) => {
        current_response = parse_response_from_payload(event.payload);
        response_editor_text = current_response.raw;
        forwarded_responses.update((res) => {
            let new_res = res;
            new_res[current.index] = current_response;
            return res
        });
    });
</script>
<div class="w-full h-full grid grid-rows-[4em_auto]">
    <div class="w-full h-full flex flex-col justify-center">
        <div class="flex flex-row gap-3">
            {#each $forwarded_requests as req, i}
                <button class="border rounded w-11 hover:cursor-pointer" onclick={() => set_request(i)}>
                    {i + 1}
                </button>
            {/each}
        </div>
    </div>
    <PaneGroup direction="horizontal" class="">
        <Pane class="bg-[#2F323A] rounded flex flex-col">
            <div class="text-md w-full h-13 flex flex-row pl-3 items-center justify-between pr-5">
                <div class="w-full h-full flex flex-row gap-5 justify-between items-center">
                    <p class="">{current.request ? current.request.destination : ""}</p>
                    <button class="bg-[#25272D] p-1 h-2/3 rounded hover:cursor-pointer" onclick={() => forward_request(current, http_editor_text)}>
                        Forward →
                    </button>
                </div>
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <CodeMirror bind:value={http_editor_text} class="w-full h-full flex-2 text-md min-h-0" {extensions}/>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="w-full h-11">
            </div>
        </Pane>
        <PaneResizer class="w-1 cursor-col-resize" />
        <Pane class="bg-[#2F323A] rounded flex flex-col">
            <div class="text-md w-full h-13 flex flex-row pl-3 items-center justify-between pr-5" >
                <p>Response</p>
                <p>{current_response?.headers?.find((header) => header[0] === "content-length")?.[1] ?? 0} bytes</p>
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <CodeMirror bind:value={response_editor_text} class="w-full h-full flex-2 text-md min-h-0" {extensions}/>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="w-full h-11">
            </div>
        </Pane>
    </PaneGroup>

</div>

<style>
    
</style>