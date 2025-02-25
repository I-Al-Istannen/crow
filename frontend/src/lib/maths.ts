export function degToRad(deg: number): number {
  return (deg * Math.PI) / 180
}

export function unlerp(min: number, max: number, value: number): number {
  return (value - min) / (max - min)
}

export class Vec2d {
  static ZERO = new Vec2d(0, 0)
  static RIGHT = new Vec2d(1, 0)

  readonly x: number
  readonly y: number

  constructor(x: number, y: number) {
    this.x = x
    this.y = y
  }

  add(v: Vec2d): Vec2d {
    return new Vec2d(this.x + v.x, this.y + v.y)
  }

  sub(v: Vec2d): Vec2d {
    return new Vec2d(this.x - v.x, this.y - v.y)
  }

  to(v: Vec2d): Vec2d {
    return v.sub(this)
  }

  mul(s: number): Vec2d {
    return new Vec2d(this.x * s, this.y * s)
  }

  div(s: number): Vec2d {
    return new Vec2d(this.x / s, this.y / s)
  }

  rotation(to: Vec2d): number {
    // https://stackoverflow.com/a/16544330
    const dot = this.x * to.x + this.y * to.y;
    const det = this.x * to.y - this.y * to.x;
    return Math.atan2(det, dot);
  }

  angle(to: Vec2d): number {
    return Math.abs(this.rotation(to));
  }

  normalize(): Vec2d {
    return this.div(this.length())
  }

  length(): number {
    return Math.sqrt(this.x * this.x + this.y * this.y)
  }
}
