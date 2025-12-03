<script lang="ts">
    import { open } from "@tauri-apps/plugin-dialog";
    import { Pane, PaneGroup, PaneResizer } from "paneforge";
    import { listen } from "@tauri-apps/api/event"
    import { invoke } from "@tauri-apps/api/core";
    import { writeTextFile } from "@tauri-apps/plugin-fs";
    import { tempDir, join } from "@tauri-apps/api/path";
    import { string } from "three/tsl";

    let mode = $state(false); // true: raw, false: guided
    let attack_types = ["form", "basic"];
    let attack_type = $state("");
    let file_paths = $state(["", ""]);
    let wordlists_content = $state(["", ""]);
    let scanning = $state(false);
    let url = $state("");
    let credentials = $state([["admin", "yellow", "https://10.10.10.10/end/point"]]);

    async function browse_files() {
        let path = await open({
            multiple: false,
            directory: false,
        });
        return path;
    }

    async function createTempWordlist(contents, name) {
        const temp_dir = await tempDir();
        const path = await join(temp_dir, name);

        await writeTextFile(path, contents);

        return path;
    }

    async function start_scan() {
        scanning = true;
        for (const idx in file_paths) {
            if (file_paths[idx] === "") {
                file_paths[idx] = await createTempWordlist(wordlists_content[idx], `wordlist${idx}.txt`);
            }
        }

        invoke("bruteforce", {filePaths: file_paths, attackType: attack_type, url: url})

        console.log("Started scanning");
    }

    listen<[string, string]>("bruteforce-responses", (event) => {
        scanning = false;
        credentials = [...credentials, event.payload];
        console.log(credentials);
    });
</script>

<div class="w-full h-full grid grid-rows-[4em_auto] pb-2 pr-2 pl-4">
    <div class="w-full h-full flex items-center">
        <button
            class="flex flex-row rounded place-items-center border-2 p-0 relative overflow-hidden w-fit h-fit"
            onclick={() => {mode= !mode}}
        >
            <div class="relative flex w-40 h-7">
                <span id="" class="w-20 m-0 p-0 h-full flex items-center justify-center">Raw</span>
                <span id="" class="w-20 m-0 p-0 h-full flex items-center justify-center">Guided</span>
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
            </Pane>
            <PaneResizer class="w-1 cursor-col-resize" />
            <Pane class="bg-[#2F323A] rounded grid grid-rows-3">
            </Pane>
        {:else}
        <Pane>
            <PaneGroup direction="vertical">
                <Pane class="bg-[#2F323A] rounded flex flex-col">
                    <div class="text-md w-full h-12 flex flex-row items-center justify-between p-5">
                        <p>Bruteforce Settings</p>
                        <button class="bg-[#25272D] rounded hover:cursor-pointer h-8 p-1" onclick={() => start_scan()}>
                            {#if scanning}
                                Bruteforcing...
                            {:else}
                                Start bruteforce
                            {/if}
                        </button>
                    </div>
                    <div class="h-0.75 w-full bg-[#25272D]">
                    </div>
                    <div class="p-2 flex flex-col gap-3 h-full">
                        <div>
                            <label for="url">Host URL:</label>
                            <input type="text" id="url" placeholder="https://example.com/directory" class="border rounded p-1 w-100" bind:value={url}>
                        </div>
                        <div>
                            <label for="url">Attack type:</label>
                            <select name="attack_type" class="border rounded p-1 w-20" bind:value={attack_type}>
                                {#each attack_types as type}
                                    <option value="{type}">
                                        {type}
                                    </option>
                                {/each}
                            </select>
                        </div>
                        <p>Wordlists:</p>
                        <div class="grid grid-cols-2 grid-rows-1 w-full">
                            <div class="flex flex-col gap-2 pt-1">
                                <input type="button" id="file" class="cursor-pointer w-fit border rounded p-1" onclick={async () => file_paths[0] = await browse_files()} value="Upload File">
                                <!--<p>{file_paths[0] ?? ""}</p>-->
                                <textarea name="" id="" class="border rounded w-1/2 h-70 min-h-0 resize-none overflow-auto" bind:value={wordlists_content[0]}></textarea>
                            </div>
                            <div class="flex flex-col gap-2 pt-1">
                                <input type="button" id="file" class="cursor-pointer w-fit border rounded p-1" onclick={async () => file_paths[1] = await browse_files()} value="Upload File">
                                <!--<p>{file_paths[1]}</p>-->
                                <textarea name="" id="" class="border rounded w-1/2 h-70 min-h-0 resize-none overflow-auto" bind:value={wordlists_content[1]}></textarea>
                            </div>
                        </div>
                    </div>
                </Pane>
                <PaneResizer class="w-1 cursor-col-resize" />
            </PaneGroup>
        </Pane>
        <PaneResizer class="w-1 cursor-col-resize bg-[#25272d]" />
        <Pane>
            <PaneGroup direction="vertical">
                <Pane class="bg-[#2F323A] flex flex-col rounded">
                    <div class="text-md w-full h-12 flex flex-row items-center justify-between p-5">
                        <p>Credentials</p>
                    </div>
                    <div class="h-0.75 w-full bg-[#25272D]">
                    </div>
                    <div class="w-full h-full p-2 flex flex-row">
                        {#each credentials as response, i}
                            {i+1}. 
                            Username:&nbsp;<p class="text-green-600"> {response[0]} </p>&nbsp;
                            Password:&nbsp; <p class="text-green-600"> {response[1]} </p>&nbsp;
                            Host:&nbsp; <p class="text-green-600"> {response[2]} </p>&nbsp;
                        {/each}
                    </div>
                </Pane>
                <PaneResizer class="bg-[#25272d] h-0.75 w-full cursor-col-resize" />
                <Pane class="bg-[#2F323A] flex flex-col rounded">
                    <div class="w-full h-full">
                    </div>
                </Pane>
            </PaneGroup>
        </Pane>
        {/if}
    </PaneGroup>
</div>

<style>

</style>