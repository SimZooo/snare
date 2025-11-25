import json
from mitmproxy import http, ctx
import uuid
def serialize_headers(headers):
    return {k: v for k, v in headers.items()}

def request(flow: http.HTTPFlow) -> None:
    req = flow.request
    obj = {
        "id": flow.id,
        "method": req.method,
        "path": req.path,
        "host": req.host,
        "headers": serialize_headers(req.headers),
        "body": (req.raw_content or b"").decode("utf-8", errors="replace"),
    }
    print(json.dumps(obj), flush=True)

def response(flow: http.HTTPFlow) -> None:
    res = flow.response

    obj = {
        "id": flow.id,
        "status": res.status_code,
        "headers": serialize_headers(res.headers),
        "body": (res.raw_content or b"").decode("utf-8", errors="replace"),
        "cookies": getattr(res.cookies, "fields", []),
    }

    print(json.dumps(obj), flush=True)