# haalka [হালকা](https://translate.google.com/?sl=bn&tl=en&text=%E0%A6%B9%E0%A6%BE%E0%A6%B2%E0%A6%95%E0%A6%BE&op=translate)

[![Crates.io Version](https://img.shields.io/crates/v/haalka?style=for-the-badge)](https://crates.io/crates/haalka)
[![Docs.rs](https://img.shields.io/docsrs/haalka?style=for-the-badge)](https://docs.rs/haalka)

```text
in bengali, haalka means "light" (e.g. not heavy) and can also be used to mean "easy"
```

[haalka](https://github.com/databasedav/haalka) is an ergonomic reactive [Bevy](https://github.com/bevyengine/bevy) UI library powered by the incredible [FRP](https://en.wikipedia.org/wiki/Functional_reactive_programming) signals of [futures-signals](https://github.com/Pauan/rust-signals) and the convenient async ECS of [bevy-async-ecs](https://github.com/dlom/bevy-async-ecs) with API ported from web UI libraries [MoonZoon](https://github.com/MoonZoon/MoonZoon) and [Dominator](https://github.com/Pauan/rust-dominator).

While haalka is primarily targeted at UI and provides high level UI abstractions as such, its [core abstraction](https://docs.rs/haalka/latest/haalka/struct.RawHaalkaEl.html) can be used to manage signals-powered reactivity for any entity, not just [bevy_ui nodes](https://github.com/bevyengine/bevy/blob/main/crates/bevy_ui/src/node_bundles.rs).

## examples

<p align="center">
  <img src="https://raw.githubusercontent.com/databasedav/haalka/main/docs/static/counter.gif">
</p>

```rust
use bevy::prelude::*;
use haalka::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, HaalkaPlugin))
        .add_systems(Startup, (ui_root, camera))
        .run();
}

#[derive(Component)]
struct Counter(Mutable<i32>);

fn ui_root(world: &mut World) {
    let counter = Mutable::new(0);
    El::<NodeBundle>::new()
        .height(Val::Percent(100.))
        .width(Val::Percent(100.))
        .align_content(Align::center())
        .child(
            Row::<NodeBundle>::new()
                .with_style(|mut style| style.column_gap = Val::Px(15.0))
                .item(counter_button(counter.clone(), "-", -1))
                .item(El::<TextBundle>::new().text_signal(counter.signal().map(|count| {
                    Text::from_section(
                        count.to_string(),
                        TextStyle {
                            font_size: 30.0,
                            ..default()
                        },
                    )
                })))
                .item(counter_button(counter.clone(), "+", 1))
                .update_raw_el(move |raw_el| raw_el.insert(Counter(counter))),
        )
        .spawn(world);
}

fn counter_button(counter: Mutable<i32>, label: &str, step: i32) -> impl Element {
    let hovered = Mutable::new(false);
    El::<NodeBundle>::new()
        .width(Val::Px(45.0))
        .align_content(Align::center())
        .background_color_signal(
            hovered
                .signal()
                .map_bool(|| Color::hsl(300., 0.75, 0.85), || Color::hsl(300., 0.75, 0.75))
                .map(BackgroundColor),
        )
        .hovered_sync(hovered)
        .on_click(move || *counter.lock_mut() += step)
        .child(El::<TextBundle>::new().text(Text::from_section(
            label,
            TextStyle {
                font_size: 30.0,
                ..default()
            },
        )))
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
```

```bash
cargo run --example counter  # the example above
cargo run --example button  # port of https://github.com/bevyengine/bevy/blob/main/examples/ui/button.rs
cargo run --example align  # alignment API demo, port of https://github.com/MoonZoon/MoonZoon/tree/main/examples/align and https://github.com/MoonZoon/MoonZoon/tree/main/examples/align_content
cargo run --example scroll  # scrollability API demo, inspired by https://github.com/mintlu8/bevy-rectray/blob/main/examples/scroll_discrete.rs
cargo run --example scroll_grid  # i can't believe it's not scrolling !
cargo run --example snake  # with adjustable grid size and tick rate
cargo run --example ecs_ui_sync  # forward ecs changes to the ui
cargo run --example key_values_sorted  # text inputs, scrolling/viewport control, and reactive lists; promises made promises kept ! https://discord.com/channels/691052431525675048/1192585689460658348/1193431789465776198 (yes i take requests)

# ui challenges from https://github.com/bevyengine/bevy/discussions/11100
cargo run --example challenge01  # game menu
cargo run --example challenge02  # inventory
cargo run --example challenge03  # health bar
cargo run --example challenge04  # responsive menu
cargo run --example challenge05  # character editor
```

## Bevy version compatibility

|Bevy|haalka|
|-|-|
|`0.13`|`0.1`|

## license
All code in this repository is dual-licensed under either:

- MIT License ([LICENSE-MIT](https://github.com/databasedav/haalka/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/databasedav/haalka/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)

at your option.

Assets used in examples may be licensed under different terms, see the [`examples` README](https://github.com/databasedav/haalka/blob/main/examples/README.md).

### your contributions
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
