# Pinball2DMulti

En "cozy" 2D pinball i nettleseren med transparent brett og et delt "deep-space" bak. Naar en ball rommer via escape-slotten, fortsetter den i deep-space og kan komme inn igjen (eller inn hos andre spillere naar vi kobler paa server).

## Status

- Lokal pinball (1 spiller) fungerer: Rapier2D + PixiJS (flippere, launcher, bumpere, telleverk).
- Deep-space fungerer lokalt:
  - Sfaere-modell: baller som beveger seg paa great circles paa en enhetssfaere (`SphereDeepSpace`)
  - Portal-hit via dot-test, reroute-failsafe for "cozy return"
- Mock multiplayer: mange portaler/spillere genereres lokalt (`MockWorld` + `PortalPlacement`).

Neste: server-authoritative deep-space + ekte multiplayer.

## Tech stack

- **PixiJS** - rendering
- **Rapier2D** - WASM physics
- **TypeScript**
- **Vite**
- **Vitest**

## Kjor lokalt

```bash
npm install
npm run dev
```

Aapne URL-en Vite viser (typisk `http://localhost:3000`).

## Kontroller

| Tast | Handling |
|------|----------|
| Venstrepil | Venstre flipper |
| Hoyrepil | Hoyre flipper |
| Space (hold) | Lad launcher |
| Space (slipp) | Skyt ball |

## Scripts

| Kommando | Beskrivelse |
|----------|-------------|
| `npm run dev` | Start dev-server |
| `npm run build` | Typecheck + build |
| `npm run preview` | Preview av build |
| `npm test` | Kjor tester (Vitest) |
| `npm run format` | Prettier |

## Kode-struktur

```
src/
├── main.ts
├── constants.ts
├── game/
│   ├── Game.ts              Fixed timestep + render
│   └── InputManager.ts      Keyboard input
├── board/
│   ├── BoardGeometry.ts     Data-driven brett (segmenter/definisjoner)
│   ├── Board.ts             Vegger/colliders
│   ├── Ball.ts              Rapier-ball + Pixi graphics
│   ├── Flipper.ts           Kinematisk flipper (pivot + collider offset)
│   ├── flipperLogic.ts      Ren flipperlogikk (testbar)
│   ├── Launcher.ts          Launcher visuals
│   ├── launcherLogic.ts     Ren launcher state machine (testbar)
│   └── Pin.ts               Bumper/pin med hit glow
├── physics/
│   └── PhysicsWorld.ts      Rapier wrapper + unit conversions
├── layers/
│   ├── BoardLayer.ts
│   ├── UILayer.ts
│   └── SphereDeepSpaceLayer.ts  Deep-space "neighborhood disk" view
└── shared/
    ├── SphereDeepSpace.ts   Sfaere-sim (ren logikk)
    ├── sphere.ts            Fibonacci sphere + PortalPlacement
    ├── vec3.ts              3D vektor-matte
    ├── MockWorld.ts         Mock spillerliste/portaler
    └── types.ts             Delt kontrakt (Player, SpaceBall3D, config)
```

## Arkitektur

- Pinball-sim er klient-lokal og kjorer med fixed timestep (120 Hz).
- Deep-space er en ren logikk-modul (`SphereDeepSpace`) uten Pixi/Rapier.
- **Escape pipeline:**
  1. Ball rommer gjennom escape-slot -> snapshot (vx/vy)
  2. Snapshot mappes inn i deep-space (sfaere)
  3. Naar ball "treffer portal" -> capture event
  4. Capture hos deg -> ball injiseres tilbake paa brettet
- **Ytelse:** Poolede Graphics-objekter for deep-space rendering (ingen per-frame clear/redraw). Ball-pool for aa unngaa Rapier/Pixi allokerings-spikes. Zero-alloc tick-loop i `SphereDeepSpace`.

## Test

Ren logikk testes med Vitest (flipper/launcher/vec3/sphere/deep-space). Rapier-integrasjon testes i `tests/physics.test.ts`.

## Roadmap: Server (planned)

Maal: Server authoritative for deep-space + portalplassering + reroute/ownership. Klient authoritative for pinball (Rapier).
