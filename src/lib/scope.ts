import { get, writable, type Writable } from "svelte/store";
import type { Request } from "./network";

export type Scope = {
    name: string,
    in_scope: string[],
    out_of_scope: string[],
};

export let scopes: Writable<Scope[]> = writable([]);
export let current_scope_index: Writable<number> = writable();

export function check_request_scope(request: Request): boolean {
    let host = request.headers.host ?? request.headers.Host;
    const current_scope = get(scopes)[get(current_scope_index)];
    if (!current_scope) {
        return true;
    }
    if (!current_scope.in_scope.find((scope) => scope === host) || current_scope.out_of_scope.find((scope) => scope === host)) {
        return false;
    }

    return true;
}