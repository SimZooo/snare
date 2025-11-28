<script lang="ts">
    import { Pane, PaneGroup, PaneResizer } from "paneforge";
    import { listen, emit } from "@tauri-apps/api/event"
    import { open } from "@tauri-apps/plugin-dialog";
    import { invoke } from "@tauri-apps/api/core";

    let discovered_dirs = $state([]);
    let wordlist_content = $state("");
    let scanning = $state(false);
    let host = $state("");
    let file_path = $state("");

    listen<object>("dir-scanning-finished", (event) => {
        scanning = false;
        console.log("Finished scanning")
    });

    async function browse_files() {
        file_path = await open({
            multiple: false,
            directory: false,
        });
    }

    function start_scan() {
        scanning = true;
        if (scanning) {
            /*
            if (file_path === "") {
                invoke("probe_dirs", {host: host, wordlist: wordlist_content, file: false});
            }
            */

            invoke("probe_dirs", {host: host, wordlist: file_path, isFile: true});
            console.log("Started scanning");
        }
    }

    listen<object>("dir-received", (event) => {
        discovered_dirs.push(event.payload);
    });
</script>

<div class="w-full h-full grid grid-rows-[4em_auto] pb-2 pr-2 pl-4">
    <div></div>
    <PaneGroup direction="horizontal" class="flex flex-col">
        <Pane class="bg-[#2F323A] rounded flex flex-col">
            <div class="text-md w-full h-12 flex flex-row pl-3 items-center justify-between pr-5">
                <p>Discovered Directories</p>
                <button class="bg-[#25272D] p-1 h-2/3 rounded hover:cursor-pointer" onclick={() => start_scan()}>
                    {#if scanning}
                        Scanning...
                    {:else}
                        Start Scan
                    {/if}
                </button>
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="w-full h-full flex-1 text-md p-5 min-h-0 overflow-auto">
                {#each discovered_dirs as dir}
                    <div class="w-full flex flex-row justify-between">
                        <p>{dir.url}</p>
                        <p>{dir.status}</p>
                    </div>
                {/each}
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="w-full h-10 text-md">
            </div>
        </Pane>
        <PaneResizer class="h-full w-1 cursor-col-resize bg-[#25272D]" />
        <Pane class="bg-[#2F323A] rounded flex flex-col">
            <div class="text-md w-full h-12 flex flex-row pl-3 items-center justify-between pr-5">
                <p>Prober Settings</p>
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="w-full h-full flex-1 flex flex-col text-md p-2">
                <h1 class="">Host</h1>
                <div class="flex flex-col pl-5 gap-2 pt-1 pb-1">
                    <input type="text" bind:value={host} class="w-1/3 border rounded">
                </div>
                <h1 class="">Wordlist</h1>
                <div class="flex flex-col pl-5 gap-2 pt-1">
                    <button class="cursor-pointer w-fit border rounded p-1" onclick={() => browse_files()}> Upload File </button>
                    <p>{file_path}</p>
                    <textarea name="" id="" class="border rounded w-1/3 h-70 min-h-0 resize-none overflow-auto" bind:value={wordlist_content}></textarea>
                </div>
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="w-full h-10 text-md">
            </div>
        </Pane>
    </PaneGroup>
</div>

<style>
    input[type="file"]::file-selector-button {
        background-color: #EDE9E7;
        color: #25272D;
        padding: 0.2em;
        border-radius: 2px;
    }
</style>