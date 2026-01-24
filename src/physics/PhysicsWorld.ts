import RAPIER from '@dimforge/rapier2d-compat';
import { GRAVITY_X, GRAVITY_Y, PPM } from '../constants';

export class PhysicsWorld {
  world: RAPIER.World;
  eventQueue: RAPIER.EventQueue;

  constructor() {
    this.world = new RAPIER.World({
      x: GRAVITY_X / PPM,
      y: GRAVITY_Y / PPM,
    });
    this.eventQueue = new RAPIER.EventQueue(true);
  }

  step() {
    this.world.step(this.eventQueue);
  }

  // Convert pixel position to physics (meters)
  toPhysicsX(px: number): number {
    return px / PPM;
  }

  toPhysicsY(py: number): number {
    return py / PPM;
  }

  toPhysicsSize(px: number): number {
    return px / PPM;
  }

  // Convert physics (meters) to pixel position
  toPixelsX(m: number): number {
    return m * PPM;
  }

  toPixelsY(m: number): number {
    return m * PPM;
  }
}
