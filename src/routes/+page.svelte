<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core"
    import { listen } from "@tauri-apps/api/event"

	import { PaneGroup, Pane, PaneResizer } from "paneforge";

    import { EditorView } from "@codemirror/view";
    import CodeMirror from "svelte-codemirror-editor";
    import ResizableTable from "./components/ResizableTable.svelte";
    import type { RequestEntry } from "./components/ResizableTable.svelte";

    let requests = $state([]);
    let curr_id = $state(0);
    let selected_id = $state(0);
    let selected_entry: RequestEntry = $state();

    let requests_rows = [
        {name: "ID", default_size: 2},
        {name: "Method", default_size: 4},
        {name: "Destination", default_size: 5},
        {name: "Path", default_size: 10},
        {name: "Query", default_size: 5},
        {name: "State", default_size: 5},
        {name: "Length", default_size: 3,},
        {name: "Status", default_size: 3}
    ];
    
    let requests_cols: RequestEntry[] = $state([]);

    listen<string>("request-intercepted", (event) => {
        let entry = parse_request(event.payload);
        requests_cols.push(entry);
        requests = [...requests, {id: curr_id, payload: event.payload}];
    });

    function parse_request(request: string) {
        curr_id += 1;

        let req = request.raw.split('\r\n');
        let req_line = req[0].split(" ");
        let method = req_line[0];
        let path = req_line[1];
        let host = req.find((line: string) => line.toLowerCase().startsWith("host")).split(" ")[1].split(":")[0];
        let user_agent = req.find((line: string) => line.toLowerCase().startsWith("user-agent")).split(" ")[1];
        let query = "";
        let state = "";
        let length = "";
        let status = "";

        let result: RequestEntry = {
            id: curr_id,
            method: method,
            path: path,
            destination: host,
            user_agent: user_agent,
            query: query,
            state: state,
            length: length,
            status: status,
        };

        return result;
    }

    let response_editor_text = $state("");
    let http_editor_text = $state("");

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

    $effect(() => {
        if (selected_id !== 0) {
            selected_entry = requests_cols.find((req) => req.id === selected_id);
            http_editor_text = requests.find((req) => req.id === selected_id).payload.raw;
        }
    });
</script>

<PaneGroup direction="vertical" class="w-full h-full p-2 pl-4">
    <Pane defaultSize={40} class="bg-[#2F323A] rounded flex flex-col">
        <div class="h-full w-full">
            <ResizableTable rows={requests_rows} cols={requests_cols} bind:selected={selected_id}/>
        </div>
        <div class="w-full h-0.5 bg-[#25272D]">
        </div>
    </Pane>
    <PaneResizer class="h-1 cursor-col-resize" />
    <Pane>
        <PaneGroup direction="horizontal" class="">
            <Pane class="bg-[#2F323A] rounded flex flex-col">
                <div class="text-md w-full h-12 flex flex-row pl-3 items-center justify-between pr-5">
                    <p>{selected_entry ? selected_entry.destination :  "" }</p>
                    <div class="flex flex-row gap-4">
                        <select name="request_display_type" id="" class="">
                            <option value="original">Original</option>
                        </select>
                        <button class="bg-[#25272D] p-1 rounded hover:cursor-pointer">
                            Forward →
                        </button>
                    </div>
                </div>
                <div class="h-0.75 w-full bg-[#25272D]">
                </div>
                <CodeMirror bind:value={http_editor_text} class="w-full h-full flex-1 text-md" {extensions}/>
                <div class="h-0.75 w-full bg-[#25272D]">
                </div>
                <div class="w-full h-10">
                </div>
            </Pane>
            <PaneResizer class="w-1 cursor-col-resize" />
            <Pane class="bg-[#2F323A] rounded flex flex-col">
                <div class="text-md w-full h-12 flex flex-row pl-3 items-center justify-between pr-5">
                    <p>Reponse</p>
                    <p>18112 bytes | 27ms</p>
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
    </Pane>
</PaneGroup>

<style>
    select {
        background-color: #2F323A;
    }

    select option {
        background-color: #2F323A;
    }
</style>