#!/usr/bin/env python3
"""Author Beargarden's skate clips in ActionForge (https://actionforge.app), driven over the
Chrome DevTools protocol, and download each as a GLB on the Mixamo skeleton.

ActionForge runs in a Chrome started with --remote-debugging-port=9333 on actionforge.app.
Each clip in skate_clips.json is made with ActionForge's New animation (from the rest pose),
posed key by key in Animate > Transform (WORLD rotations in degrees, the pelvis's location in
cm), then exported with Download (GLB, skeleton Mixamo, no character mesh).

    python3 tools/actionforge.py tools/skate_clips.json art/clips/   [clip ...]
"""
import base64, json, os, sys, time, urllib.request
import websocket

PORT = 9333


class Page:
    def __init__(self):
        tabs = json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json"))
        url = [t for t in tabs if t["type"] == "page"][0]["webSocketDebuggerUrl"]
        self.ws = websocket.create_connection(url, timeout=120, suppress_origin=True)
        self.n = 0

    def call(self, m, **p):
        self.n += 1
        self.ws.send(json.dumps({"id": self.n, "method": m, "params": p}))
        while True:
            r = json.loads(self.ws.recv())
            if r.get("id") == self.n:
                if "error" in r:
                    raise RuntimeError(r["error"])
                return r["result"]

    def js(self, src):
        r = self.call("Runtime.evaluate", expression=src, awaitPromise=True, returnByValue=True)
        if "exceptionDetails" in r:
            raise RuntimeError(r["exceptionDetails"])
        return r["result"].get("value")

    def click(self, x, y):
        for t in ("mouseMoved", "mousePressed", "mouseReleased"):
            self.call("Input.dispatchMouseEvent", type=t, x=x, y=y, button="left", clickCount=1)


HELPERS = r"""
window.af = (() => {
  const set = (el, v) => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(el, String(v));
    el.dispatchEvent(new Event('input', {bubbles: true}));
    el.dispatchEvent(new KeyboardEvent('keydown', {key: 'Enter', code: 'Enter', bubbles: true}));
    el.dispatchEvent(new Event('change', {bubbles: true})); el.blur(); };
  const leaf = t => [...document.querySelectorAll('*')].find(e => e.children.length == 0 && e.innerText && e.innerText.replace(/[\u25c6\u25c7]/g, '').trim() == t && e.getBoundingClientRect().x < 240 && e.offsetParent);
  const fields = pre => [...document.querySelectorAll('input[type=number]')].filter(i => i.offsetParent && (i.title || '').startsWith(pre));
  const button = t => [...document.querySelectorAll('button')].find(b => b.offsetParent && b.innerText.trim() == t);
  return {
    pick(name) { let e = leaf(name); if (!e) { const o = document.querySelector('.outliner, [class*=outliner]'); }
      e = leaf(name); if (!e) return 'no ' + name; e.scrollIntoView({block: 'center'}); e.click(); return 'ok'; },
    put(kind, v) { const f = fields(kind); if (f.length < 3) return 'no ' + kind; v.forEach((x, i) => { if (x !== null) set(f[i], x); }); return f.map(i => i.value); },
    frameAt() { const r = document.querySelector('[aria-label=Frame]').getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; },
    frameIn(n) { const i = document.querySelector('[aria-label=Frame] input'); if (i.hidden) return 'hidden'; set(i, n);
      return document.querySelector('[aria-label=Frame]').getAttribute('aria-valuenow'); },
    frame() { return document.querySelector('[aria-label=Frame]').getAttribute('aria-valuenow'); },
    newAnim(name, len) { button('NEW ANIMATION').click();
      const d = [...document.querySelectorAll('input')].filter(i => i.offsetParent);
      const nm = d.find(i => i.type == 'text' && i.value == 'My animation'); const ln = d.find(i => i.type == 'number' && i.closest('form'));
      const s = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
      s.call(nm, name); nm.dispatchEvent(new Event('input', {bubbles: true}));
      s.call(ln, String(len)); ln.dispatchEvent(new Event('input', {bubbles: true}));
      const rest = d.find(i => i.type == 'radio' && i.value == 'rest'); if (rest) rest.click();
      button('CREATE').click(); return 'made ' + name; },
    tab(t) { const b = button(t); if (b) b.click(); return !!b; },
    hook() { if (window.__hooked) return 'already'; window.__hooked = 1; window.__dl = [];
      const c = HTMLAnchorElement.prototype.click;
      HTMLAnchorElement.prototype.click = function () { if (this.download) { window.__dl.push({name: this.download, href: this.href}); return; } return c.call(this); };
      return 'hooked'; },
    exportSetup() { button('DOWNLOAD ▾').click();
      const sel = [...document.querySelectorAll('select')].filter(s => s.offsetParent);
      const pickv = (val) => { const s = sel.find(s => [...s.options].some(o => o.value == val)); s.value = val; s.dispatchEvent(new Event('change', {bubbles: true})); };
      pickv('mixamo'); pickv('glb');
      const cb = [...document.querySelectorAll('input[type=checkbox]')].find(c => c.offsetParent && c.parentElement.innerText.trim() == 'INCLUDE CHARACTER MESH');
      if (cb && cb.checked) cb.click();
      return [...document.querySelectorAll('select')].filter(s => s.offsetParent).map(s => s.value).concat([cb && cb.checked]); },
    download() { window.__dl = []; button('↓ DOWNLOAD').click(); return 'clicked'; },
    async fetched() { const out = [];
      for (const d of window.__dl || []) { const u = new Uint8Array(await (await fetch(d.href)).arrayBuffer()); let s = '';
        for (let i = 0; i < u.length; i += 32768) s += String.fromCharCode.apply(null, u.subarray(i, i + 32768));
        out.push({name: d.name, b64: btoa(s)}); }
      return out; },
  };
})(); 'ok'
"""


def author(page, clip, out_dir):
    name, length = clip["name"], clip["frames"]
    page.js(HELPERS)
    page.js("af.hook()")
    page.call("Input.dispatchKeyEvent", type="keyDown", key="Escape", code="Escape")
    page.call("Input.dispatchKeyEvent", type="keyUp", key="Escape", code="Escape")
    print(page.js(f"af.newAnim({json.dumps(name)}, {length})"))
    time.sleep(1.0)
    page.js("af.tab('Animate')")
    for key in clip["keys"]:
        f = key["frame"]
        x, y = page.js("af.frameAt()")
        page.click(x, y)
        time.sleep(0.15)
        got = page.js(f"af.frameIn({f})")
        page.call("Input.dispatchKeyEvent", type="keyDown", key="Escape", code="Escape")
        page.call("Input.dispatchKeyEvent", type="keyUp", key="Escape", code="Escape")
        if str(got) != str(f):
            raise RuntimeError(f"{name}: frame {f} -> {got}")
        for bone, spec in key["pose"].items():
            r = page.js(f"af.pick({json.dumps(bone)})")
            if r != "ok":
                raise RuntimeError(f"{name}: {r}")
            time.sleep(0.05)
            if "rot" in spec:
                page.js(f"af.put('Rotation', {json.dumps(spec['rot'])})")
            if "loc" in spec:
                page.js(f"af.put('Location', {json.dumps(spec['loc'])})")
        print(f"  {name}: frame {f}, {len(key['pose'])} bones")
    page.js("af.exportSetup()")
    time.sleep(0.5)
    page.js("af.download()")
    for _ in range(40):
        time.sleep(0.5)
        files = page.js("af.fetched()")
        if files:
            break
    else:
        raise RuntimeError(f"{name}: no download")
    page.call("Input.dispatchKeyEvent", type="keyDown", key="Escape", code="Escape")
    page.call("Input.dispatchKeyEvent", type="keyUp", key="Escape", code="Escape")
    for f in files:
        path = os.path.join(out_dir, name + ".glb")
        open(path, "wb").write(base64.b64decode(f["b64"]))
        print(f"  {name}: {f['name']} -> {path} ({os.path.getsize(path)} bytes)")


def main():
    spec = json.load(open(sys.argv[1]))
    out = sys.argv[2]
    os.makedirs(out, exist_ok=True)
    only = set(sys.argv[3:])
    page = Page()
    for clip in spec["clips"]:
        if only and clip["name"] not in only:
            continue
        author(page, clip, out)


if __name__ == "__main__":
    main()
