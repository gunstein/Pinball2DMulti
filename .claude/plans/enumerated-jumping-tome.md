# Plan: Client-side bot (screensaver mode)

## Overview
Add a robot icon next to the info icon (bottom-left). When toggled on, a simple bot takes over flipper and launcher control. The player can watch the bot play pinball as a screensaver. Click again to regain control.

## Key insight: minimal integration
The `fixedUpdate` in Game.ts already reads 3 booleans from InputManager:
```typescript
this.leftFlipper.fixedUpdate(dt, this.input.leftFlipper);
this.rightFlipper.fixedUpdate(dt, this.input.rightFlipper);
this.launcher.fixedUpdate(dt, this.input.launch);
```
When bot is active, we just substitute these 3 booleans. No controller abstraction needed.

## Coordinates (for bot logic)
- Y-axis: **down is positive** (gravity = +300 px/s²)
- PPM = 500 (pixels per meter). Ball positions from `ball.getPosition()` are in **meters**.
- Flipper Y ≈ 580px = 1.16m
- Left flipper pivot: (110, 580)px = (0.22, 1.16)m
- Right flipper pivot: (290, 580)px = (0.58, 1.16)m
- Board center X: 200px = 0.4m

## Files

### 1. NEW: `client/src/game/ClientBot.ts` (~80-100 lines)

Pure logic, no dependencies on PixiJS or DOM.

```typescript
export class ClientBot {
  private leftCooldown = 0;
  private rightCooldown = 0;
  private launchHoldTime = 0;
  private launchCooldown = 0;

  update(dt: number, balls: BallInfo[]): BotOutput
  reset(): void  // called when bot is toggled off
}

interface BallInfo {
  x: number; y: number;   // physics meters
  vx: number; vy: number;
  inLauncher: boolean;
  inShooterLane: boolean;
}

interface BotOutput {
  leftFlipper: boolean;
  rightFlipper: boolean;
  launch: boolean;
}
```

**Flipper heuristic:**
- Find the lowest active ball (highest y in our coordinate system)
- Define a flip zone: y > 1.0m (≈500px) and vy > 0 (moving down)
- Left flipper: flip if ball x < 0.4m (left half)
- Right flipper: flip if ball x >= 0.4m (right half)
- Cooldown per flipper: ~200ms (prevents vibrating)

**Launcher heuristic:**
- If any ball is in launcher or shooter lane and launch cooldown is 0:
  - Hold launch for 0.3-0.7s (random charge)
  - Release (launcher fires on release edge: `true→false`)
  - Cooldown: 1s after release

### 2. MODIFY: `client/src/game/Game.ts`

Add:
```typescript
private clientBot: ClientBot | null = null;
private botEnabled = false;

setBotEnabled(on: boolean): void {
  this.botEnabled = on;
  if (on) {
    this.clientBot = this.clientBot ?? new ClientBot();
  } else {
    this.clientBot?.reset();
  }
}

isBotEnabled(): boolean {
  return this.botEnabled;
}
```

Change in `fixedUpdate()`:
```typescript
private fixedUpdate(dt: number) {
  let left = this.input.leftFlipper;
  let right = this.input.rightFlipper;
  let launch = this.input.launch;

  if (this.botEnabled && this.clientBot) {
    const ballInfos = this.balls
      .filter(b => b.isActive())
      .map(b => ({
        ...b.getPosition(),
        ...b.getVelocity(),       // note: getVelocity returns {x,y} too
        inLauncher: b.isInLauncher(),
        inShooterLane: b.isInShooterLane(),
      }));
    const cmd = this.clientBot.update(PHYSICS_DT, ballInfos);
    left = cmd.leftFlipper;
    right = cmd.rightFlipper;
    launch = cmd.launch;
  }

  this.leftFlipper.fixedUpdate(dt, left);
  this.rightFlipper.fixedUpdate(dt, right);
  this.launcher.fixedUpdate(dt, launch);
  // ... rest unchanged
}
```

Note: `getPosition()` returns `{x, y}` and `getVelocity()` returns `{x, y}` — but both have the same property names. Use `vx: vel.x, vy: vel.y` to avoid collision:
```typescript
const pos = b.getPosition();
const vel = b.getVelocity();
return { x: pos.x, y: pos.y, vx: vel.x, vy: vel.y, inLauncher: ..., inShooterLane: ... };
```

Also: when bot is active, still send activity heartbeats so server-side bots stay active. The `sendActivityHeartbeat` already checks `this.input.lastActivityTime` — we should also trigger activity when bot is playing. Simplest: set `this.input.lastActivityTime` from Game when bot is active, or just call `this.deepSpaceClient.sendActivity()` periodically when bot is on. Simplest approach: in `sendActivityHeartbeat`, also send if `this.botEnabled`.

### 3. MODIFY: `client/src/main.ts`

Add robot button next to info icon. Same style pattern as `createInfoPanel`.

```typescript
function createBotToggle(game: Game) {
  const btn = document.createElement("button");
  btn.textContent = "\u2699";  // or "⚙" / robot unicode
  // Position: left: 48px (12 + 28 + 8, next to info icon)
  // Same styling as info icon but with active glow state
  
  btn.addEventListener("click", (e) => {
    e.stopPropagation();
    const on = !game.isBotEnabled();
    game.setBotEnabled(on);
    btn.style.borderColor = on ? "rgba(77, 166, 166, 0.8)" : "rgba(77, 166, 166, 0.4)";
    btn.style.color = on ? "rgba(77, 166, 166, 1)" : "rgba(77, 166, 166, 0.7)";
    btn.style.boxShadow = on ? "0 0 8px rgba(77, 166, 166, 0.4)" : "none";
  });
}
```

Unicode for robot icon: `\u{1F916}` (🤖) renders well on all platforms but might look big. Alternative: use a simple gear or play icon. I'll use 🤖 and size it down.

### 4. NEW: `client/tests/clientBot.test.ts`

Test the pure bot logic:
- Ball in left flip zone + moving down → left flipper fires
- Ball in right flip zone + moving down → right flipper fires
- Ball above flip zone → no flippers
- Ball moving up (vy < 0) → no flippers
- Cooldown prevents rapid re-flipping
- Ball in launcher → launch sequence (hold then release)
- No balls → no actions
- `reset()` clears all state

## Files summary
1. `client/src/game/ClientBot.ts` — NEW (~80-100 lines)
2. `client/src/game/Game.ts` — MODIFY (add botEnabled flag, swap inputs in fixedUpdate)
3. `client/src/main.ts` — MODIFY (add robot toggle button)
4. `client/tests/clientBot.test.ts` — NEW (bot logic unit tests)

## Verification
- `cd client && npx vitest run` — all tests pass including new bot tests
- Visual: robot icon visible next to info icon, click toggles bot, bot plays pinball, click again restores human control
- Bot sends activity heartbeats so server-side bots stay active
