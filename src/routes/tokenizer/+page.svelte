<script lang="ts">
    import { EditorView } from "@codemirror/view";
    import { Pane, PaneGroup, PaneResizer } from "paneforge";
    import CodeMirror from "svelte-codemirror-editor";
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";
    import { json } from "@codemirror/lang-json";

    let mode = $state(true);

    let raw_token = $state("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiYWRtaW4iOnRydWUsImlhdCI6MTUxNjIzOTAyMn0.KMUFsIDTnFmyG3nMiGM6H9FNFUROf3wh7SmqJp-QV30");
    let json_token = $state("");
    let header = $state("");
    let header_raw = $state("");
    let payload = $state("");
    let payload_raw = $state("");
    let secret = $state("a-string-secret-at-least-256-bits-long");
    let notes = $state([]);

    onMount(async () => {
        if (mode) {
            parse_jwt();
        } else {
            encode_jwt();
        }
    });

    async function parse_jwt() {
        try {
            let res = await invoke("parse_jwt_token", {rawToken: raw_token, secret: secret});
            header = JSON.parse(res.header);
            header_raw = JSON.stringify(header, null, 2);
            payload = JSON.parse(res.payload);
            payload_raw = JSON.stringify(payload, null, 2);
            notes = res.notes;
        } catch ({ name, message }) {
            console.log(name, message);
        }
    }

    async function encode_jwt() {
        let res = await invoke("encode_jwt", {header: header_raw, payload: payload_raw, secret: secret});
        raw_token = `${res.header}.${res.payload}.${res.signature}`;
        console.log(res);
    }

    let note_colors = {
        "info": "062A16",
        "warning": "292406",
        "error": "46110E",
    }

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

    const json_extensions = [editor_theme, EditorView.lineWrapping, json()];
    const extensions = [editor_theme, EditorView.lineWrapping];

    function get_note_color(importance: string) {
        return note_colors[importance] ?? "";
    }
</script>


<div class="w-full h-full grid grid-rows-[4em_auto] pb-2 pr-2 pl-4">
    <div class="w-full h-full flex items-center">
        <button
            class="flex flex-row rounded place-items-center border-2 p-0 relative overflow-hidden w-fit h-fit"
            onclick={() => {mode= !mode; mode ? parse_jwt() : encode_jwt()}}
        >
            <div class="relative flex w-40 h-7">
                <span id="" class="w-20 m-0 p-0 h-full flex items-center justify-center">Decode</span>
                <span id="" class="w-20 m-0 p-0 h-full flex items-center justify-center">Encode</span>
                <span
                    class="w-20 m-0 p-0 h-7 absolute bg-[#B6BDBD] transition"
                    style={mode ? "transform:translateX(100%)" : ""}
                ></span>
            </div>
        </button>
    </div>

    <PaneGroup direction="horizontal" class="">
        {#if mode}
            <Pane class="bg-[#2F323A] rounded flex flex-col">
                <div class="text-md w-full h-12 flex flex-row pl-3 items-center justify-between pr-5">
                    <p>Raw Token</p>
                </div>
                {#if notes.length === 0}
                    <div class="h-0.75 w-full bg-[#25272D]">
                    </div>
                {:else}
                    <div class="w-full bg-[#25272D] text-sm">
                        {#each notes as note}
                            <div style="background-color: #{get_note_color(note.importance)}">
                                {note.note}
                            </div>
                        {/each}
                    </div>
                {/if}
                <div class="w-full h-full flex-1 text-md">
                    <CodeMirror bind:value={raw_token} class="w-full h-full flex-1 text-md" {extensions} onchange={() => parse_jwt()}/>
                </div>
                <div class="h-0.75 w-full bg-[#25272D]">
                </div>
                <div class="w-full h-10">
                </div>
            </Pane>
            <PaneResizer class="w-1 cursor-col-resize" />
            <Pane class="bg-[#2F323A] rounded grid grid-rows-3">
                <div class="w-full h-full flex text-md flex-col">
                    <div class="text-md w-full h-12 flex flex-row pl-3 items-center justify-between pr-5" >
                        <p> JWT Header </p>
                    </div>
                    <div class="h-0.75 w-full bg-[#25272D]">
                    </div>
                    <CodeMirror bind:value={header_raw} class="w-full h-full flex-1 text-md" extensions={json_extensions} editable={false}/>
                    <div class="h-0.75 w-full bg-[#25272D]">
                    </div>

                </div>
                <div class="w-full h-full flex text-md flex-col">
                    <div class="text-md w-full h-12 flex flex-row pl-3 items-center justify-between pr-5" >
                        <p> JWT Payload </p>
                    </div>
                    <div class="h-0.75 w-full bg-[#25272D]">
                    </div>
                    <CodeMirror bind:value={payload_raw} class="w-full h-full flex-1 text-md" extensions={json_extensions} editable={false}/>
                    <div class="h-0.75 w-full bg-[#25272D]">
                    </div>
                </div>
                <div class="w-full">
                    <div class="w-full h-full flex flex-col">
                        <p class="p-3"> Secret </p>
                        <div class="h-0.75 w-full bg-[#25272D]">
                        </div>
                        <textarea bind:value={secret} class="h-full w-full items-start p-2" onchange={() => parse_jwt()}>
                        </textarea>
                        <div class="min-h-0.75 w-full bg-[#25272D]">
                        </div>
                        <div class="h-10 w-full max-h-10 min-h-10">
                        </div>
                    </div>
                </div>
            </Pane>
        {:else}
        <Pane>
            <PaneGroup direction="vertical">
                <Pane class="bg-[#2F323A] rounded flex flex-col">
                    <div class="text-md w-full h-12 flex flex-row pl-3 items-center justify-between pr-5">
                        <p>JWT Header</p>
                    </div>
                    <div class="h-0.75 w-full bg-[#25272D]">
                    </div>
                    <div class="w-full h-full flex-1 text-md">
                        <CodeMirror bind:value={header_raw} class="w-full h-full flex-1 text-md" extensions={json_extensions} onchange={() => encode_jwt()}/>
                    </div>
                </Pane>
                <PaneResizer class="w-full h-0.75 cursor-col-resize bg-[#25272d]" />
                <Pane class="bg-[#2F323A] rounded flex flex-col">
                    <div class="text-md w-full h-12 flex flex-row pl-3 items-center justify-between pr-5">
                        <p>JWT Payload </p>
                    </div>
                    <div class="h-0.75 w-full bg-[#25272D]">
                    </div>
                    <div class="w-full h-full flex-1 text-md">
                        <CodeMirror bind:value={payload_raw} class="w-full h-full flex-1 text-md" extensions={json_extensions} onchange={() => encode_jwt()}/>
                    </div>
                    <div class="h-0.75 w-full bg-[#25272D]">
                    </div>
                    <div class="w-full h-10">
                    </div>
                </Pane>
                <PaneResizer class="w-1 cursor-col-resize" />
            </PaneGroup>
        </Pane>
        <PaneResizer class="w-1 cursor-col-resize bg-[#25272d]" />
        <Pane>
            <PaneGroup direction="vertical">
                <Pane class="bg-[#2F323A] flex flex-col rounded">
                    <div class="text-md w-full h-12 flex flex-row pl-3 items-center justify-between pr-5">
                        <p> Raw JWT </p>
                    </div>
                    <div class="h-0.75 w-full bg-[#25272D]">
                    </div>
                    <div class="w-full h-full flex-1 text-md">
                        <CodeMirror bind:value={raw_token} class="w-full h-full flex-1 text-md" {extensions} onchange={() => encode_jwt()}/>
                    </div>
                </Pane>
                <PaneResizer class="bg-[#25272d] h-0.75 w-full cursor-col-resize" />
                <Pane class="bg-[#2F323A] w-full flex flex-col rounded">
                    <div class="text-md w-full h-12 flex flex-row pl-3 items-center justify-between pr-5">
                        <p> Secret </p>
                    </div>
                    <div class="h-0.75 w-full bg-[#25272D]"></div>
                    <textarea
                        bind:value={secret}
                        class="flex-1 w-full p-2 resize-none bg-[#2F323A] text-white outline-none"
                    ></textarea>
                    <div class="h-0.75 w-full bg-[#25272D]"></div>
                    <div class="w-full h-10"></div>
                </Pane>
            </PaneGroup>
        </Pane>
        {/if}
    </PaneGroup>
</div>

<style>
</style>