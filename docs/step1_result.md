# Step 1 Test Result

## Result: FLICKERS ❌

Moving spawn from Update to PostUpdate (while keeping it direct/manual) **causes flicker**.

## What This Confirms

✅ **The issue IS about Update vs PostUpdate schedule timing**

The flicker is NOT caused by:
- ❌ Signal processing (we used direct systems, not signals)
- ❌ child_signal API
- ❌ Jonmo's reactive system

The flicker IS caused by:
- ✅ Spawning entities in PostUpdate instead of Update
- ✅ Something about PostUpdate timing relative to render pipeline

## Key Insight

Even though automated tests showed components are identical at frame boundaries, **PostUpdate spawns still flicker**. This means:

1. **Component state is correct** - all components are present by end of frame
2. **But something in the render pipeline** misses PostUpdate-spawned entities
3. **The timing window** between PostUpdate and extraction/rendering is critical

## Next Steps

Since we've identified that Update vs PostUpdate is the key variable, we can either:

1. **Stop here** - we know the root cause is schedule timing, solution is to spawn in Update
2. **Investigate deeper** - find out WHY PostUpdate causes issues (render world extraction timing, query snapshots, etc.)

## Recommendation

For practical purposes, we've found the root cause:
- **Spawn in Update** = works
- **Spawn in PostUpdate** = flickers

The solution is clear: time-sensitive UI should spawn in Update. Further investigation would be about understanding Bevy internals, not fixing the haalka/jonmo usage pattern.
