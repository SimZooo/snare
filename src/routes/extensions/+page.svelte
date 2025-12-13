<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { Pane, PaneGroup, PaneResizer } from "paneforge";
    import { onMount } from "svelte";
    import { open } from "@tauri-apps/plugin-dialog";
    import { current_script_index, scripts } from "$lib/store";
    import type { Schema, Script } from "$lib/script";

    let saved = $state(false);
    let current_script: Script = $state();

    async function add_script() {
        let path = await browse_files();
        let schema: Schema = await invoke("get_args", { path });
        let argvals = 
            Object.entries(schema.args).map(([key, type]) => {
                switch(type) {
                    case "String":
                        return {[key]: "SimZooo"};
                    case "Number":
                        return {[key]: 0.0};
                    case "Integer":
                        return {[key]: 0};
                    case "Boolean":
                        return {[key]: false};
                }
            });
        let script: Script = {
            name: schema.name,
            path,
            args: schema.args,
            enabled: false,
            argvals,
            description: schema.description
        };

        scripts.update((sc) => {
            let new_sc = sc;
            new_sc.push(script);
            return new_sc;
        });

        let res = await invoke("add_script", { "path": path, "args": script.argvals });
        // TODO: give feedback if add_script fails
    }

    function remove_script(index: number) {
        let res = invoke("remove_script", { "name": $scripts[index].name});

        scripts.update((scopes) => {
            let new_scopes = [...scopes];
            new_scopes.splice(index, 1);
            return new_scopes;
        })
        if ($current_script_index === index) {
            current_script = undefined;
            $current_script_index = undefined;
        }
    }

    async function browse_files() {
        let path = await open({
            multiple: false,
            directory: false,
        });
        return path;
    }

    function update_script(index: number) {
        let script = $scripts[index];
        let res = invoke("update_script", { name: script.name, args: script.argvals, enabled: script.enabled });
        $scripts[index] = script;
        saved = true;
    }

    async function save_script() {
        let i = $scripts.findIndex((scr) => scr.name === current_script.name);
        console.log(current_script, i);
        update_script(i);
    }

    function set_arg_value(argvals, key, newValue) {
        let index = argvals.findIndex(item => item.hasOwnProperty(key));
        console.log(index);
        if (index !== -1) {
            argvals[index] = {[key]: newValue};
        } else {
            argvals.push({[key]: newValue});
        }
        return argvals;
    }
</script>
<div class="w-full h-full grid grid-rows-[4em_auto]">
    <div>
    </div>
    <PaneGroup direction="horizontal" class="w-full h-full">
        <Pane class="bg-[#2F323A] rounded flex flex-col w-full h-full">
            <div class="w-full h-12 flex justify-center flex-col">
                <div class="justify-between flex">
                    <p class="w-fit flex flex-col justify-center pl-3">
                        Extensions
                    </p>
                    <button class="w-40 h-9 p-1 bg-[#a1213f] hover:cursor-pointer rounded place-self-end mr-1" onclick={() => add_script()}>
                        + Add Extension
                    </button>
                </div>
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="h-full bg-[#25272D] m-1 rounded">
            {#each $scripts as script, i}
                <div class="w-full {$current_script_index === i ? "selected" : ""} p-1 flex flex-row">
                    <div class="h-full flex p-1 w-6">
                        <input type="checkbox" name="enabled" id="" class="" bind:checked={script.enabled} onchange={async () => update_script(i)}>
                    </div>
                    <button class="text-left w-full h-full flex items-center" onclick={() => { $current_script_index= i; current_script = {...$scripts[i]}; saved = false }}>
                        {script.name}
                    </button>
                    <!-- svelte-ignore a11y_consider_explicit_label -->
                    <button class="hover:cursor-pointer mr-1" onclick={() => remove_script(i)}>
                        <svg class="w-5 text-red-400 fill-[#25272D]" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                            <path d="M4 6H20L18.4199 20.2209C18.3074 21.2337 17.4512 22 16.4321 22H7.56786C6.54876 22 5.69264 21.2337 5.5801 20.2209L4 6Z"
                                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                            <path d="M7.34491 3.14716C7.67506 2.44685 8.37973 2 9.15396 2H14.846C15.6203 2 16.3249 2.44685 16.6551 3.14716L18 6H6L7.34491 3.14716Z"
                                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                            <path d="M2 6H22" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                            <path d="M10 11V16" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                            <path d="M14 11V16" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                        </svg>
                    </button>
                </div>
            {/each}
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="w-full h-10">
            </div>
        </Pane>
        <PaneResizer class="w-1 cursor-col-resize" />
        <Pane class="bg-[#2F323A] rounded flex flex-col h-full w-full">
            <div class="w-full h-12 flex justify-center flex-col">
            {#if current_script}
                <p class="pl-3">{current_script.name}</p>
            {/if}
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="w-full h-full p-3 flex flex-col items-start gap-5">
            {#if current_script}
                <p class="text-lg underline"> Description </p>
                <p class="pl-4"> {current_script.description} </p>
                <p class="text-lg underline">Arguments</p>
                <div class="w-full flex flex-col gap-5 h-1/2">
                    {#each Object.entries(current_script.args) as [key, value], i}
                        <div class="flex flex-row pl-4">
                            <p>
                                {key}({value}):&nbsp
                            </p>
                            {#if value === "String"}
                                <input type="text" placeholder={value} class="bg-[#25272D] p-1 rounded outline-none" oninput={(e) => current_script.argvals = set_arg_value(current_script.argvals, key, e.target.value)} onkeypress={() => saved = false}>
                            {:else if value === "Number" || value == "Integer"}
                                <input type="number" placeholder="0" class="bg-[#25272D] p-1 rounded outline-none" bind:value={current_script.argvals[key]} onkeypress={() => saved = false}>
                            {:else if value === "Boolean"}
                                <input type="checkbox" placeholder="0" class="bg-[#25272D] p-1 rounded outline-none" bind:checked={current_script.argvals[key]} onkeypress={() => saved = false}>
                            {/if}
                        </div>
                        <button class="w-20 h-9 p-1 bg-[#a1213f] hover:cursor-pointer rounded mr-1" onclick={() => save_script()}>
                            {#if !saved}
                                Save
                            {:else}
                                Saved ✓
                            {/if}
                        </button>
                    {/each}
                </div>
            {/if}
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="w-full h-10">
            </div>
        </Pane>
    </PaneGroup>
</div>

<style>
    .selected {
        color: #DAA049;
    }
</style>