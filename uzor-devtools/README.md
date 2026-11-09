# uzor-devtools

The transport of uzor's dev control: another process drives a uzor app over a
loopback JSON RPC. Apps do not depend on this crate directly; they turn the
whole thing on with one framework feature:

```toml
uzor-framework = { version = "=1.5.26", features = ["native", "dev-control"] }
```

```rust
let mut native = NativeOptions::default();
// `--dev` or MYAPP_DEV=1; without the feature the option is ignored (one stderr line).
native.dev_control = DevControlConfig::from_args_env("my-app", "my-app", "MYAPP_DEV");
run_native(app, cfg, native)
```

A build without `dev-control` contains none of it.

`--headless` (implies `--dev`; or `MYAPP_DEV=headless`) runs the same app with
no window and no display server: the framework's headless host ticks it in
real time, screenshots are painted offscreen, and the RPC is the same. A
build without the feature refuses `--headless` with an error.

* `uzor_framework::dev_control::Dispatcher` (in the framework) is the RPC
  core: one JSON request in, one JSON response out, driving the app through
  the framework's automation port. It plugs into this crate through the `Rpc` trait.
* `server` — a std-only HTTP/1.1 server bound to `127.0.0.1` on a random
  port: `POST /rpc`, `GET /events`. A request needs the exact loopback
  `Host` (421 otherwise), must not carry an `Origin` (browsers are refused
  with 403), and needs `Authorization: Bearer <token>` (401 otherwise,
  compared in constant time). The 32-byte token is random per run.
* `discovery` — the port and token are published in a `0600` file
  `dev-<pid>.json` under `$XDG_RUNTIME_DIR/<app>/` (else
  `~/.config/<app>/`, directory `0700`), removed when the server stops.
* `events` — the bounded event log `GET /events` long-polls.
* `uzor-dev` — a CLI that finds that file and calls the RPC
  (`cargo install uzor-devtools`, then `uzor-dev --app my-app ping`).

This crate does not depend on the framework, so the framework can depend on
it.

## Methods

| method | params | result |
|---|---|---|
| `ping` | – | `{pong, app, pid, protocol, methods}` |
| `windows` | – | open windows |
| `dump-tree` | `window?`, `widgets?` | accessibility nodes (+ every registered widget) |
| `click` | `selector` or `x`,`y`; `button?`, `count?` | `{id?, x, y}` |
| `hover` | `selector` or `x`,`y` | `{id?, x, y}` |
| `focus` | `selector` (clicks it, waits until it holds the focus) | `{id, window, focused}` |
| `type` | `text`, `selector?` (focused first, like `focus`), `clear?` (replace what the field holds), `ime?` (one IME commit) | `{chars}` |
| `key` | `key` (e.g. `Enter`, `Ctrl+A`), `repeat?` | `{key}` |
| `scroll` | `selector` or `x`,`y`; `dx?`, `dy`, `unit?` (`px`/`lines`) | `{x, y}` |
| `screenshot` | `window?`, `source?` (`auto`/`window`/`offscreen`) | `{png_base64, width, height, source}` |
| `state` | – | the app's whitelisted state report |
| `wait-for` | `target` (selector) + `state?` (`present`/`gone`/`focused`/`enabled`), `value?`; or `cond` (a JSON subset of `state`); `timeout_ms?` (5000), `window?` | `{id?, checks, frame, waited_ms}` |
| `settle` | `idle_ms?` (300), `timeout_ms?`, `window?` | `{frame, frames, checks, changes, waited_ms}` |
| `record` | `on` (`true`/`false`/`"on"`/`"off"`); none: status | off: `{recording, events, jsonl}` |
| `replay` | `events` (JSONL text or array), `speed?` (1; 0 = no pauses), `secrets?` | `{replayed, speed, ms}` |

`wait-for` and `settle` are checked once per painted frame (and every 250 ms
while nothing paints). `settle` waits until what the UI shows stopped
changing: the accessibility tree (names, values, bounds, states), the widget
rects, focus, hover and the state report. Pixel-only animation (a caret blink,
a sprite) does not keep it from settling. Both fail with `timeout`.

### Recording

`record on` taps the input the app receives (from the OS or the RPC) and
writes one JSON line per step, `{"t_ms", "method", "params"}`, with RPC
methods and params, so a recording can be replayed or edited by hand:

```jsonl
{"t_ms":0,"method":"click","params":{"selector":"#lh-code"}}
{"t_ms":420,"method":"type","params":{"clear":true,"into":"#lh-code","masked":true,"text":"<masked>"}}
{"t_ms":1730,"method":"key","params":{"key":"Enter"}}
```

Clicks name the widget under the pointer (`selector`: `#id`, or `[id="row:42"]`
when the id has selector syntax in it), coordinates only when there is none. Typing into one field and scrolls of one target merge
into one line; a typing line holds what the field ended up with and
replays with `clear`, so a field that already holds something (a remembered
name) comes out as recorded. Text typed into a masked field is never recorded: it is
written as `"<masked>"`, and `replay` refuses such a line unless `secrets`
supplies it (`{"lh-code": "…"}` by field id, or an array in order). A
replayed step with a selector first waits (5 s) for its target. Recording
and replay report progress in the event log (`record`, `record-state`,
`replay` events).

## In the browser

With the feature on a wasm build, an app passes
`WebOptions::default().dev_control(WebDevControl::new("my-app"))` to
`run_web_with`. When the page URL has `?dev=1`, `window.__uzorDev` exposes the
same RPC to page JavaScript (`await __uzorDev.call({method:"dump-tree"})`,
`__uzorDev.events(since)`); an app can add aliases. Nothing listens on the
network: tests reach it through whatever already drives the page (CDP
`Runtime.evaluate`). Screenshots are offscreen. Ship production wasm without
the feature.

Selectors are `uzor::a11y::query` selectors: `#id`, `button[name="Save"]`,
`textinput[name~=nick]`, `[id^=room-]:nth(0)`. `#id` also finds widgets
without an accessibility role.

Errors are `{"ok":false,"error":{"code","message"}}` with stable English
codes: `bad_request`, `unknown_method`, `bad_selector`, `not_found`,
`ambiguous`, `not_focused`, `no_window`, `unsupported`, `timeout`, `host_closed`, `failed`.

`GET /events?since=<seq>&wait_ms=<ms>` returns the events after `seq`
(calls: `{seq, t_ms, type:"rpc", method, ok, error?}`; recording and replay
events too), waiting up to `wait_ms` (at most 30 s) for the first one.

The CLI takes the new methods as `uzor-dev wait-for '#id' [state=gone]`,
`uzor-dev settle 500`, `uzor-dev record on`, `uzor-dev record off --out
s.jsonl`, `uzor-dev replay s.jsonl secrets=@secrets.json` (a `@PATH` value is
read from the file, so secrets stay off the command line).
