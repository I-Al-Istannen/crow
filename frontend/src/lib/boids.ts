import { Vec2d, degToRad, unlerp } from '@/lib/maths.ts'

const BOID_RADIUS = 100
const BOID_FOV = degToRad(280)
const BOID_BORDER_DISTANCE = 100
const BOID_MOUSE_RADIUS = 125
const BOID_TARGET_SPEED = 150

// Stolen.
const BOID_F_BORDERS = 400
const BOID_F_SEPARATION = 3
const BOID_F_COHESION = 0.8
const BOID_F_ALIGNMENT = 0.5
const BOID_F_SPEED = 0.5
const BOID_F_MOUSE = 100

function canSeeOther(boid: Boid, other: Boid): boolean {
  const delta = other.pos.to(boid.pos)

  if (delta.length() > BOID_RADIUS) {
    return false
  }

  return boid.velocity.angle(delta) < BOID_FOV / 2
}

export class Boid {
  id: string
  pos: Vec2d
  velocity: Vec2d
  width: number
  height: number
  neighbours: Boid[]

  constructor(id: string, position: Vec2d, velocity: Vec2d, width: number, height: number) {
    this.id = id
    this.pos = position
    this.velocity = velocity
    this.width = width
    this.height = height
    this.neighbours = []
  }

  updateNeighbours(all: Boid[]): void {
    this.neighbours = all.filter((boid) => boid !== this && canSeeOther(this, boid))
  }

  forceSeparation(): Vec2d {
    // 1 at BOID_RADIUS, infinite at 0
    let force = Vec2d.ZERO

    for (const neighbour of this.neighbours) {
      const delta = neighbour.pos.to(this.pos)
      const value = delta.length() / BOID_RADIUS
      force = force.add(delta.normalize().mul(1 / value))
    }

    return force
  }

  forceCohesion(): Vec2d {
    if (this.neighbours.length === 0) {
      return Vec2d.ZERO
    }

    let center = Vec2d.ZERO

    for (const neighbour of this.neighbours) {
      center = center.add(neighbour.pos)
    }
    center = center.div(this.neighbours.length)

    return this.pos.to(center)
  }

  forceAlignment(): Vec2d {
    if (this.neighbours.length === 0) {
      return Vec2d.ZERO
    }

    let velocity = Vec2d.ZERO
    for (const neighbour of this.neighbours) {
      velocity = velocity.add(neighbour.velocity)
    }
    velocity = velocity.div(this.neighbours.length)

    return this.velocity.to(velocity)
  }

  forceSpeed(): Vec2d {
    const target = this.velocity.normalize().mul(BOID_TARGET_SPEED)
    return this.velocity.to(target)
  }

  forceBorders(bounds: Vec2d): Vec2d {
    // Force is zero at BORDER_DISTANCE, 1 at the border, grows quadratically
    let force = Vec2d.ZERO

    // left border
    const leftDist = unlerp(BOID_BORDER_DISTANCE, 0, this.pos.x)
    const leftStrength = Math.max(0, leftDist) ** 2
    force = force.add(new Vec2d(leftStrength, 0))

    // right border
    const rightDist = unlerp(bounds.x - BOID_BORDER_DISTANCE, bounds.x, this.pos.x)
    const rightStrength = Math.max(0, rightDist) ** 2
    force = force.add(new Vec2d(-rightStrength, 0))

    // top border
    const topDist = unlerp(BOID_BORDER_DISTANCE, 0, this.pos.y);
    const topStrength = Math.max(0, topDist) ** 2
    force = force.add(new Vec2d(0, topStrength))

    // bottom border
    const bottomDist = unlerp(bounds.y - BOID_BORDER_DISTANCE, bounds.y, this.pos.y)
    const bottomStrength = Math.max(0, bottomDist) ** 2
    force = force.add(new Vec2d(0, -bottomStrength))

    return force
  }

  forceMouse(mouse: Vec2d): Vec2d {
    const delta = mouse.to(this.pos)
    if (delta.length() > BOID_MOUSE_RADIUS) {
      return Vec2d.ZERO
    }

    const value = delta.length() / BOID_MOUSE_RADIUS
    return delta.normalize().mul(1 / value)
  }

  update(bounds: Vec2d, mouse: Vec2d | null, nearby: Boid[], dt: number): void {
    this.updateNeighbours(nearby)

    const separation = this.forceSeparation().mul(BOID_F_SEPARATION)
    const cohesion = this.forceCohesion().mul(BOID_F_COHESION)
    const alignment = this.forceAlignment().mul(BOID_F_ALIGNMENT)
    const speed = this.forceSpeed().mul(BOID_F_SPEED)
    const borders = this.forceBorders(bounds).mul(BOID_F_BORDERS)
    const mouseForce = mouse === null ? Vec2d.ZERO : this.forceMouse(mouse).mul(BOID_F_MOUSE)

    this.velocity = this.velocity
      .add(separation)
      .add(cohesion)
      .add(alignment)
      .add(speed)
      .add(borders)
      .add(mouseForce)

    this.pos = this.pos.add(this.velocity.mul(dt))
  }
}
