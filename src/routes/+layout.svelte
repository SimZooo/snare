<script>
	import favicon from '$lib/assets/favicon.svg';
	import { page } from '$app/state';
	import "../app.css";
    import { slide } from 'svelte/transition';

	let pages = $state([
		{ name: "Project", members: [
			{ name: "Scopes", path: "/scopes", image: "./proxy.png"  },
			{ name: "Extensions", path: "/extensions", image: "./proxy.png"  },
		],  expanded: true },
		{ name: "Proxy", members: [
			{ name: "Intercept", path: "/", image: "./intercept.png" },
			{ name: "Repeater", path: "/repeater", image: "./forward.png" },
		],  expanded: true },
		{ name: "Discover", members: [
			{ name: "Prober", path: "/prober", image: "./proxy.png"  },
			{ name: "Bruter", path: "/bruter", image: "./proxy.png"  },
		], expanded: true },
		{ name: "Vulnerability", members: [
			{ name: "Tokenizer", path: "/tokenizer", image: "./proxy.png"  },
			{ name: "Scan", path: "/scan", image: "./proxy.png" }
		], expanded: true }
	]);

	let side_menu = $state(true);

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<main class="w-screen h-screen bg-[#25272D] flex flex-row">
	<div class="w-50 h-full flex flex-col">
		<div class="w-full h-[4em] items-center justify-center flex">
		</div>
		<div class="w-full h-full bg-[#2F323A] ms-2 mb-2 rounded flex flex-col mt-1 text-sm gap-2 p-2">
			{#each pages as dir}
				<div class="flex flex-col text-gray-300">
					<button class="text-[#8A8C90] text-left hover:cursor-pointer flex flex-row gap-1" onclick={() => dir.expanded = !dir.expanded}>
						<p class="{ dir.expanded ? "rotate-90" : "-rotate-90"} transition duration-300">&gt;</p>
						{dir.name}
					</button>
					{#if dir.expanded}
						<div class="flex flex-col overflow-hidden" transition:slide={{ duration: 300 }}>
							{#each dir.members as p}
								<a href="{p.path}" style="color: {page.url.pathname === p.path ? "#DAA049" : "#EDE9E7"}" class="w-full h-fit p-1 hover:bg-gray-500 rounded">
									{p.name}
								</a>
							{/each}
						</div>
					{/if}
				</div>
			{/each}
			<div class="w-full h-full flex items-end justify-center pb-4">
				<button class="text-center hover:cursor-pointer" onclick={() => side_menu = false}>&lt; Collapse</button>
			</div>
		</div>
	</div>
	<div class="flex flex-col w-full h-full">
		<div>
		</div>
		<div class="w-full h-full pl-3 pr-2 pb-2">
			{@render children()}
		</div>
	</div>
</main>

<style>
	main {
		color: #EDE9E7;	
		font-family: 'JetBrains Mono';
	}
</style>