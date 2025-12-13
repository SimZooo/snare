import { writable, type Writable } from "svelte/store";
import { type Request, type Response } from "$lib/network";
import type { Script } from "./script";
import { persisted, type Persisted } from "svelte-persisted-store";

export let responses: Writable<Response[]> = writable([]);
export let requests: Writable<Request[]> = writable([]);
export let forwarded_requests: Writable<Request[]> = writable([]);
export let forwarded_responses: Writable<Response[]> = writable([]);

export let scan_requests: Writable<Request[]> = writable([]);
export let scripts: Writable<Script[]> = writable([]);
export let current_script_index: Writable<number> = writable(0);