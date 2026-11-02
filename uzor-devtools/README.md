# uzor-devtools

The transport of uzor's dev control: another process drives a uzor app over a
loopback JSON RPC. Apps do not depend on this crate directly; they turn the
whole thing on with one framework feature:

```toml
uzor-framework = { version = "=1.5.23", features = ["native", "dev-control"] }
```

```rust
let mut native = NativeOptions::default();
// `--dev` or MYAPP_DEV=1; without the feature the option is ignored (one stderr line).
native.dev_control = DevControlConfig::from_args_env("my-app", "my-app", "MYAPP_DEV");
run_native(app, cfg, native)
```

A build without `dev-control` contains none of it.

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
| `type` | `text`, `selector?` (focused first, like `focus`), `ime?` (one IME commit) | `{chars}` |
| `key` | `key` (e.g. `Enter`, `Ctrl+A`), `repeat?` | `{key}` |
| `scroll` | `selector` or `x`,`y`; `dx?`, `dy`, `unit?` (`px`/`lines`) | `{x, y}` |
| `screenshot` | `window?`, `source?` (`auto`/`window`/`offscreen`) | `{png_base64, width, height, source}` |
| `state` | – | the app's whitelisted state report |

Selectors are `uzor::a11y::query` selectors: `#id`, `button[name="Save"]`,
`textinput[name~=nick]`, `[id^=room-]:nth(0)`. `#id` also finds widgets
without an accessibility role.

Errors are `{"ok":false,"error":{"code","message"}}` with stable English
codes: `bad_request`, `unknown_method`, `bad_selector`, `not_found`,
`ambiguous`, `not_focused`, `no_window`, `unsupported`, `timeout`, `host_closed`, `failed`.

`GET /events?since=<seq>&wait_ms=<ms>` returns the calls after `seq`
(`{seq, t_ms, type:"rpc", method, ok, error?}`), waiting up to `wait_ms`
(at most 30 s) for the first one.
