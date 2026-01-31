# Step 1 Test: Direct Spawn in PostUpdate

## What This Tests

This version moves the spawn/despawn logic from `Update` (in on_click) to `PostUpdate` (in dedicated systems), while keeping it **direct** (not signal-based).

## Key Changes from Fixed Version

1. **on_click now only toggles component** (runs in Update):
   - Just adds/removes `DropdownShowing`
   - No longer spawns or despawns entities

2. **PostUpdate systems do the actual spawn/despawn**:
   - `spawn_dropdown_options`: Watches for entities with `DropdownShowing` but no children → spawns options
   - `despawn_dropdown_options`: Watches for entities without `DropdownShowing` but with children → despawns options

3. **Still direct spawn** (not signal-based):
   - Uses regular ECS systems, not jonmo signals
   - Manually builds and spawns entities

## What We're Testing

**If this version flickers**: The issue is simply about Update vs PostUpdate schedule timing

**If this version works**: The issue is more subtle - it's not just about the schedule, but about how signals process vs direct systems

## How to Test

Run the example and click between dropdowns:

```bash
cargo run --example dropdown_test_step1
```

**Does it flicker when switching dropdowns?**

- Click dropdown 1 to open it
- Click dropdown 2 to switch
- Watch for one-frame flash/gap/flicker

## Expected Behavior

- Dropdowns should appear/disappear when clicked
- Options should be visible and clickable
- No console errors

The question is: **Does it flicker?**
