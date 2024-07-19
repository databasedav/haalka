All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [unreleased]

- none

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
