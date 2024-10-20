All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project vaguely adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## unreleased

# 0.2.1 (2024-10-19)

### added

- `multicam` feature

# 0.2.0 (2024-10-12)

### added

- `PointerEventAware::on_click_outside...` methods.
- `RawHaalkaEl::observe`.
- `RawHaalkaEl::on_remove` for adding removal hooks to elements.
- `RawHaalkaEl::on_spawn_with_system`.
- `Enter` and `Leave` events triggered by hover management system.
- Component-based event handler blockability.
- `PointerEventAware` pressing with system methods with throttle variants.
- `ElementWrapper::into_el` for encapsulating ultimate element building logic.
- `text_input::FocusedTextInput` resource for exposing control of `bevy_cosmic_edit` focus.
- `BasicScrollHandler` correctly handles `ScrollDirection::Both`, holding shift to scroll horizontally.
- `BorderRadius` convenience methods for base elements.
- release CI powered by [release-plz](https://github.com/MarcoIeni/release-plz).
- `impl From<RawHaalkaEl>` for all built-in element types.
- `Signal` utility `signal_eq`.
- `raw::utils` module containing utilities for managing `RawHaalkaEl`s.
- `raw::AppendDirection` renamed to `DeferredUpdaterAppendDirection`.

### changed

- Upgraded Bevy to `0.14.2`.
- Multiple scrolling, pressed/pressing, text input focus change, and text input change handlers can be registered on the same entity.
- Reactive methods and event handling use one shot systems throughout.
- Child entities attached to parents before being populated.
- Hovering methods triggered by observers on `Enter` and `Leave`.
- Cursor API uses observers.
- Control input focus using `text_input::FocusedTextInput` instead of `bevy_cosmic_edit::FocusedWidget`.
- `Scrollable` renamed to `MouseWheelScrollable` (`HoverableScrollable` to `OnHoverMouseWheelScrollable`) and methods migrated to observers.
- `Cursorable` renamed to `CursorOnHoverable` and is component driven with a signal layer on top.
- Library modules reorganized to better control privacy and clarify docs.
- Scrollability wrapper element management moved to `ViewportMutable::mutable_viewport`.
- Re-export `bevy_cosmic_edit`, which re-exports `cosmic-text`.

### fixed

- Throttle-able pressing functions now correctly block immediately after the first press.
- `ViewportMutable` elements can now be self-aligned.
- `UiRootable::ui_root` adds `Pickable` component.

# 0.1.1 (2024-07-18)

### added

- Reactive cursor API via `Cursorable` trait.
- Cursor API usage in the `challenge02` example (with issues).
- `calculator` example.
- Convenience methods for `RawHaalkaEl`: `on_event_disableable`, `on_event_mut_disableable`, and `on_event_propagation_stoppable_disableable`.
- `Nameable` convenience trait, for reactively setting the `Name` component.
- `Signal`/`Mutable` utilities `sync`, `sync_neq`, and `flip`.
- `UiRootable` trait, for marking the UI root.

### changed
- Use one shot system in `RawHaalkaEl::on_event_with_system_disableable`.

### fixed
- Static children are now spawned synchronously, eliminating the pop in visible when done asynchronously.

# 0.1.0 (2024-06-27)

### added

- Initial release.
