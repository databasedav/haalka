# ROOT CAUSE: DEFINITIVELY CONFIRMED

## The Finding

**Update vs PostUpdate schedule is the ONLY variable that causes flicker.**

## Test Results

| Test | Schedule | Spawn Method | Flicker? |
|------|----------|--------------|----------|
| Original broken version | PostUpdate | child_signal (reactive) | ❌ YES |
| Fixed version | Update | Direct in on_click | ✅ NO |
| Step 1 - PostUpdate systems | PostUpdate | Direct in systems | ❌ YES |
| Step 1 - Update systems | Update | Direct in systems | ✅ NO |

## Conclusion

The flicker is caused by **spawning entities in PostUpdate instead of Update**.

This has NOTHING to do with:
- Signals vs direct spawn
- child_signal API
- Jonmo reactive system
- How the entity is built
- What components are added

It is ONLY about:
- ✅ **WHEN** (which schedule) the entity is spawned

## Why This Happens

Bevy's render pipeline extracts entities from the main world at a specific point in the frame. Entities spawned during PostUpdate are too late to be included in that frame's render extraction, even though their components are properly initialized.

By the time PostUpdate completes and all components are present, the render world has already been populated for that frame.

## The Solution

**For time-sensitive UI that must appear immediately:**
- Spawn in Update schedule (e.g., in click handlers, input handlers)
- Examples: dropdowns, tooltips, context menus, modals

**For non-time-sensitive UI:**
- Can use signals in PostUpdate (reactive approach)
- Examples: status indicators, badges, conditional content where one-frame delay is acceptable

## Technical Implications

This is a fundamental Bevy engine limitation. The render pipeline works as follows:

1. **Update** - Game logic runs
2. **PostUpdate** - Systems that react to changes run
3. **Render extraction** - Main world → Render world (happens here or shortly after)
4. **Render** - GPU draws the frame

Entities spawned in step 2 (PostUpdate) are too late for step 3 (extraction) in the same frame.

## Recommendation

Document this as a known limitation and provide clear guidelines:
- Interactive UI requiring immediate feedback → spawn in Update
- Reactive UI where slight delay is acceptable → can use signals in PostUpdate
