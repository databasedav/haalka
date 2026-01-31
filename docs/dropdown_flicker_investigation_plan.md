# Dropdown Flicker Investigation Plan

## What We Know

1. ✅ **Both versions have identical components at frame boundaries**
   - Entity exists, ComputedNode, UiGlobalTransform, InheritedVisibility, BackgroundColor all present
   - This was verified with automated tests

2. ✅ **Direct spawn in Update eliminates flicker**
   - Empirically confirmed to work

3. ✅ **child_signal spawn in PostUpdate causes flicker**
   - Empirically confirmed to flicker

4. ❌ **Original hypothesis disproven**
   - It's NOT about component presence in main world
   - It's NOT about extraction query timing (as originally theorized)

## What We Need to Find

**The actual mechanism that causes the visual difference between spawning in Update vs PostUpdate**

## Investigation Plan

### Phase 1: Understand Bevy's Render Pipeline (Research)

**Goal**: Understand when and how UI entities are extracted and rendered

**Tasks**:
1. Read Bevy 0.17 UI rendering source code
   - When does UI extraction happen relative to PostUpdate?
   - What queries do UI extract systems use?
   - Are there any entity age checks or "new entity" special handling?

2. Understand Bevy's frame graph
   - What render passes exist for UI?
   - What order do they execute in?
   - When is the render world updated from main world?

3. Check if there are any known issues
   - Search Bevy issue tracker for similar problems
   - Look for issues about "UI flicker", "entity spawn timing", "extraction timing"

**Deliverable**: Document summarizing how Bevy's UI rendering works

### Phase 2: Add Render World Logging (Instrumentation)

**Goal**: See what's actually in the render world during the problematic frame

**Tasks**:
1. Add logging to track render world state
   - Create a system that runs in Extract schedule
   - Log what UI entities exist in render world
   - Compare render world entities to main world entities

2. Add frame-by-frame tracking
   - Log when entities enter render world
   - Track entity IDs from main world to render world
   - See if there's a one-frame delay for PostUpdate-spawned entities

3. Compare both versions side-by-side
   - Run child_signal version with render logging
   - Run direct_spawn version with render logging
   - Find the difference

**Deliverable**: Logs showing render world state for both versions

### Phase 3: Trace Entity Lifecycle (Deep Dive)

**Goal**: Follow a single entity from spawn to render

**Tasks**:
1. Add comprehensive tracing for one specific entity
   - When it's spawned in main world
   - When components are added
   - When it appears in extract queries
   - When it appears in render world
   - When it's actually rendered

2. Compare lifecycle between versions
   - Track the same entity in child_signal version
   - Track the same entity in direct_spawn version
   - Identify exactly where they diverge

3. Check intermediate states
   - Not just First and Last, but also PostUpdate sub-stages
   - Check right before/after SignalProcessing
   - Check right before/after UI system sets
   - Check right before/after Transform propagation

**Deliverable**: Timeline diagram showing entity lifecycle differences

### Phase 4: Test Hypotheses (Experiments)

Based on what we learn, test specific hypotheses:

**Hypothesis A: Extract Systems Run Before PostUpdate Completes**
- Test: Add logging in extract systems with timestamps
- Expected: If true, extract systems run before PostUpdate finishes, missing late-spawned entities
- Experiment: Force extract to run later, see if flicker disappears

**Hypothesis B: Render World Updates Once Per Frame**
- Test: Check if render world is updated incrementally or all-at-once
- Expected: If updated once, entities spawned after that point won't render until next frame
- Experiment: N/A (if true, this is Bevy's design)

**Hypothesis C: UI Extract Systems Have Entity Age Requirements**
- Test: Search Bevy source for "Changed" filters or similar in UI extract systems
- Expected: If true, newly-spawned entities might be filtered out
- Experiment: Check if removing Changed filters fixes issue

**Hypothesis D: First-Frame Initialization Special Case**
- Test: Spawn entity in frame N, check if it renders same as entity spawned in frame N-1
- Expected: If true, entities need a "settling" frame for some systems to process them
- Experiment: Pre-spawn invisible entities, then make visible

**Hypothesis E: Parent-Child Synchronization Issue**
- Test: Check if parent transform needs a frame to propagate to children
- Expected: If true, children spawned late might not get proper transforms until next frame
- Experiment: Manually set transforms on spawn, see if that helps

**Deliverable**: Test results for each hypothesis with evidence

### Phase 5: Root Cause Confirmation

**Goal**: Definitively identify the root cause

**Tasks**:
1. Based on Phase 4 results, identify the most likely cause
2. Create a minimal reproduction
   - Simplest possible example showing the issue
   - Remove all dropdown logic, just spawn a colored box
3. Verify the cause is general, not dropdown-specific
4. Document the exact mechanism

**Deliverable**: 
- Minimal reproduction
- Clear explanation of root cause
- Evidence supporting the conclusion

### Phase 6: Validate Solution & Document

**Goal**: Confirm our solution and document best practices

**Tasks**:
1. Verify direct spawn solution works for the right reasons
   - Not just empirically, but mechanistically
   - Understand WHY it works

2. Document the proper fix
   - Update analysis document with real root cause
   - Update solution document with mechanistic explanation
   - Add warnings/caveats if needed

3. Consider if there's a better solution
   - Could jonmo be modified to avoid this?
   - Could Bevy be modified to fix this?
   - Are there other workarounds?

**Deliverable**:
- Updated documentation with accurate root cause
- Best practices for haalka/jonmo users
- Potential upstream fixes to propose

## Success Criteria

We'll know we've succeeded when we can:

1. ✅ Explain exactly WHY child_signal causes flicker
2. ✅ Explain exactly WHY direct spawn fixes it
3. ✅ Provide evidence (logs, traces, or code) supporting our explanation
4. ✅ Create a minimal reproduction that anyone can verify
5. ✅ Document the solution with accurate technical details

## Resources Needed

- Bevy 0.17 source code (already have)
- Bevy documentation on rendering pipeline
- Access to run and modify the automated examples
- Time to instrument and trace execution
- Possibly: Bevy render graph debugging tools

## Timeline Estimate

- Phase 1: 1-2 hours (reading and research)
- Phase 2: 2-3 hours (instrumentation and logging)
- Phase 3: 2-3 hours (detailed tracing)
- Phase 4: 2-4 hours (hypothesis testing)
- Phase 5: 1-2 hours (confirmation)
- Phase 6: 1 hour (documentation)

Total: ~9-15 hours of investigation

## Next Steps

Start with **Phase 1** - understand Bevy's render pipeline by:
1. Reading Bevy UI extract systems source
2. Understanding when Extract schedule runs
3. Checking for known issues

Ready to proceed?
