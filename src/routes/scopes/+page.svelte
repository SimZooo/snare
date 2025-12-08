<script lang="ts">
    import { check_request_scope, current_scope_index, scopes, type Scope } from "$lib/scope";
    import { Pane, PaneGroup, PaneResizer } from "paneforge";

    let current_scope: Scope = $state();
    let in_scope_text: string = $state();
    let out_of_scope_text: string = $state();

    function new_preset() {
        scopes.update((scopes) => {
            scopes.push({name: "New Preset", in_scope: [], out_of_scope: []});
            return scopes;
        });
    }

    function update_scope() {
        scopes.update((scopes) => {
            scopes[$current_scope_index] = {...current_scope};
            return scopes;
        });
        console.log($scopes, current_scope);
    }
    
    function delete_scope(index: number) {
        scopes.update((scopes) => {
            let new_scopes = [...scopes];
            new_scopes.splice(index, 1);
            return new_scopes;
        })
        if ($current_scope_index === index) {
            current_scope = undefined;
            $current_scope_index = undefined;
        }
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
                        Presets
                    </p>
                    <button class="w-35 h-9 p-1 bg-[#a1213f] hover:cursor-pointer rounded place-self-end mr-1" onclick={() => new_preset()}>
                        + New Preset
                    </button>
                </div>
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="h-full bg-[#25272D] m-1 rounded">
                {#each $scopes as scope, i}
                    <div class="w-full {$current_scope_index === i ? "selected" : ""} p-1 flex flex-row" >
                        <button class="text-left w-full" onclick={() => { $current_scope_index= i; current_scope = {...$scopes[i]}; in_scope_text = current_scope.in_scope.join("\n"); out_of_scope_text = current_scope.out_of_scope.join("\n"); }}>{scope.name}</button>
                        <!-- svelte-ignore a11y_consider_explicit_label -->
                        <button class="hover:cursor-pointer mr-1" onclick={() => delete_scope(i)}>
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
                <p class="pl-3">Scope Settings</p>
            </div>
            <div class="h-0.75 w-full bg-[#25272D]">
            </div>
            <div class="w-full h-full p-3 flex flex-col items-start gap-5">
                {#if current_scope}
                    <p>Name</p>
                    <input type="text" bind:value={current_scope.name} class="bg-[#25272D] p-1 rounded w-fit">
                    <div class="w-full grid grid-cols-2 grid-rows-[auto_1fr] gap-x-5 h-1/2">
                        <p> In-scope </p>
                        <p> Out-of-scope </p>
                        <textarea name="in_scope" id="" class="resize-none bg-[#25272D]" bind:value={in_scope_text} onchange={() => { current_scope.in_scope = in_scope_text.split("\n")}}></textarea>
                        <textarea name="out_of_scope" id="" class="resize-none bg-[#25272D]" bind:value={out_of_scope_text} onchange={() => { current_scope.out_of_scope = out_of_scope_text.split("\n")}}></textarea>
                    </div>
                    <button class="w-20 h-9 p-1 bg-[#a1213f] hover:cursor-pointer rounded mr-1" onclick={() => update_scope()}>
                        Save
                    </button>
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