<script>
	import favicon from '$lib/assets/favicon.svg';
	import "../app.css";

	let pages = [
		{ name: "Intercept", path: "/", image: "./intercept.png" },
		{ name: "Repeater", path: "/repeater", image: "./forward.png" },
		{ name: "Tokenizer", path: "/tokenizer", image: "./proxy.png"  },
		{ name: "Prober", path: "/prober", image: "./proxy.png"  },
		//{ name: "Tamper", path: "/settings", image: "./proxy.png"  },
		//{ name: "History", path: "/settings", image: "./proxy.png"  }
	];

	let side_menu = $state(true);

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<main class="w-screen h-screen bg-[#25272D] flex flex-row">
	{#if side_menu}
	<div class="w-50 h-full flex flex-col">
		<div class="w-full h-[4em] items-center justify-center flex">
		</div>
		<div class="w-full h-full bg-[#2F323A] ms-2 mb-2 rounded flex flex-col justify-between mt-1">
			{#each pages as page}
				<a href="{page.path}" class="w-full h-fit gap-5 flex justify-center p-3 align-middle items-center">
					<!--<img src="{page.image}" alt="" class="w-6">-->
					<p>{page.name}</p>
				</a>
				<div class="h-0.75 w-full bg-[#25272D]">
				</div>
			{/each}
			<div class="w-full h-full flex items-end justify-center pb-4">
				<button class="text-center hover:cursor-pointer" onclick={() => side_menu = false}>&lt; Collapse</button>
			</div>
		</div>
	</div>
	{:else}
	<div class="w-20 h-full flex flex-col">
		<div class="w-full h-[4em] p-4">
		</div>
		<div class="w-20 h-full flex items-start bg-[#2F323A] rounded mb-2 ms-2 p-2 flex-col justify-end">
			<div class="gap-4 flex flex-col">
				{#each pages as page}
					<a href="{page.path}" class="w-full pl-4 gap-5 flex">
						<img src="{page.image}" alt="" class="w-6">
					</a>
				{/each}
			</div>

			<div class="w-full h-full flex items-end justify-center">
				<button class="w-full text-center hover:cursor-pointer pb-6" onclick={() => side_menu = true}>&gt;</button>
			</div>
		</div>
	</div>
	{/if}
	
	<div class="flex flex-col w-full h-full">
		<div>
		</div>
		<div class="w-full h-full">
			{@render children()}
		</div>
	</div>
</main>

<style>
	* {
		color: #EDE9E7;	
		font-family: 'JetBrains Mono';
	}
</style>