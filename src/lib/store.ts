import { writable, type Writable } from "svelte/store";
import { type Request, type Response } from "$lib/network";

export let responses: Writable<Response[]> = writable([]);
export let requests: Writable<Request[]> = writable([]);
export let forwarded_requests: Writable<Request[]> = writable([]);
export let forwarded_responses: Writable<Response[]> = writable([]);

export let scan_requests: Writable<Request[]> = writable([]);