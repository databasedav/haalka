# Verification of Root Cause Analysis

## Automated Test Results

Created automated versions of both dropdown examples that:
- Auto-click to open dropdown at frame 10
- Log component state at `First` and `Last` schedule points
- Exit at frame 15

### Key Finding: BOTH versions show identical behavior!

**SIGNAL_CHILD version (child_signal spawning in PostUpdate):**
```
Frame 10: show signal fired for 19v0: true
Frame 10: map_true_in: creating Column for 19v0
Frame 10 LAST: options entity exists=true ComputedNode=true UiGlobalTransform=true InheritedVisibility=true BackgroundColor=true
Frame 11 FIRST: options entity exists=true ComputedNode=true UiGlobalTransform=true InheritedVisibility=true BackgroundColor=true
```

**DIRECT_SPAWN version (direct spawning in Update):**
```
Frame 10: UPDATE spawning options container for dropdown 19v0
Frame 10: UPDATE spawned options container 143v0 for dropdown 19v0
Frame 10 LAST: options entity exists=true ComputedNode=true UiGlobalTransform=true InheritedVisibility=true BackgroundColor=true
Frame 11 FIRST: options entity exists=true ComputedNode=true UiGlobalTransform=true InheritedVisibility=true BackgroundColor=true
```

## Analysis: The Initial Hypothesis Was INCORRECT

The automated tests show that **both versions have all components present by the end of Frame 10**, including:
- Entity exists ✓
- ComputedNode ✓ 
- UiGlobalTransform ✓
- InheritedVisibility ✓
- BackgroundColor ✓

This means my hypothesis about **extraction query timing** was **WRONG**. The components ARE present in the same frame, even with `child_signal` spawning in PostUpdate.

## What's Actually Different?

Since the automated test doesn't show a difference in component presence, the flicker must be caused by something else:

### Possible Real Causes:

1. **Render pipeline caching**: The renderer might cache queries or buffers at a specific point that misses late-spawned entities even if they have all components.

2. **Extract system ordering**: Specific extract systems might run at different sub-stages within PostUpdate, and entities spawned during SignalProcessing might miss some of these.

3. **Visual artifact vs missing entity**: The flicker might not be about the entity being invisible, but about z-ordering, occlusion, or some other visual artifact during the transition.

4. **First-frame initialization**: There might be a difference in how entities are initialized in their first frame vs subsequent frames, unrelated to component presence.

## Next Steps to Verify

To find the ACTUAL root cause, we need to:

1. **Check render world extraction**: Log what entities are present in the render world, not just the main world
2. **Frame-by-frame visual inspection**: Capture what's actually rendered to screen each frame
3. **Bevy render pipeline investigation**: Look at when exactly extraction happens relative to PostUpdate stages
4. **Test with Bevy's frame graph debugging**: Use Bevy's built-in debugging to see render passes

## Conclusion

The original analysis was **partially correct** about timing being the issue, but **incorrect** about the specific mechanism. Components ARE present in the same frame, so the issue must be deeper in Bevy's render pipeline, not in the main world ECS state.

The direct spawn workaround still works (no flicker observed in practice), but for a different reason than initially hypothesized.
