<script lang="ts">
    type Row = {
        name: string,
        default_size: number,
    };

    export type RequestEntry = {
        id: number,
        method: string,
        destination: string,
        path: string,
        query: string,
        state: string,
        length: string,
        status: string,
        user_agent: string,
    };

    import { Pane, PaneGroup, PaneResizer } from "paneforge";
    let { rows, cols, selected = $bindable() }: { rows: Row[], cols: RequestEntry[], selected: number } = $props();

    let row_sizes = $state({});
</script>

<div class="w-full grid grid-rows-[3.75rem_auto_1fr]">
    <PaneGroup class="h-15 items-center justify-center" direction="horizontal" onLayoutChange={(e) => {
        e.forEach((size, i) => row_sizes[i] = size);
    }}>
        {#each rows as row, i}
            <Pane defaultSize={row.default_size} class="pl-4">
                <p>{row.name}</p>
            </Pane>
            {#if i < rows.length - 1}
                <PaneResizer class="w-0.5 h-full bg-[#25272D] cursor-col-resize" />
            {/if}
        {/each}
    </PaneGroup>

    <div class="w-full bg-[#25272D] h-0.5">
    </div>

    <div class="overflow-auto min-h-0">
        <PaneGroup direction="vertical" class="w-full bg-[#2F323A]">
        {#each cols as entry, row_i}
            <PaneGroup
            direction="horizontal"
            class="hover:bg-[#25272D]"
            >
            {#each rows as row, col_i}
                <Pane
                maxSize={row_sizes[col_i]}
                minSize={row_sizes[col_i]}
                class="pl-4 bg-[#{selected === entry.id ? "25272D" : ""}]"
                onclick={() => {if (selected === entry.id) { selected = 0 } else {selected = entry.id }}}
                >
                <p>{entry[row.name.toLowerCase()]}</p>
                </Pane>
            {/each}
            </PaneGroup>
        {/each}
        </PaneGroup>
    </div>
</div>