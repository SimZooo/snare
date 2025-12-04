import { writable, type Writable } from "svelte/store";
import { type HttpReqRecv, type HttpResRecv, type Request, type Response } from "$lib/network";

export let responses: Writable<Response[]> = writable([]);
export let requests: Writable<Request[]> = writable([]);
export let forwarded_requests: Writable<Request[]> = writable([]);
export let forwarded_responses: Writable<Response[]> = writable([]);