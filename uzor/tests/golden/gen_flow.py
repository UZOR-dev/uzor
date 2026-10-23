"""Regenerate flow.json: render each case of flow_cases.json in headless Chrome and record
every box's getBoundingClientRect (pre-order). usage: python gen_flow.py (needs google-chrome
and the `websockets` package). Boxes are border-box, like uzor::ui::flow."""
import asyncio, json, os, subprocess, tempfile, time, urllib.request
import websockets

HERE = os.path.dirname(os.path.abspath(__file__))

def html(node):
    kids = "".join(html(c) for c in node.get("children", []))
    return f'<div style="{node.get("style", "")}">{kids}</div>'

async def measure(cases):
    prof = tempfile.mkdtemp()
    chrome = subprocess.Popen(["google-chrome", "--headless=new", "--disable-gpu", "--no-sandbox", "--remote-debugging-port=9335",
        f"--user-data-dir={prof}", "about:blank"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(100):
            try:
                tabs = json.load(urllib.request.urlopen("http://127.0.0.1:9335/json")); break
            except Exception:
                time.sleep(0.2)
        ws_url = [t for t in tabs if t["type"] == "page"][0]["webSocketDebuggerUrl"]
        async with websockets.connect(ws_url, max_size=50_000_000) as ws:
            n = [0]
            async def cmd(m, **p):
                n[0] += 1; i = n[0]; await ws.send(json.dumps({"id": i, "method": m, "params": p}))
                while True:
                    r = json.loads(await ws.recv())
                    if r.get("id") == i: return r.get("result", {})
            for c in cases:
                await cmd("Emulation.setDeviceMetricsOverride", width=c["viewport"], height=800, deviceScaleFactor=1, mobile=False)
                page = ("<!doctype html><style>*{box-sizing:border-box;margin:0;border:0 solid}body{margin:0}</style>"
                        f'<body><div id="root-wrap">{html(c["tree"])}</div></body>')
                await cmd("Page.navigate", url="data:text/html;charset=utf-8," + urllib.parse.quote(page))
                await asyncio.sleep(0.5)
                js = "JSON.stringify([...document.querySelectorAll('#root-wrap div')].map(e=>{const r=e.getBoundingClientRect();return [r.x,r.y,r.width,r.height]}))"
                r = await cmd("Runtime.evaluate", expression=js, returnByValue=True)
                c["chrome"] = json.loads(r["result"]["value"])
    finally:
        chrome.kill()
    return cases

import urllib.parse
cases = json.load(open(os.path.join(HERE, "flow_cases.json")))
out = asyncio.run(measure(cases))
json.dump(out, open(os.path.join(HERE, "flow.json"), "w"), indent=1)
print("wrote", len(out), "cases")
