<script lang="ts">
    import { listen, emit } from "@tauri-apps/api/event"
    import { invoke } from "@tauri-apps/api/core";

    import { persisted } from 'svelte-persisted-store'
	import { PaneGroup, Pane, PaneResizer } from "paneforge";

    import { EditorView, keymap } from "@codemirror/view";
    import CodeMirror from "svelte-codemirror-editor";
    import ResizableTable from "./components/ResizableTable.svelte";
    import type { RequestEntry } from "./components/ResizableTable.svelte";
    import { select } from "three/tsl";
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";

    let requests = persisted("requests", []);
    let responses = persisted("responses", []);
    let pending_responses = $state([]);
    let selected_id = $state(0);
    let selected_entry: RequestEntry = $state();
    let selected_res = $state();
    let curr_id = $state(0);
    let intercept_state = $state(false);
    let forward_requests = persisted("forwarded_requests", []);

    onMount(() => {
        $requests.forEach((req) => {
            let entry = parse_request(req.packet);
            req.id = entry.id;
            let res = $responses.find((res) => res.id == req.packet.id)
            if (res) {
                entry.status = res.status;
                entry.state = "Complete"
            }
            requests_cols.push(entry);
        })
    });

    type HttpReqRecv = {
        method: string
        path: string,
        host: string,
        headers: object,
        body: string,
        cookies: object
        id: string
    };
    
    type HttpResRecv = {
        id: string,
        status: string,
        headers: object,
        body: string,
        cookies: object,
    };

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

    listen<HttpReqRecv>("request-received", (event) => {
        let payload = event.payload;

        let entry = parse_request(event.payload);
        requests_cols.push(entry);

        requests.update((reqs) => {reqs.push({id: entry.id, packet: payload}); return reqs});
        console.log($requests);

        pending_responses = pending_responses.filter((res) => {
            if (res.id === entry.uuid) {
                entry.status = res.status;
                entry.state = "Complete";
                $responses = [...$responses, res];
                return false;
            }
            return true;
        });
    });

    function parse_request(request: HttpReqRecv): RequestEntry {
        curr_id += 1;

        let result: RequestEntry = {
            id: curr_id,
            uuid: request.id,
            method: request.method,
            destination: request.host,
            path: request.path,
            query: "",
            state: "Waiting",
            length: request.body.length,
            status: status,
            user_agent: request.headers["user-agent"] ?? ""
        };

        return result;
    }

    listen<HttpResRecv>("response-received", (event) => {
        let req = $requests.find((req) => req.packet.id === event.payload.id);
        let req_col = requests_cols.find((req_col) => req_col.id === req.id);

        if (!req_col) {
            pending_responses = [...pending_responses, event.payload];
            return;
        }

        req_col.status = event.payload.status;
        req_col.state = "Complete"

        $responses = [...$responses, event.payload];
    });

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
            let req = $requests.find((req) => req.id === selected_id);
            http_editor_text = construct_request_packet(req);
            let res = $responses.find((res) => res.id === req.packet.id);
            selected_res = res;
            response_editor_text = construct_response_packet(res);
        }
    });

    function construct_request_packet(request): string {
        if (!request) { return "" };

        let text = `${request.packet.method} ${request.packet.path} HTTP/1.1\r\n`;

        for (const header in request.packet.headers) {
            text += header + ": " + request.packet.headers[header] + "\r\n";
        }
        text += "\r\n";
        text += request.packet.body;
        return text
    }

    function construct_response_packet(response): string {
        if (!response) { return "" };

        let text = `HTTP/1.1 ${response.status}\r\n`;

        for (const header in response.headers) {
            text += header + ": " + response.headers[header] + "\r\n";
        }
        text += "\r\n";
        text += response.body;
        return text;
    }

    function get_key(obj: object, key: string) {
        const lower_case_obj = Object.entries(obj).map(([k, value]) => [k.toLowerCase(), value]);
        let val = lower_case_obj.find(([k, v]) => k == key);
        return val ? val[1] : undefined;
    }
    
    function clear_all() {
        requests.update((req) => req = []);
        responses.update((res) => res = []);
        requests_cols = [];
        response_editor_text = "";
        http_editor_text = "";
        selected_res = "";
        selected_entry = {} as RequestEntry;
    }

    function send_to_forward() {
        if (!selected_entry) return;

        forward_requests.update((forw) => [...forw, {entry: selected_entry, raw: http_editor_text}]);
        goto("/forward");
    }
</script>

<div class="w-full h-full grid grid-rows-[4em_auto] pb-2 pr-2">
    <div class="pl-6 w-full h-full items-center align-middle flex justify-between">
        <div class="flex gap-2 items-center">
            <div class="flex flex-col border rounded p-0.5 text-gray-500 border-gray-500 w-fit h-fit">
                <input type="text" placeholder="Search requests" class="">
            </div>
            <button class="border-2 rounded text-gray-500 p-0.5 w-20">
                Search
            </button>
        </div>
        <div class="flex flex-row gap-3">
            <button class="border rounded p-1 hover:cursor-pointer" onclick={() => {clear_all()}}>
                Clear
            </button>
            <button class="flex flex-col border rounded p-1 mr-5 hover:cursor-pointer button_{intercept_state ? "enabled" : "disabled"}" onclick={() => {intercept_state = !intercept_state; invoke("toggle_intercept", {interceptToggle: intercept_state});}}>
                &gt;&gt;&nbsp;Intercept
            </button>
        </div>
    </div>

    <PaneGroup direction="vertical" class="w-full h-full pl-4">
        <Pane defaultSize={40} class="bg-[#2F323A] rounded flex flex-col">
            <div class="h-full w-full overflow-auto">
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
                            <button class="bg-[#25272D] p-1 rounded hover:cursor-pointer" onclick={() => send_to_forward()}>
                                Send to Forward
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
                        <p>{selected_res ? (get_key(selected_res.headers, "content-length") ?? "0") : ("0") } bytes</p>
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
</div>

<style>
    select {
        background-color: #2F323A;
    }

    select option {
        background-color: #2F323A;
    }

    .button_enabled {
        border: 1px solid oklch(72.3% 0.219 149.579);
        color: oklch(72.3% 0.219 149.579);
    }

    .button_disabled {
        border: 1px solid oklch(63.7% 0.237 25.331);
        color: oklch(63.7% 0.237 25.331);
    }
</style>