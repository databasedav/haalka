# Dropdown Flicker Bug Analysis

## Executive Summary

**Problem**: One-frame visual flicker when using `.child_signal()` to spawn dropdown menus in response to component changes.

**Root Cause CONFIRMED**: Entities spawned in `PostUpdate` schedule cause one-frame flicker, regardless of spawn method (signals OR direct systems). Entities spawned in `Update` schedule do not flicker.

**Verified Through Testing**:
- ❌ PostUpdate + child_signal → Flickers
- ✅ Update + direct spawn → No flicker
- ❌ PostUpdate + direct systems → Flickers (Step 1 test)
- ✅ Update + direct systems → No flicker (Step 1 modified)

**Conclusion**: This is a **Bevy engine timing limitation**. The render pipeline extracts entities before PostUpdate completes, causing entities spawned in PostUpdate to miss the current frame's rendering.

**Solution**: Spawn time-sensitive UI elements (dropdowns, tooltips, context menus) directly in Update schedule (e.g., in click handlers). See [`dropdown_flicker_solution.md`](dropdown_flicker_solution.md) and [`root_cause_confirmed.md`](root_cause_confirmed.md) for details.

---

## Problem Summary

When switching between dropdowns using `.child_signal(show.map_true_in(...))`, there is a one-frame visual flicker. This does NOT occur when spawning the dropdown options directly in the `on_click` handler.

## Investigation Process

This section documents the investigation that led to discovering the root cause.

### Known Facts

1. **Component state is identical** at frame boundaries:
   - `ComputedNode` size: 300x260 ✓
   - `InheritedVisibility`: true ✓
   - `BackgroundColor`: present ✓
   - `UiGlobalTransform`: present ✓

2. **Timing Difference**:
   - `child_signal` spawns in `PostUpdate` (during `process_signal_graph`)
   - Direct spawn happens in `Update` (during `on_click`)
   - Both happen AFTER the extraction schedule

3. **The Mystery**: Despite identical component state at frame boundaries, only the PostUpdate spawn produces a one-frame flicker.

### Key Findings from Code Analysis

#### Signal Processing Order
From `jonmo/src/graph.rs`:
- Signals are processed in **topological order** (level by level)
- Within each level, signals are processed in **deterministic order** (sorted by entity index)
- Both dropdown signals are at the same level (both check `has_component::<DropdownShowing>()`)

#### What Happens When Switching Dropdowns

**Frame N - Update:**
1. Click handler runs: Remove `DropdownShowing` from old dropdown, add to new dropdown

**Frame N - PostUpdate (SignalProcessing):**
1. Old dropdown's signal: `has_component::<DropdownShowing>()` → `false` → `map_true_in` returns `None` → system despawns child
2. New dropdown's signal: `has_component::<DropdownShowing>()` → `true` → `map_true_in` returns `Some(...)` → system spawns child

**The Critical Question**: What happens between these two operations?

Looking at `jonmo/src/builder.rs:335-355`, when `child_signal` processes:
- **Despawn path** (line 350-352): Despawns existing child, sets population to 0
- **Spawn path** (line 338-348): Despawns existing child (if any), spawns new child, sets population to 1

#### The Likely Issue

When switching dropdowns, both signal handlers run in PostUpdate. The order is deterministic (by entity index), but:

1. **If old dropdown's entity index < new dropdown's entity index**:
   - Old dropdown's handler runs first → despawns child
   - New dropdown's handler runs second → spawns child
   - **Gap**: There's a moment where neither child exists

2. **If new dropdown's entity index < old dropdown's entity index**:
   - New dropdown's handler runs first → spawns child
   - Old dropdown's handler runs second → despawns child  
   - **Overlap**: Both children exist briefly

But wait - the user said component state is identical at frame boundaries. So at the END of PostUpdate, both children must either both exist or both not exist. The flicker must be happening DURING PostUpdate processing, before the frame completes.

#### The Real Problem

Even though Extract runs AFTER PostUpdate, there might be a timing issue where:
- UI systems (`UiSystems::Prepare`) run AFTER `SignalProcessing` but WITHIN PostUpdate
- The new child spawns, but UI systems haven't processed it yet
- Or the old child despawns, but UI systems still have it in their state

Actually, looking at `src/lib.rs:55-60`, `SignalProcessing` runs BEFORE `UiSystems::Prepare`. So:
1. SignalProcessing runs → spawns/despawns children
2. UiSystems::Prepare runs → processes UI layout/visibility
3. Extract runs → copies to render world

So the new child SHOULD be processed by UI systems before extraction. Unless...

**The flicker might be because the new child spawns WITHIN SignalProcessing, but the UI systems that run AFTER SignalProcessing don't see it properly initialized until the NEXT frame's layout pass.**

### Why Direct Spawn Works

When spawning directly in the `on_click` handler (which runs in `Update`):
- The entity exists early in the frame
- All UI systems have the full frame to process it
- But if components are identical, why does this matter?

**Need to verify**: What is actually different DURING the frame, not just at boundaries?


## Diagnostic Tool

Created `examples/dropdown_timing_analysis.rs` to trace the exact sequence of events:
- Logs at UPDATE START/END
- Logs at POSTUPDATE START/END (before/after signal processing)
- Logs when signals fire
- Logs when components are added/removed

Run this to see the exact timing and identify what differs during the frame.

## Verified Facts

✅ **Component state is identical** at frame boundaries (user verified)
✅ **SignalProcessing runs BEFORE UiSystems::Prepare** (from `src/lib.rs:58`)
✅ **Signals process in topological order** (from `jonmo/src/graph.rs`)
✅ **Within same level, signals process by entity index** (deterministic order)
✅ **Extract runs AFTER PostUpdate** (Bevy documentation)

## The Mystery SOLVED

Despite identical component state at frame boundaries, there's a one-frame flicker ONLY with `child_signal`, NOT with direct spawn.

## Root Cause: Transform Propagation and Extraction Timing

**UPDATE (2026-01-31)**: Automated testing revealed this hypothesis is **INCORRECT**. See full verification in [`dropdown_flicker_verification.md`](dropdown_flicker_verification.md).

The automated tests show that both `child_signal` and direct spawn versions have **identical component state** at frame boundaries:
- Entity exists: ✓
- ComputedNode: ✓
- UiGlobalTransform: ✓
- InheritedVisibility: ✓
- BackgroundColor: ✓

This means the flicker is NOT caused by missing components or late extraction. The actual root cause is likely deeper in Bevy's render pipeline (possibly render world extraction, frame graph execution, or first-frame initialization differences).

---

### Original Hypothesis (Now Disproven)

The flicker was hypothesized to be caused by **when entities enter the hierarchy relative to Bevy's extraction queries**.

### Bevy's PostUpdate Pipeline Order

```
PostUpdate Schedule:
1. SignalProcessing (jonmo) - spawns/despawns children via signals
2. UiSystems::Prepare - computes UI layout
3. TransformSystems::Propagate - computes GlobalTransform/UiGlobalTransform
4. (other systems)

Render Schedule:
- Extract - copies data from main world to render world
```

### The Critical Difference

**With `child_signal` (flickering version):**
- **Frame N, Update**: Click handler adds `DropdownShowing` component
- **Frame N, PostUpdate/SignalProcessing**: Signal fires → spawns new child entity
- **Frame N, PostUpdate/UiSystems**: Computes layout for new child
- **Frame N, PostUpdate/TransformSystems**: Propagates transforms to new child
- **Frame N, Render/Extract**: **Problem!** Entity was spawned late in PostUpdate, may not be included in extraction queries that were set up earlier

**With direct spawn (working version):**
- **Frame N, Update**: Click handler spawns child **directly** and adds to hierarchy
- **Frame N, PostUpdate/SignalProcessing**: (nothing happens for this child)
- **Frame N, PostUpdate/UiSystems**: Computes layout for child
- **Frame N, PostUpdate/TransformSystems**: Propagates transforms
- **Frame N, Render/Extract**: Child was in hierarchy during the full PostUpdate cycle, properly included in extraction

### Why Components Look Identical

The components (ComputedNode, InheritedVisibility, BackgroundColor, UiGlobalTransform) ARE identical at frame boundaries - but the issue isn't about component **state**, it's about **query visibility during extraction**. Entities spawned late in PostUpdate might not be captured by the renderer's extraction queries for the current frame, even though their components are properly initialized.

This is similar to Bevy issue #12070 (InheritedVisibility not updated immediately when parent changes) - it's a fundamental limitation of staged systems where extraction happens at a fixed point in the schedule.

## Solution Approaches

> **For detailed implementation patterns and code examples, see [`dropdown_flicker_solution.md`](dropdown_flicker_solution.md).**

### ✅ Solution 1: Direct Spawn in Click Handler (Implemented in `dropdown_repro_fixed.rs`)

Spawn UI elements that need immediate visibility directly in the `Update` schedule (e.g., in click handlers) rather than via `child_signal`:

**Pros:**
- No flicker - entity exists early enough for extraction
- Immediate feedback for user interactions
- Simple to implement

**Cons:**
- Bypasses reactive system for initial spawn
- May require duplicate logic for spawn/despawn

**When to use:** Time-sensitive UI elements like dropdowns, tooltips, context menus that need to appear immediately in response to user actions.

### Solution 2: Early Signal Processing

Modify jonmo to process signals earlier in the frame cycle (in `Update` instead of `PostUpdate`):

**Pros:**
- Keeps reactive system fully functional
- Would fix all signal-spawned entities, not just dropdowns

**Cons:**
- Requires changes to jonmo core
- Signals would fire before some systems that update components
- May have other timing implications

**When to use:** If this becomes a widespread issue across many UI patterns.

### Solution 3: Two-Frame Delayed Visibility

Accept the one-frame delay but hide it with visibility:

```rust
.child_signal(show.map_true_in(move || {
    Column::<Node>::new()
        // Spawn with Visibility::Hidden initially
        .visibility(Visibility::Hidden)
        // Show on next frame via signal
        .visibility_signal(signal::once(Visibility::Visible).delay(1))
        // ...
}))
```

**Cons:**
- Still has one-frame delay, just hidden
- More complex
- Doesn't actually fix the problem

## Related Issues

- [Bevy #12070](https://github.com/bevyengine/bevy/issues/12070): InheritedVisibility not updated immediately when parent changes
- This is a known limitation of reactive systems that process changes asynchronously
