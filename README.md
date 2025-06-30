# haalka [হালকা](https://translate.google.com/?sl=bn&tl=en&text=%E0%A6%B9%E0%A6%BE%E0%A6%B2%E0%A6%95%E0%A6%BE&op=translate)

[![Crates.io Version](https://img.shields.io/crates/v/haalka?style=for-the-badge)](https://crates.io/crates/haalka)
[![Docs.rs](https://img.shields.io/docsrs/haalka?style=for-the-badge)](https://docs.rs/haalka)
[![Following released Bevy versions](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue?style=for-the-badge)](https://bevyengine.org/learn/quick-start/plugin-development/#main-branch-tracking)

```text
in bengali, haalka means "light" (e.g. not heavy) and can also be used to mean "easy"
```

[haalka](https://github.com/databasedav/haalka) is an ergonomic reactive [Bevy](https://github.com/bevyengine/bevy) UI library powered by the incredible [FRP](https://en.wikipedia.org/wiki/Functional_reactive_programming) signals of [futures-signals](https://github.com/Pauan/rust-signals) and the convenient async ECS of [bevy-async-ecs](https://github.com/dlom/bevy-async-ecs) with API ported from web UI libraries [MoonZoon](https://github.com/MoonZoon/MoonZoon) and [Dominator](https://github.com/Pauan/rust-dominator).

While haalka is primarily targeted at UI and provides high level UI abstractions as such, its [core abstraction](https://docs.rs/haalka/latest/haalka/struct.RawHaalkaEl.html) can be used to manage signals-powered reactivity for any entity, not just [`bevy_ui` nodes](https://github.com/bevyengine/bevy/blob/main/crates/bevy_ui/src/node_bundles.rs).

## assorted features

- signals integration for all entities, components, and children
    - constant time reactive updates for collections via futures-signals' `MutableVec` and `MutableBTreeMap`
- simple high-level alignment semantics ported from MoonZoon (see align example below)
- pointer event handling methods
    - hovered change methods (including web-style `Enter` and `Leave` events)
    - click and on-click-outside methods
    - pressing methods, with throttle-ability
- cursor-on-hover management
- global event handling methods
- mouse wheel scroll handling methods
- signals-integrated text input, a thin layer on top of [bevy_ui_text_input](https://github.com/ickshonpe/bevy_ui_text_input)
- viewport mutation handling methods
- simple grid layout model ported from MoonZoon
- macro rules for adding signal helper methods to custom element structs

## considerations

- Reactive updates done by haalka are [**eventually consistent**](https://en.wikipedia.org/wiki/Eventual_consistency), that is, once some ECS world state has been updated, any downstream reactions should not be expected to run in the same frame. This is due to the indirection involved with using an async signals library, which dispatches Bevy commands after polling by the async runtime. The resulting "lag" should not be noticeable in most popular cases, e.g. reacting to hover/click state or synchronizing UI (one can run the examples to evaluate this themselves), but in cases where frame perfect responsiveness is critical, one should simply use Bevy-native systems directly.

## [feature flags](https://docs.rs/haalka/latest/haalka/#feature-flags-1)

## examples
<p align="center">
  <img src="https://raw.githubusercontent.com/databasedav/haalka/main/docs/static/counter.gif">
</p>

```rust no_run
use bevy::prelude::*;
use haalka::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, HaalkaPlugin))
        .add_systems(
            Startup,
            (
                |world: &mut World| {
                    ui_root().spawn(world);
                },
                camera,
            ),
        )
        .run();
}

#[derive(Component)]
struct Counter(Mutable<i32>);

fn ui_root() -> impl Element {
    let counter = Mutable::new(0);
    El::<Node>::new()
        .height(Val::Percent(100.))
        .width(Val::Percent(100.))
        .cursor(CursorIcon::default())
        .align_content(Align::center())
        .child(
            Row::<Node>::new()
                .with_node(|mut node| node.column_gap = Val::Px(15.0))
                .item(counter_button(counter.clone(), "-", -1))
                .item(
                    El::<Text>::new()
                        .text_font(TextFont::from_font_size(25.))
                        .text_signal(counter.signal_ref(ToString::to_string).map(Text)),
                )
                .item(counter_button(counter.clone(), "+", 1))
                .update_raw_el(move |raw_el| raw_el.insert(Counter(counter))),
        )
}

fn counter_button(counter: Mutable<i32>, label: &str, step: i32) -> impl Element {
    let hovered = Mutable::new(false);
    El::<Node>::new()
        .width(Val::Px(45.0))
        .align_content(Align::center())
        .border_radius(BorderRadius::MAX)
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .background_color_signal(
            hovered
                .signal()
                .map_bool(|| Color::hsl(300., 0.75, 0.85), || Color::hsl(300., 0.75, 0.75))
                .map(BackgroundColor),
        )
        .hovered_sync(hovered)
        .on_click(move || *counter.lock_mut() += step)
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(25.))
                .text(Text::new(label)),
        )
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
```

### on the web

All examples are compiled to wasm for both webgl2 and webgpu (check [compatibility](<https://github.com/gpuweb/gpuweb/wiki/Implementation-Status#implementation-status>)) and deployed to github pages.

- [**`counter`**](https://github.com/databasedav/haalka/blob/main/examples/counter.rs) [webgl2](https://databasedav.github.io/haalka/examples/webgl2/counter/) [webgpu](https://databasedav.github.io/haalka/examples/webgpu/counter/)

    the example above, a simple counter

- [**`button`**](https://github.com/databasedav/haalka/blob/main/examples/button.rs) [webgl2](https://databasedav.github.io/haalka/examples/webgl2/button/) [webgpu](https://databasedav.github.io/haalka/examples/webgpu/button/)

    a basic button, port of <https://github.com/bevyengine/bevy/blob/main/examples/ui/button.rs>

- [**`align`**](https://github.com/databasedav/haalka/blob/main/examples/align.rs) [webgl2](https://databasedav.github.io/haalka/examples/webgl2/align/) [webgpu](https://databasedav.github.io/haalka/examples/webgpu/align/)

    alignment API demo, port of <https://github.com/MoonZoon/MoonZoon/tree/main/examples/align> and <https://github.com/MoonZoon/MoonZoon/tree/main/examples/align_content>

- [**`scroll`**](https://github.com/databasedav/haalka/blob/main/examples/scroll.rs) [webgl2](https://databasedav.github.io/haalka/examples/webgl2/scroll/) [webgpu](https://databasedav.github.io/haalka/examples/webgpu/scroll/)

    scrollability API demo, inspired by <https://github.com/mintlu8/bevy-rectray/blob/main/examples/scroll_discrete.rs>

- [**`scroll_grid`**](https://github.com/databasedav/haalka/blob/main/examples/scroll_grid.rs) [webgl2](https://databasedav.github.io/haalka/examples/webgl2/scroll_grid/) [webgpu](https://databasedav.github.io/haalka/examples/webgpu/scroll_grid/)

    i can't believe it's not scrolling!

- [**`snake`**](https://github.com/databasedav/haalka/blob/main/examples/snake.rs) [webgl2](https://databasedav.github.io/haalka/examples/webgl2/snake/) [webgpu](https://databasedav.github.io/haalka/examples/webgpu/snake/)

    the classic, with adjustable grid size and tick rate

- [**`dot_counter`**](https://github.com/databasedav/haalka/blob/main/examples/dot_counter.rs) [webgl2](https://databasedav.github.io/haalka/examples/webgl2/dot_counter/) [webgpu](https://databasedav.github.io/haalka/examples/webgpu/dot_counter/)

    forward ecs changes to the ui, throttled button presses

- [**`key_values_sorted`**](https://github.com/databasedav/haalka/blob/main/examples/key_values_sorted.rs) [webgl2](https://databasedav.github.io/haalka/examples/webgl2/key_values_sorted/) [webgpu](https://databasedav.github.io/haalka/examples/webgpu/key_values_sorted/)

    text inputs, scrolling/viewport control, and reactive lists; promises made promises kept! <https://discord.com/channels/691052431525675048/1192585689460658348/1193431789465776198> (yes I take requests)

- [**`calculator`**](https://github.com/databasedav/haalka/blob/main/examples/calculator.rs) [webgl2](https://databasedav.github.io/haalka/examples/webgl2/calculator/) [webgpu](https://databasedav.github.io/haalka/examples/webgpu/calculator/)

    simple calculator, spurred by <https://discord.com/channels/691052431525675048/885021580353237032/1263661461364932639>

- [**`nested_lists`**](https://github.com/databasedav/haalka/blob/main/examples/nested_lists.rs) [webgl2](https://databasedav.github.io/haalka/examples/webgl2/nested_lists/) [webgpu](https://databasedav.github.io/haalka/examples/webgpu/nested_lists/)

    nested dynamic lists, arbitrarily deeply nested retained reactivity, spurred by <https://discord.com/channels/691052431525675048/885021580353237032/1356769984474517617>

- [**`main_menu`**](https://github.com/databasedav/haalka/blob/main/examples/main_menu.rs) [webgl2](https://databasedav.github.io/haalka/examples/webgl2/main_menu/) [webgpu](https://databasedav.github.io/haalka/examples/webgpu/main_menu/)

    sub menus, sliders, dropdowns, reusable composable widgets, gamepad navigation

- [**`inventory`**](https://github.com/databasedav/haalka/blob/main/examples/inventory.rs) [webgl2](https://databasedav.github.io/haalka/examples/webgl2/inventory/) [webgpu](https://databasedav.github.io/haalka/examples/webgpu/inventory/)

    grid, icons, drag and drop, tooltips

- [**`healthbar`**](https://github.com/databasedav/haalka/blob/main/examples/healthbar.rs) [webgl2](https://databasedav.github.io/haalka/examples/webgl2/healthbar/) [webgpu](https://databasedav.github.io/haalka/examples/webgpu/healthbar/)

    3D character anchor, customizable widgets

- [**`responsive_menu`**](https://github.com/databasedav/haalka/blob/main/examples/responsive_menu.rs) [webgl2](https://databasedav.github.io/haalka/examples/webgl2/responsive_menu/) [webgpu](https://databasedav.github.io/haalka/examples/webgpu/responsive_menu/)

    nine-patch buttons, screen size reactivity

- [**`character_editor`**](https://github.com/databasedav/haalka/blob/main/examples/character_editor.rs) [webgl2](https://databasedav.github.io/haalka/examples/webgl2/character_editor/) [webgpu](https://databasedav.github.io/haalka/examples/webgpu/character_editor/)

    scrollable buttons, mutable viewport, text input reactivity

Or run them locally with `cargo`.
```bash
cargo run --example counter
cargo run --example button
cargo run --example align
cargo run --example scroll
cargo run --example scroll_grid
cargo run --example snake
cargo run --example dot_counter
cargo run --example key_values_sorted
cargo run --example calculator
cargo run --example nested_lists

# ui challenges from https://github.com/bevyengine/bevy/discussions/11100
cargo run --example main_menu
cargo run --example inventory
cargo run --example healthbar
cargo run --example responsive_menu
cargo run --example character_editor
```
Or with [`just`](https://github.com/casey/just), e.g. `just example snake -r`.

## Bevy compatibility
|bevy|haalka|
|-|-|
|`0.15`|`0.4`|
|`0.14`|`0.2`|
|`0.13`|`0.1`|

## development
- avoid the gh-pages branch and include submodules when fetching the repo
    ```bash
    git clone --single-branch --branch main --recurse-submodules https://github.com/databasedav/haalka.git
    ```
- install [just](https://github.com/casey/just?tab=readme-ov-file#installation)
- install [nickel](https://github.com/tweag/nickel?tab=readme-ov-file#run) for modifying CI configuration (`nickel` must be in your PATH)
- install [File Watcher](https://marketplace.visualstudio.com/items?itemName=appulate.filewatcher) for automatically syncing nickels

## license
All code in this repository is dual-licensed under either:

- MIT License ([LICENSE-MIT](https://github.com/databasedav/haalka/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/databasedav/haalka/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)

at your option.

Assets used in examples may be licensed under different terms, see the [`examples` README](https://github.com/databasedav/haalka/blob/main/examples/README.md).

### your contributions
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
