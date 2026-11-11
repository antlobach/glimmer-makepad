# Glimmer Makepad

A self-rendered native GUI backend for [Jolt](https://github.com/jolt-lang/jolt)
and [Glimmer](https://github.com/jolt-lang/glimmer), using Makepad through a Rust C ABI.
There is no browser, Go layer, or wrapper around OS controls.

Glimmer owns reactive components, reconciliation, keyed identity, and subscriptions.
Rust owns the retained widget tree, Makepad widgets, layout, drawing, and input.
The backend implements Glimmer's lifecycle, scheduling, child reordering, disposal,
and property-update contracts rather than introducing another reconciler.

**Production readiness is not established.** The implemented widgets are real native
Makepad widgets, but this is not a complete binding to Makepad's API. Android/iOS
startup, Jolt runtime embedding, packaging, accessibility, and device qualification
are not implemented or certified by this repository.

## Build and run

Requirements: Rust 1.93+, a platform C toolchain, and Jolt 0.8.25. Makepad uses the
unmodified crates.io `makepad-widgets` 1.0.0 release, with registry dependencies
and checksums pinned in `Cargo.lock`; Glimmer is pinned in `deps.edn`.
On Linux, install X11, Xcursor, Xrandr, Xi, ALSA, PulseAudio, and OpenGL development
libraries as listed in [CI](.github/workflows/ci.yml).

```sh
cargo build --workspace --locked
jolt -M:demo
```

Run the GUI on the OS main thread. Requiring the backend registers it with Glimmer:

```clojure
(require '[glimmer-makepad.core :as makepad]
         '[glimmer.core :as ui]
         '[glimmer.ratom :as r])

(defonce clicks (r/atom 0))

(defn app []
  [:vbox {:spacing 16 :padding 24}
   [:label {:label (str "Clicks: " @clicks) :font-size 24}]
   [:button {:label "Increment" :on-click #(swap! clicks inc)}]
   [:button {:label "Close" :on-click makepad/close!}]])

(ui/run app :title "Glimmer Makepad" :width 900 :height 640)
```

The full [example](examples/counter.clj) includes a controlled text input and keyed
rows that can be reversed without recreating their native widget handles.
`jolt -M:demo --smoke` updates reactive state from a worker, waits for reconciliation,
then closes the application and its Jolt process.

## Application lifetime (ABI 2)

**Makepad owns the entire application lifetime. Closing the GUI also ends Jolt,
its nREPL server, and all worker threads.** This is not an embeddable event loop
that returns control to a surviving host.

After GUI startup, `ui/run` / `gm_run` does not return. Normal native shutdown
ends the process with status 0; a native event-handler or Jolt callback failure
reports the error to stderr and ends it with a nonzero status. `GM_CLOSING` unmounts
the Glimmer root before normal termination. Invalid launch arguments still return
an error before startup, leaving the caller's context usable.

Do not rely on code after `ui/run`, Clojure `finally` blocks, shutdown hooks, or
Rust destructors to save application data. Complete required persistence before
requesting close. Headless `call-with-context` keeps its ordinary return/cleanup
contract. ABI 2 rejects old wrappers that assumed the GUI loop returned.

### Live REPL

For live REPL development, start `jolt -M:demo --repl` and connect your editor to
the loopback port written to `.nrepl-port`. That server shares the window's Jolt
process. For example, `(swap! counter/count* inc)` or
`(reset! counter/name* "Anton")` updates the mounted UI without restarting it.
After redefining the root, use `(ui/reload! counter/app)` without starting a second
loop. With no argument, `(ui/reload!)` remounts the existing root function and picks
up redefinitions of the child components it calls.

The REPL is available only while the app is alive. Starting a fresh application
creates a fresh process. The port file can remain after process termination;
the next `--repl` startup overwrites it.

## Implemented surface

| Tag | Properties beyond shared style | Events |
| --- | --- | --- |
| `:vbox` | Ordered children | — |
| `:hbox` | Ordered children | — |
| `:label` | `:label` | — |
| `:button` | `:label` | `:on-click`, zero arguments |
| `:text-input` | `:text`, `:placeholder` | `:on-change`, `:on-submit`, current string argument |

Shared style: `:width`, `:height` (`:fill`, `:fit`, or logical pixels), `:spacing`,
uniform `:padding` and `:margin`, `:font-size`, `:corner-radius`, `:align-x`,
`:align-y`, `:color`, and `:background`. Alignment is in `[0,1]`; colors are
`#RRGGBB`, `#RRGGBBAA`, or unsigned `0xRRGGBBAA` integers. Label backgrounds and
corner radii belong on their containing box. Unknown properties are errors.
Removing a property restores its default on the next reconciliation.

There are no bindings yet for scrolling, virtualized lists, images, menus,
dialogs, rich text, or custom application shaders. This is a single-window backend.
Only one Makepad event-loop startup is supported per process; reload the mounted
Glimmer root inside that loop instead of starting another one.

## Ownership and FFI

The public contract is [`include/glimmer_makepad.h`](include/glimmer_makepad.h).
Build products include a dynamic library and a static library. The ABI exposes
contexts, opaque widget IDs, tree mutation, text, style snapshots, native lifecycle,
wakeups, error reporting, and memory counters.

- Contexts and widget operations are confined to the creating OS thread. IDs are
  never reused. Do not pass raw Makepad pointers between languages.
- Removing or replacing a child destroys its whole subtree. Reordering preserves
  handles, native widgets, input selection, and Glimmer keyed identity.
- UTF-8 uses explicit byte lengths, including embedded NULs. Text-copy queries and
  insufficient-buffer failures do not partially overwrite the caller's buffer.
- Style validation is atomic. Invalid ownership mutations leave the tree intact.
- Rust panics are contained at exported C boundaries and inside native event
  dispatch. Invalid or misaligned C pointers still violate the caller's contract.
- Foreign callbacks must contain their exceptions and return nonzero on failure.
  The Jolt wrapper reports the original error before native termination; errors
  are not rethrown into a host after GUI shutdown.
- `gm_wake` and allocator counters are thread-safe. Use `makepad/schedule!`,
  `ui/on-gui`, or reactive atoms for worker/nREPL changes; do not mutate native
  widgets directly from a worker. Input callbacks run on the GUI thread.
- The native loop is a Jolt `:blocking` foreign call; callbacks are `:collect-safe`
  so another Jolt thread can collect while Makepad owns the main thread.

`makepad/call-with-context` supports headless construction and inspection with
resources released on return. It does not start an event loop. `makepad/memory-stats`
adds live widget and registered-handler counts while a context is active; call it
on the owning thread, or through `makepad/schedule!` during a GUI run.

## Verification and memory

```sh
cargo test --workspace --locked
jolt -M:test
cc -Wall -Wextra -Werror -Iinclude examples/ownership.c \
  -Ltarget/debug -lglimmer_makepad -Wl,-rpath,"$PWD/target/debug" -o /tmp/ownership
/tmp/ownership
cargo run -p glimmer-makepad --example label --locked
jolt -M:demo --smoke
```

The C compile command applies to Unix development hosts. Tests cover recursive
ownership teardown, cross-context/thread rejection, keyed reorder identity,
UTF-8/NUL and short buffers, invalid style snapshots, failed-mount cleanup,
subscription disposal, real Makepad pointer hit testing, text-input selection,
painter ordering across differently colored backgrounds, and rejected launch
arguments retaining caller ownership.

The C ownership program warms the tree, then asserts that 20,000 create/attach/
remove operations cause **no further Rust allocations and no live-byte growth**.
It also checks that 1,000 create/destroy context cycles return to the same baseline.

`gm_memory_live_bytes`, `gm_memory_peak_bytes`, and `gm_memory_allocation_count`
measure requested Rust heap bytes and allocation calls made through this library's
allocator. They are not process RSS and exclude Chez/Jolt, GPU resources, and OS
allocations. Fixed-capacity thread-local tables and toolkit caches can outlive a
context. Use OS process tools alongside these counters; on macOS, for example:

```sh
/usr/bin/time -l jolt -M:demo --smoke
```

Earlier macOS x86_64 debug measurements: the C ownership executable retained
1,448 Rust bytes with its root alive and 628 bytes after context destruction;
its maximum process RSS was 5,525,504 bytes. These are development measurements,
not budgets or guarantees.

The stock-Makepad ABI 2 smoke run on macOS x86_64 reached 14 native widgets and
4 handlers, then terminated normally. At the update checkpoint it reported
96,041,670 requested Rust heap bytes and a 96,317,440-byte Rust peak; maximum
process RSS across the run was 260,530,176 bytes. These are debug-build development
measurements, not memory budgets. There is no post-GUI heap snapshot: GUI shutdown
ends the process.

Native compilation and tests are configured for Linux, macOS, and Windows. Jolt
integration and the C ownership executable run on Linux and macOS in CI. Actual
window/framebuffer smoke verification has been performed on macOS x86_64 only.
Physical mouse/keyboard, IME, accessibility, Linux/Windows display sessions, and
mobile devices have not been qualified by that smoke run.

## Upstream Makepad

There is no vendored toolkit, Git fork, Cargo path override, or upstream source
patch. The binding ends its process on Makepad's `Shutdown` event and also on
platforms where the native loop returns; it does not attempt to release toolkit
globals for a host that continues running.

Dynamic templates retain Makepad's `draw_call_always` shader flag so later captions
cannot be batched underneath differently colored backgrounds. Stock Makepad 1.0.0
consumes this flag during shader compilation but can also print a spurious
`no matching field: draw_call_always` property-application diagnostic. The binding
does not patch or suppress that upstream diagnostic.

## License

Glimmer Makepad is [MIT](LICENSE). Makepad and all other registry dependencies
retain their own licenses; Makepad's platform package declares `MIT OR Apache-2.0`.
