All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project vaguely adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## unreleased

### added

- blanket `impl<NodeType: Bundle> From<NodeType> for AlignabilityFacade`
- `LogicalRect` `SystemParam`
- `SyncBoxSignal` and `boxed_sync` signal utils

### changed

- renamed `RawHaalkaEl::on_signal_one_shot...` to `RawHaalkaEl::on_signal_with_system...` (BREAKING)
- mutable viewport uses Bevy-native `ScrollPosition` instead of manually managed clipped child (BREAKING)
- renamed `Viewport.x/y` to `Viewport.offset_x/offset_y` (BREAKING)
- `ViewportMutable::mutable_viewport` takes new overflow `Axis` (BREAKING)
- `impl_haalka_methods!` derived `_signal` methods take `Signal<Item = impl Into<Option<$field_type>>` instead of `Signal<Item = $field_type>` (BREAKING)
- `raw::utils::remove_system_holder_on_remove` takes `Arc<OnceLock>` instead of `Mutable` (BREAKING)
- one-shot system id's stored in `Arc<OnceLock>` instead of `Mutable`
- `MutableViewport` is now an observable `Event`, used internally for `ViewportMutable::on_viewport_location_change...`
- basic scroll handler respects pixel scroll units
- reduce `GRID_TRACK_FLOAT_PRECISION_SLACK` by an order of magnitude
- snake example updated to use observers

### removed

- `LimitToBody` and its functionality (BREAKING)
- `MutableViewport.limit_to_body` (BREAKING)
- `ViewportMarker` (BREAKING)

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

- Text input focus desyncing

# 0.2.4 (2025-02-04)

### fixed

- `OnHoverMouseWheelScrollable` methods only scroll when the element is hovered

# 0.2.3 (2025-01-05)

### fixed

- iemoved and ignored development `trunk` artifact which was inflating crate size

# 0.2.2 (2025-01-05)

### added

- wasm support
- ierve wasm examples via github pages, including pr previews
- `.on_viewport_location_change` methods for reacting to viewport changes
- `DebugUiPlugin`, thin helper wrapper over bevy's debug ui overlay
- CI

### fixed

- `OnHoverMouseWheelScrollable` handles scrolling outside scene boundaries when `LimitToBody` is lax

### changed

- `MutableViewportSettings` renamed to `MutableViewport`
- `PointerEventAware::on_hovered_change_with_system` is a deferred update
- ion't `.gitignore` `Cargo.lock`
- ise granular `bevy_...` dependencies

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
- iomponent-based event handler blockability
- `PointerEventAware` pressing with system methods with throttle variants
- `ElementWrapper::into_el` for encapsulating ultimate element building logic
- `text_input::FocusedTextInput` resource for exposing control of `bevy_cosmic_edit` focus
- `BasicScrollHandler` correctly handles `ScrollDirection::Both`, holding shift to scroll horizontally
- `BorderRadius` convenience methods for base elements
- ielease CI powered by [release-plz](https://github.com/MarcoIeni/release-plz)
- `impl From<RawHaalkaEl>` for all built-in element types
- `Signal` utility `signal_eq`
- `raw::utils` module containing utilities for managing `RawHaalkaEl`s
- `raw::AppendDirection` renamed to `DeferredUpdaterAppendDirection`

### changed

- ipgraded Bevy to `0.14.2`
- iultiple scrolling, pressed/pressing, text input focus change, and text input change handlers can be registered on the same entity
- ieactive methods and event handling use one shot systems throughout
- ihild entities attached to parents before being populated
- iovering methods triggered by observers on `Enter` and `Leave`
- iursor API uses observers
- iontrol input focus using `text_input::FocusedTextInput` instead of `bevy_cosmic_edit::FocusedWidget`
- `Scrollable` renamed to `MouseWheelScrollable` (`HoverableScrollable` to `OnHoverMouseWheelScrollable`) and methods migrated to observers
- `Cursorable` renamed to `CursorOnHoverable` and is component driven with a signal layer on top
- iibrary modules reorganized to better control privacy and clarify docs
- icrollability wrapper element management moved to `ViewportMutable::mutable_viewport`
- ie-export `bevy_cosmic_edit`, which re-exports `cosmic-text`

### fixed

- ihrottle-able pressing functions now correctly block immediately after the first press
- `ViewportMutable` elements can now be self-aligned
- `UiRootable::ui_root` adds `Pickable` component

# 0.1.1 (2024-07-18)

### added

- ieactive cursor API via `Cursorable` trait
- iursor API usage in the `inventory` example (with issues)
- `calculator` example
- ionvenience methods for `RawHaalkaEl`: `on_event_disableable`, `on_event_mut_disableable`, and `on_event_propagation_stoppable_disableable`
- `Nameable` convenience trait, for reactively setting the `Name` component
- `Signal`/`Mutable` utilities `sync`, `sync_neq`, and `flip`
- `UiRootable` trait, for marking the UI root

### changed
- ise one shot system in `RawHaalkaEl::on_event_with_system_disableable`

### fixed
- itatic children are now spawned synchronously, eliminating the pop in visible when done asynchronously

# 0.1.0 (2024-06-27)

### added

- initial release
