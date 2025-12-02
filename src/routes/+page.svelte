<script lang="ts">
    import { listen, emit } from "@tauri-apps/api/event"
    import { invoke } from "@tauri-apps/api/core";

    import { persisted, type Persisted } from 'svelte-persisted-store'
	import { PaneGroup, Pane, PaneResizer } from "paneforge";

    import { EditorView } from "@codemirror/view";
    import CodeMirror from "svelte-codemirror-editor";
    import ResizableTable from "./components/ResizableTable.svelte";
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { construct_request_packet, construct_response_packet, parse_request_from_payload, parse_response_from_payload, type HttpReqRecv, type HttpResRecv, type Request, type Response } from "$lib/network";
    import { filter_query } from "$lib/search";

    let requests: Persisted<Request[]> = persisted("requests", []);
    let responses: Persisted<Response[]> = persisted("responses", []);
    let pending_responses: Response[] = $state([]);
    let search = $state("");

    let selected_entry: Request = $state();
    let selected_res: Response = $state();

    let intercept_state = $state(false);

    let forward_requests = persisted("forwarded_requests", []);
    let forward_responses = persisted("forward_responses", []);
    let filtered_requests = $state($requests);

    onMount(() => {
        requests.update((reqs) =>
            reqs.map((req) => {
                let res = $responses.find((res) => res.uuid === req.uuid);
                return res
                    ? { ...req, status: res.status, state: "Complete" }
                    : req;
            })
        );
    });

    let requests_rows = [
        {name: "ID", default_size: 2},
        {name: "Method", default_size: 4},
        {name: "Destination", default_size: 5},
        {name: "Path", default_size: 10},
        {name: "", default_size: 5},
        {name: "State", default_size: 5},
        {name: "Length", default_size: 3,},
        {name: "Status", default_size: 3}
    ];
    
    listen<HttpReqRecv>("request-received", (event) => {
        let payload = event.payload;

        let request = parse_request_from_payload(payload);
        pending_responses = pending_responses.filter((res) => {
            if (res.uuid === request.uuid) {
                request.status = res.status;
                request.state = "Complete";
                $responses = [...$responses, res];
                return false;
            }
            return true;
        });

        requests.update((reqs) =>
            //if (reqs.some(r => r.uuid === request.uuid)) return reqs; 
            [...reqs, request]
        );
        if (search === "") {
            filter();
        }
    });

    listen<HttpResRecv>("response-received", (event) => {
        let res = parse_response_from_payload(event.payload);
        let req = $requests.find((req) => req.uuid === event.payload.id);

        if (!req) {
            pending_responses = [...pending_responses, res];
            return;
        }

        res.id = req.id;
        requests.update((reqs) => {
            const index = reqs.findIndex((r) => r.id === req.id);

            let new_reqs = [...reqs];
            new_reqs[index] = {
                ...new_reqs[index],
                status: res.status,
                state: "Complete"
            }
            return new_reqs;
        });

        responses.update((r) => [...r, res]);
        if (search === "") {
            filter();
        }
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
        if (!selected_entry || !selected_entry.uuid) {
            return;
        }

        http_editor_text = construct_request_packet(selected_entry);
        let res = $responses.find((res) => res.uuid === selected_entry.uuid);
        selected_res = res;
        response_editor_text = construct_response_packet(res);
    });

    function get_key(obj: object, key: string) {
        const lower_case_obj = Object.entries(obj).map(([k, value]) => [k.toLowerCase(), value]);
        let val = lower_case_obj.find(([k, v]) => k == key);
        return val ? val[1] : undefined;
    }
    
    function clear_all() {
        requests.update((_) => []);
        responses.update((_) => []);
        response_editor_text = "";
        http_editor_text = "";
        selected_res = undefined;
        selected_entry = undefined;
        forward_requests.update((_) => []);
        forward_responses.update((_) => []);
        filtered_requests = [];
    }

    function send_to_repeater() {
        if (!selected_entry) return;

        forward_requests.update((forw) => [...forw, {entry: selected_entry, raw: http_editor_text}]);
        goto("/repeater");
    }

    function filter() {
        filtered_requests = filter_query(search, {req: $requests}).req
    }
</script>

<div class="w-full h-full grid grid-rows-[4em_auto] pb-2 pr-2">
    <div class="pl-6 w-full h-full items-center align-middle flex justify-between">
        <div class="flex gap-2 items-center">
            <div class="flex flex-col border rounded p-0.5 text-gray-500 border-gray-500 w-fit h-fit">
                <input type="text" placeholder="E.g: req.method:GET;" class="" bind:value={search}>
            </div>
            <button class="border-2 rounded text-gray-500 p-0.5 w-20" onclick={() => filter()}>
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
                <ResizableTable rows={requests_rows} cols={filtered_requests} bind:selected={selected_entry}/>
            </div>
            <div class="w-full h-0.5 bg-[#25272D]">
            </div>
        </Pane>
        <PaneResizer class="h-1 cursor-col-resize" />
        <Pane>
            <PaneGroup direction="horizontal" class="">
                <Pane class="bg-[#2F323A] rounded flex flex-col">
                    <div class="text-md w-full h-12 flex flex-row pl-3 items-center justify-between pr-5">
                        <p>{selected_entry ? selected_entry.destination ?? "":  "" }</p>
                        <div class="flex flex-row gap-4">
                            <select name="request_display_type" id="" class="">
                                <option value="original">Original</option>
                            </select>
                            <button class="bg-[#25272D] p-1 rounded hover:cursor-pointer" onclick={() => send_to_repeater()}>
                                Send to Repeater
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
                        <p>Response</p>
                        <p>{selected_res && selected_res.headers ? (get_key(selected_res.headers, "content-length") ?? "0") : ("0") } bytes</p>
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