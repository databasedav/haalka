# Iterative Investigation Plan: Finding the Breaking Change

## Approach

Start with the **working** version (direct spawn in Update) and incrementally change it to match the **broken** version (child_signal in PostUpdate), testing after each change to find exactly what causes the flicker.

## Current State

**Working version** (`dropdown_repro_fixed.rs`):
- Spawns dropdown options **directly in on_click handler**
- on_click runs in **Update schedule**
- Manual spawn: `world.spawn_empty()` + `add_child()` + `spawn_on_entity()`
- Manual despawn: query for container + `despawn()`

**Broken version** (`dropdown_repro.rs`):
- Spawns dropdown options **via child_signal**
- Signal processes in **PostUpdate schedule** (via jonmo's SignalProcessing)
- Reactive spawn: `.child_signal(show.map_true_in(|| ...))`
- Automatic despawn: signal returns None

## Iteration Steps

### Step 0: Baseline (Current Fixed Version)
✅ **Working** - No flicker

**Test**: Does it flicker?
**User answers**: No

---

### Step 1: Move spawn to PostUpdate (but keep it direct)

**Change**: Move the spawn logic from Update to PostUpdate, but still spawn directly (not via signal)

```rust
// Remove spawn from on_click (Update)
.on_click(move |_: In<_>, world: &mut World| {
    // Just toggle component, don't spawn
    if showing { remove component }
    else { insert component }
})

// Add system in PostUpdate that watches for component and spawns
fn spawn_dropdown_options(world: &mut World) {
    for entity with DropdownShowing {
        if !already_has_options_child {
            // spawn options directly here
        }
    }
}
```

**Hypothesis**: If this still works, the issue is NOT about Update vs PostUpdate scheduling
**If it flickers**: The issue IS about Update vs PostUpdate scheduling

**Test**: Does it flicker?
**User answers**: ???

---

### Step 2: Use world.spawn() instead of spawn_on_entity()

**Change**: Switch from `spawn_on_entity()` to regular `world.spawn()`

```rust
// Before:
let child_entity = world.spawn_empty().id();
world.entity_mut(*lazy_entity).add_child(child_entity);
options_column.into_builder().spawn_on_entity(world, child_entity).unwrap();

// After:
let child_entity = options_column.into_builder().spawn(world).unwrap();
world.entity_mut(*lazy_entity).add_child(child_entity);
```

**Hypothesis**: If this causes issues, the order of spawn vs add_child matters

**Test**: Does it flicker?
**User answers**: ???

---

### Step 3: Delay add_child to next frame

**Change**: Spawn the entity, but delay adding it as a child until next frame

```rust
// Frame N: spawn entity
let child_id = spawn_options();
world.entity_mut(child_id).insert(PendingChildOf { parent: dropdown_entity });

// Frame N+1: add as child (via system)
fn attach_pending_children(world: &mut World) {
    for (entity, pending) in query {
        world.entity_mut(pending.parent).add_child(entity);
    }
}
```

**Hypothesis**: If this flickers, parent-child relationship timing matters

**Test**: Does it flicker?
**User answers**: ???

---

### Step 4: Use a signal (but not child_signal)

**Change**: Keep direct spawn, but use a signal to trigger it instead of component check

```rust
// Use signal to detect component
let signal = signal::from_entity(dropdown)
    .has_component::<DropdownShowing>()
    .dedupe();

// But still spawn directly in a reactive system
signal.for_each(move |showing| {
    if showing {
        // spawn directly here
    }
});
```

**Hypothesis**: If this flickers, signal processing itself causes the issue

**Test**: Does it flicker?
**User answers**: ???

---

### Step 5: Use child_signal but spawn in Update

**Change**: Use `.child_signal()` but somehow force it to process in Update instead of PostUpdate

**Note**: This might require modifying jonmo or using a custom signal processor

**Hypothesis**: If this works, child_signal is fine but PostUpdate timing is the issue

**Test**: Does it flicker?
**User answers**: ???

---

### Step 6: Use child_signal exactly as broken version

**Change**: Switch to the exact broken version implementation

```rust
.child_signal(
    show.map_true_in(move || {
        Column::<Node>::new()
            // ... exact same as broken version
    })
)
```

**Test**: Does it flicker?
**User answers**: Should be YES (confirming we've reproduced the issue)

---

## Analysis Matrix

After all steps, we'll have a table like:

| Step | Description | Update/PostUpdate | Direct/Signal | Flicker? |
|------|-------------|-------------------|---------------|----------|
| 0 | Fixed version | Update | Direct | No ✅ |
| 1 | Direct spawn in PostUpdate | PostUpdate | Direct | ??? |
| 2 | Different spawn order | PostUpdate | Direct | ??? |
| 3 | Delayed parent-child | PostUpdate | Direct | ??? |
| 4 | Signal-triggered spawn | PostUpdate | Signal | ??? |
| 5 | child_signal in Update | Update | child_signal | ??? |
| 6 | Broken version | PostUpdate | child_signal | Yes ❌ |

## Finding the Root Cause

Based on the results:

- **If Step 1 flickers**: Issue is Update vs PostUpdate schedule
- **If Step 2 flickers**: Issue is spawn method or order
- **If Step 3 flickers**: Issue is parent-child timing
- **If Step 4 flickers**: Issue is signal processing
- **If Step 5 works**: Issue is child_signal + PostUpdate combination
- **If Step 6 flickers**: We've confirmed reproduction

The **first step that flickers** tells us the minimal change that breaks it!

## Implementation Plan

For each step:

1. **Create a new example file**: `dropdown_test_step1.rs`, `dropdown_test_step2.rs`, etc.
2. **Make the specific change** described above
3. **Ask user to test**: "Does this version flicker?"
4. **Record result** and proceed based on outcome
5. **Skip unnecessary steps** if we find the breaking point early

## Next Action

Create `dropdown_test_step1.rs` - move spawn to PostUpdate but keep it direct (not signal-based).

Ready to start?
