# Dropdown Flicker Solution Pattern

## Problem Summary

When using `.child_signal()` to spawn UI elements in response to component changes, there can be a one-frame flicker because entities spawned in `PostUpdate` (during signal processing) may not be visible to the renderer's extraction queries in the same frame.

## Root Cause

Bevy's PostUpdate schedule runs systems in this order:
1. `SignalProcessing` (jonmo) - spawns/despawns reactive children
2. `UiSystems::Prepare` - computes UI layout  
3. `TransformSystems::Propagate` - updates GlobalTransform

Entities spawned late in PostUpdate (step 1) may not be captured by the renderer's extraction queries, causing a one-frame delay before they become visible.

## Solution: Direct Spawn for Immediate UI

For UI elements that must appear **immediately** in response to user actions (dropdowns, tooltips, context menus), spawn them directly in the event handler rather than via `child_signal()`.

### Pattern: Direct Spawn in Click Handler

#### Before (Flickering)

```rust
.child_signal(
    show_dropdown
        .map_true_in(|| {
            Column::<Node>::new()
                .with_node(|mut node| {
                    node.position_type = PositionType::Absolute;
                    // ...
                })
                .items(/* dropdown options */)
        })
)
.on_click(move |_: In<_>, world: &mut World| {
    // Just toggle the component - child spawns in PostUpdate
    if world.entity(*dropdown_entity).contains::<ShowDropdown>() {
        world.entity_mut(*dropdown_entity).remove::<ShowDropdown>();
    } else {
        world.entity_mut(*dropdown_entity).insert(ShowDropdown);
    }
})
```

#### After (Fixed)

```rust
.on_click(clone!((dropdown_entity, options) move |_: In<_>, world: &mut World| {
    if world.entity(*dropdown_entity).contains::<ShowDropdown>() {
        // Close dropdown - remove component and despawn child
        world.entity_mut(*dropdown_entity).remove::<ShowDropdown>();
        
        // Find and despawn the options container
        let container = world
            .query_filtered::<Entity, With<DropdownOptionsContainer>>()
            .iter(world)
            .find(|&e| {
                world.get::<DropdownOptionsContainer>(e)
                    .map(|c| c.owner == *dropdown_entity)
                    .unwrap_or(false)
            });
        if let Some(container) = container {
            world.entity_mut(container).despawn_recursive();
        }
    } else {
        // Open dropdown - add component AND spawn child immediately
        world.entity_mut(*dropdown_entity).insert(ShowDropdown);
        
        // Build the options container
        let options_column = Column::<Node>::new()
            .insert(DropdownOptionsContainer { owner: *dropdown_entity })
            .with_node(|mut node| {
                node.position_type = PositionType::Absolute;
                node.width = Val::Percent(100.0);
                node.top = Val::Percent(100.0);
            })
            .items(
                options.iter().enumerate().map(|(i, opt)| {
                    // Create buttons for each option
                    // ...
                })
            );
        
        // Spawn as child immediately (in Update schedule)
        let child_entity = world.spawn_empty().id();
        world.entity_mut(*dropdown_entity).add_child(child_entity);
        options_column.into_builder().spawn_on_entity(world, child_entity).unwrap();
    }
}))
```

### Key Points

1. **Marker component for tracking**: Use a component like `DropdownOptionsContainer { owner: Entity }` to identify which dropdown owns which options container.

2. **Manual spawn/despawn**: Handle both spawn and despawn explicitly in the click handler.

3. **Close other dropdowns first**: When opening a dropdown, close any others by querying for their containers and despawning them.

4. **Runs in Update**: The click handler runs in the `Update` schedule, giving the entity time to be fully processed before extraction.

## When to Use This Pattern

### ✅ Use direct spawn for:
- **Dropdowns** - Must appear immediately when clicked
- **Context menus** - Right-click menus need instant feedback
- **Tooltips** - Should appear without delay on hover
- **Modal dialogs** - User expects immediate response
- **Autocomplete suggestions** - Must feel responsive

### ❌ Use child_signal for:
- **Declarative UI structure** - Static layouts, nested components
- **Conditional rendering** - Show/hide based on signals (where one frame delay is acceptable)
- **Reactive updates** - UI that updates based on data changes
- **Non-interactive elements** - Status indicators, badges, etc.

## Trade-offs

### Pros
- ✅ No visual flicker
- ✅ Immediate user feedback
- ✅ Better perceived performance

### Cons
- ❌ More boilerplate (explicit spawn/despawn logic)
- ❌ Bypasses reactive system
- ❌ Need to manually track relationships (owner component)
- ❌ Potential for logic duplication

## Alternative Approaches

### Option 1: Accept the Delay
For non-critical UI, accept the one-frame delay. Most users won't notice a 16ms (60fps) or 8ms (120fps) delay for non-interactive elements.

### Option 2: Process Signals Earlier (Future Enhancement)
Modify jonmo to process signals in the `Update` schedule instead of `PostUpdate`. This would fix the issue globally but requires careful consideration of signal evaluation order and component change detection timing.

### Option 3: Visibility-Based Hiding (Workaround)
Spawn with `Visibility::Hidden`, then show on the next frame. This still has a delay but can prevent visual artifacts:

```rust
.child_signal(show.map_true_in(move || {
    Column::<Node>::new()
        .visibility(Visibility::Hidden)
        .visibility_signal(
            signal::once(true)
                .delay(1)  // Wait one frame
                .map_true_in(|| Visibility::Visible)
        )
        // ...
}))
```

This approach is rarely better than direct spawn and adds complexity without solving the underlying timing issue.

## Complete Example

See `examples/dropdown_repro_fixed.rs` for a complete working implementation of this pattern, including:
- Multiple dropdowns that can open/close
- Proper cleanup when switching between dropdowns
- Owner tracking via marker components
- Direct spawn in click handlers

## Conclusion

For time-sensitive, user-interactive UI elements, **direct spawning in event handlers** is the recommended pattern to avoid visual flicker. While it requires more boilerplate than `child_signal()`, it provides the immediate feedback users expect from interactive interfaces.

For declarative, non-interactive UI, continue using `child_signal()` to benefit from the reactive system's simplicity and automatic cleanup.
