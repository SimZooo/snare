import { writable } from "svelte/store";

export type Script = {
    name: string,
    path: string,
    args: [],
    enabled: boolean,
    argvals: {},
    description: string
}

export type Schema = {
    name: string,
    description: string,
    args: [],
}