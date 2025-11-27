import { invoke } from "@tauri-apps/api/core";

export type RequestEntry = {
    id: number,
    uuid: string,
    method: string,
    destination: string,
    path: string,
    query: string,
    state: string,
    length: number,
    status: string,
    user_agent: string,
};

export type HttpReqRecv = {
    method: string
    path: string,
    host: string,
    headers: object,
    body: string,
    cookies: object
    id: string
};

export type HttpResRecv = {
    id: string,
    status: string,
    headers: object,
    body: string,
    cookies: object,
};

export type Request = {
    id: number,
    uuid: string,
    headers: object,
    path: string,
    method: string,
    body: string,
    raw: string,
    status: string,
    state: string,
    destination: string,
    length: number
};

export type Response = {
    uuid: string,
    id: number,
    headers: object,
    body: string,
    status: string
}

export function parse_request_from_payload(payload: HttpReqRecv, id: number): Request {
    return {
        id,
        uuid: payload.id,
        headers: payload.headers ?? {},
        path: payload.path ?? "",
        method: payload.method ?? "",
        body: payload.body ?? "",
        destination: payload.host ?? "",
        state: "Waiting",
        status: "",
        length: payload.body?.length ?? 0,
        raw: ""
    };
}

export function parse_response_from_payload(payload: HttpResRecv): Response {
    return {
        uuid: payload.id,
        id: 0,
        headers: payload.headers ?? {},
        body: payload.body ?? "",
        status: payload.status ?? ""
    };
}

export function parse_raw_request(raw_request: string) {
}

export function forward_request(current_request, text) {
    if (current_request) {
        let parsed = fix_whitespaces(text);
        invoke("send_request", {raw: parsed});
    }
}

export function fix_whitespaces(raw: string) {
    let normalized = raw.replace(/\r\n|\r/g, "\n");
    return normalized.replace(/\n/g, "\r\n");
}
export function construct_request_packet(request: Request): string {
    if (!request) return "";
    if (!request.headers) return "";

    let text = `${request.method} ${request.path} HTTP/1.1\r\n`;

    for (const header in request.headers) {
        text += header + ": " + request.headers[header] + "\r\n";
    }
    text += "\r\n";
    text += request.body;
    return text
}

export function construct_response_packet(response: Response): string {
    if (!response) return "";
    if (!response.headers) return "";

    let text = `HTTP/1.1 ${response.status}\r\n`;

    for (const header in response.headers) {
        text += header + ": " + response.headers[header] + "\r\n";
    }
    text += "\r\n";
    text += response.body;
    return text;
}