all notable changes to this project will be documented in this file

the format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Common Changelog](https://common-changelog.org/), and this project vaguely adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

## unreleased

### changed

- upgraded to bevy 0.17
- [jonmo](https://github.com/databasedav/jonmo) is now the default signals backend for haalka, the deprecated futures-signals backend is still available via feature flags
    - to migrate seamlessly:
        - update your haalka installation by disabling default features and adding the appropriate futures-signals feature, likely either `futures_signals_ui` or `futures_signals_text_input`
        - replace `HaalkaPlugin` with `HaalkaFuturesSignalsPlugin`
        - replace your haalka imports with `haalka::futures_signals`, e.g. `use haalka::prelude::*` should become `use haalka::futures_signals::prelude::*`
        - once your application is functional again with the bevy 0.16 to 0.17 changes, re-add `HaalkaPlugin` and migrate your application logic to jonmo signals, which are semantically identical to futures-signals but use ECS components and systems as data sources; for example, all of your `Mutable`s will become `Component`s or `Resource`s; see the examples' diffs for reference migrations and the `futures_signals_jonmo_compat` example for how to integrate both signals backends; finally, feel free to ask questions in the [discord](https://discord.com/channels/691052431525675048/1343672205233360999)
        - remove the futures-signals features and re-enable default features if desired
- component and system based alignment management instead of baking it into alignable structs at compile time, high level API unchanged, see `align.rs`
- `NodeBuilder` and `RawHaalkaEl` consolidated into jonmo's `jonmo::Builder`, `.update_raw_el` becomes `.with_builder`
- `CursorOnHover` renamed to `Cursor`
- `PointerEventAware`, `Cursor`, and `UiRootable` methods no longer insert `Pickable::default()` into the element, it must now be managed by the user
- `PointerEventAware` systems run in `PreUpdate` after `PickingSystems::Last` instead of original `Update`

### added

- `Hoverable`, `Pressable`, and `Draggable` marker components for enabling maintaining of `Hovered`, `Pressed`, and `Dragged` components
- `Stripe` base element, whose flex direction can be set dynamically
- normalized `PointerEventAware` methods across hovered, clicked, pressed, and dragged, all with `_disableable`, `_disableable_signal`, `_change`, `_change_disableable`, `_change_disableable_signal`, and `_throttled` variants
- panics/warnings that cloning base `Element` structs is a bug

### removed

- `impl_haalka_methods!` call for `Button` node type

# 0.5.1 (2025-07-05)

### fixed

- text input race bug

# 0.5.0 (2025-07-02)

### added

- ability to disable clearing previous text input selections on focus change

### changed

- upgraded Bevy to `0.16.1`
- migrated `TextInput` from [bevy_cosmic_edit](https://github.com/Dimchikkk/bevy_cosmic_edit) to [bevy_ui_text_input](https://github.com/ickshonpe/bevy_ui_text_input)
- upgraded to rust 2024 edition
- migrated from once-cell to `std::sync::LazyLock`

### removed

- `Sizeable` trait (migration: modify the `Node` component directly)

# 0.4.1 (2025-04-08)

### added

- `scroll_normalizer` for more predictable mouse scrolling

### changed

- `UiRoot` is now a component, allowing multiple ui roots

### removed

- no longer bundling fira mono, using Bevy's embedded default font directly

# 0.4.0 (2025-04-04)

### added
- `ViewportMutable::viewport_x/y_sync` methods
- blanket `impl<NodeType: Bundle> From<NodeType> for AlignabilityFacade`
- `LogicalRect` `SystemParam`
- `SyncBoxSignal` and `boxed_sync` signal utils

### changed
- **breaking:** renamed `RawHaalkaEl::on_signal_one_shot...` to `RawHaalkaEl::on_signal_with_system...`
- **breaking:** mutable viewport uses Bevy-native `ScrollPosition` instead of manually managed clipped child
- **breaking:** renamed `Viewport.x/y` to `Viewport.offset_x/offset_y`
- **breaking:** `ViewportMutable::mutable_viewport` takes new overflow `Axis`
- **breaking:** `impl_haalka_methods!` derived `_signal` methods take `Signal<Item = impl Into<Option<$field_type>>` instead of `Signal<Item = $field_type>`
- **breaking:** `raw::utils::remove_system_holder_on_remove` takes `Arc<OnceLock>` instead of `Mutable`
- one-shot system id's stored in `Arc<OnceLock>` instead of `Mutable`
- `MutableViewport` is now an observable `Event`, used internally for `ViewportMutable::on_viewport_location_change...`
- basic scroll handler respects pixel scroll units
- reduce `GRID_TRACK_FLOAT_PRECISION_SLACK` by an order of magnitude
- snake example updated to use observers

### removed
- **breaking:** `LimitToBody` and its functionality
- **breaking:** `MutableViewport.limit_to_body`
- **breaking:** `ViewportMarker`

### fixed
- `OnHoverMouseWheelScrollable` methods no longer allow scroll before first hover (0.2.4 fix was partially reverted in 0.3.0)

# 0.3.0 (2025-02-09)

### added
- `BoxShadow`, `ScrollPosition`, and `GlobalZIndex` convenience methods for base elements

### changed
- upgraded Bevy to `0.15.2`
- `GlobalEventAware` can register multiple handlers for the same event type, and the registering entity and event data is passed into the handler
- `.on_click_outside` methods use `GlobalEventAware` rather than bespoke handling
- completed tasks are cleaned up on completion rather than waiting until entity despawn
- updated examples to use latest Bevy and haalka idioms

### fixed
- text input focus desyncing

# 0.2.4 (2025-02-04)

### fixed
- `OnHoverMouseWheelScrollable` methods only scroll when the element is hovered

# 0.2.3 (2025-01-05)

### fixed
- removed and ignored development `trunk` artifact which was inflating crate size

# 0.2.2 (2025-01-05)

### added
- wasm support
- serve wasm examples via github pages, including pr previews
- `.on_viewport_location_change` methods for reacting to viewport changes
- `DebugUiPlugin`, thin helper wrapper over bevy's debug ui overlay
- CI

### fixed
- `OnHoverMouseWheelScrollable` handles scrolling outside scene boundaries when `LimitToBody` is lax

### changed
- `MutableViewportSettings` renamed to `MutableViewport`
- `PointerEventAware::on_hovered_change_with_system` is a deferred update
- don't `.gitignore` `Cargo.lock`
- use granular `bevy_...` dependencies

# 0.2.1 (2024-10-19)

### added
- `multicam` feature

# 0.2.0 (2024-10-12)

### added
- `PointerEventAware::on_click_outside...` methods
- `RawHaalkaEl::observe`
- `RawHaalkaEl::on_remove` for adding removal hooks to elements
- `RawHaalkaEl::on_spawn_with_system`
- `Enter` and `Leave` events triggered by hover management system
- component-based event handler blockability
- `PointerEventAware` pressing with system methods with throttle variants
- `ElementWrapper::into_el` for encapsulating ultimate element building logic
- `text_input::FocusedTextInput` resource for exposing control of `bevy_cosmic_edit` focus
- `BasicScrollHandler` correctly handles `ScrollDirection::Both`, holding shift to scroll horizontally
- `BorderRadius` convenience methods for base elements
- release CI powered by [release-plz](https://github.com/MarcoIeni/release-plz)
- `impl From<RawHaalkaEl>` for all built-in element types
- `Signal` utility `signal_eq`
- `raw::utils` module containing utilities for managing `RawHaalkaEl`s
- `raw::AppendDirection` renamed to `DeferredUpdaterAppendDirection`

### changed
- upgraded Bevy to `0.14.2`
- multiple scrolling, pressed/pressing, text input focus change, and text input change handlers can be registered on the same entity
- reactive methods and event handling use one shot systems throughout
- child entities attached to parents before being populated
- hovering methods triggered by observers on `Enter` and `Leave`
- cursor API uses observers
- control input focus using `text_input::FocusedTextInput` instead of `bevy_cosmic_edit::FocusedWidget`
- `Scrollable` renamed to `MouseWheelScrollable` (`HoverableScrollable` to `OnHoverMouseWheelScrollable`) and methods migrated to observers
- `Cursorable` renamed to `CursorOnHoverable` and is component driven with a signal layer on top
- library modules reorganized to better control privacy and clarify docs
- scrollability wrapper element management moved to `ViewportMutable::mutable_viewport`
- re-export `bevy_cosmic_edit`, which re-exports `cosmic-text`

### fixed
- throttle-able pressing functions now correctly block immediately after the first press
- `ViewportMutable` elements can now be self-aligned
- `UiRootable::ui_root` adds `Pickable` component

# 0.1.1 (2024-07-18)

### added
- reactive cursor API via `Cursorable` trait
- cursor API usage in the `inventory` example (with issues)
- `calculator` example
- convenience methods for `RawHaalkaEl`: `on_event_disableable`, `on_event_mut_disableable`, and `on_event_propagation_stoppable_disableable`
- `Nameable` convenience trait, for reactively setting the `Name` component
- `Signal`/`Mutable` utilities `sync`, `sync_neq`, and `flip`
- `UiRootable` trait, for marking the UI root

### changed
- use one shot system in `RawHaalkaEl::on_event_with_system_disableable`

### fixed
- static children are now spawned synchronously, eliminating the pop in visible when done asynchronously

# 0.1.0 (2024-06-27)

### added
- initial release
