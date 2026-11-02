# uzor-devtools

Dev-time remote control for apps on `uzor-framework`. It is meant for
debug and test builds behind an app's own opt-in feature; release builds
should not link it.

* `Dispatcher` — the transport-agnostic core: one JSON request in, one
  JSON response out, plus an event log a transport can stream. It talks to
  the app through `uzor_framework::automation` (feature `automation`).
* `server` — a std-only HTTP/1.1 wrapper bound to `127.0.0.1` on a random
  port: `POST /rpc`, `GET /events`. A request needs the exact loopback
  `Host`, must not carry an `Origin` (browsers are refused with 403), and
  needs `Authorization: Bearer <token>` (401 otherwise, compared in
  constant time). The 32-byte token is random per run.
* `discovery` — the port and token are published in a `0600` file
  `dev-<pid>.json` under `$XDG_RUNTIME_DIR/<app>/` (else
  `~/.config/<app>/`), removed when the server stops.
* `uzor-dev` — a CLI that finds that file and calls the RPC.

## Methods

| method | params | result |
|---|---|---|
| `ping` | – | `{pong, app, pid, protocol, methods}` |
| `windows` | – | open windows |
| `dump-tree` | `window?`, `widgets?` | accessibility nodes (+ every registered widget) |
| `click` | `selector` or `x`,`y`; `button?`, `count?` | `{id?, x, y}` |
| `hover` | `selector` or `x`,`y` | `{id?, x, y}` |
| `focus` | `selector` | `{id, focused}` |
| `type` | `text`, `selector?` | `{chars}` |
| `key` | `key` (e.g. `Enter`, `Ctrl+A`), `repeat?` | `{key}` |
| `scroll` | `selector` or `x`,`y`; `dx?`, `dy`, `unit?` (`px`/`lines`) | `{x, y}` |
| `screenshot` | `window?`, `source?` (`auto`/`window`/`offscreen`) | `{png_base64, width, height, source}` |
| `state` | – | the app's whitelisted state report |

Selectors are `uzor::a11y::query` selectors: `#id`, `button[name="Save"]`,
`textinput[name~=nick]`, `[id^=room-]:nth(0)`. `#id` also finds widgets
without an accessibility role.

Errors are `{"ok":false,"error":{"code","message"}}` with stable English
codes: `bad_request`, `unknown_method`, `bad_selector`, `not_found`,
`ambiguous`, `no_window`, `unsupported`, `timeout`, `host_closed`, `failed`.
