use rand::Rng;
use std::f64;
use std::ops::{Add, Sub, Mul, Div};

#[derive(Copy, Clone)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Vector { x: x, y: y, z: z }
    }
}

impl Add for Vector {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl Sub for Vector {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl Mul for Vector {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self::new(self.x * other.x, self.y * other.y, self.z * other.z)
    }
}

impl Mul<f64> for Vector {
    type Output = Self;

    fn mul(self, n: f64) -> Self {
        Self::new(self.x * n, self.y * n, self.z * n)
    }
}

impl Div<f64> for Vector {
    type Output = Self;

    fn div(self, n: f64) -> Self {
        Self::new(self.x / n, self.y / n, self.z / n)
    }
}

impl Vector {
    pub fn norm(self) -> Self {
        self / (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn dot(self, other: Vector) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Vector) -> Self {
        Self::new(self.y * other.z - self.z * other.y,
                  self.z * other.x - self.x * other.z,
                  self.x * other.y - self.y * other.x)
    }
}

#[derive(Copy, Clone)]
pub struct Ray {
    pub o: Vector,
    pub d: Vector,
}

impl Ray {
    pub fn new(o: Vector, d: Vector) -> Self {
        Ray { o: o, d: d }
    }
}

pub enum Refl {
    Diff,
    Spec,
    Refr,
}

pub struct Sphere {
    pub rad: f64,
    pub p: Vector,
    pub e: Vector,
    pub c: Vector,
    pub refl: Refl,
}

impl Sphere {
    pub fn new(rad: f64, p: Vector, e: Vector, c: Vector, refl: Refl) -> Self {
        Sphere {
            rad: rad,
            p: p,
            e: e,
            c: c,
            refl: refl,
        }
    }

    pub fn intersect(&self, ray: Ray) -> Option<f64> {
        let op = self.p - ray.o;
        let eps = 1e-4;
        let b = op.dot(ray.d);
        let det = b * b - op.dot(op) + self.rad * self.rad;
        if det < 0.0 {
            None
        } else {
            let det = det.sqrt();
            let t = b - det;
            if t > eps {
                Some(t)
            } else {
                let t = b + det;
                if t > eps {
                    Some(t)
                } else {
                    None
                }
            }
        }
    }
}

fn min_by_float_key<T: Iterator<Item = U>, U, F: Fn(&U) -> f64>(iter: &mut T, f: F) -> Option<U> {
    iter.fold(None, |min_opt, item| {
            let key = f(&item);
            match min_opt {
                Some((min_key, _)) if min_key < key => min_opt,
                _ => Some((key, item)),
            }
        })
        .map(|(_, min_item)| min_item)
}

fn intersect(scene: &[Sphere], ray: Ray) -> Option<(&Sphere, f64)> {
    let mut hits = scene.iter()
                        .filter_map(|s| s.intersect(ray).map(|t| (s, t)));

    min_by_float_key(&mut hits, |&(_, t)| t)
}

#[allow(non_snake_case)]
pub fn radiance<R: Rng>(scene: &[Sphere], ray: Ray, depth: i32, Xi: &mut R) -> Vector {
    if let Some((obj, t)) = intersect(scene, ray) {
        let x = ray.o + (ray.d * t);
        let n = (x - obj.p).norm();
        let nl = if n.dot(ray.d) < 0.0 {
            n
        } else {
            n * -1.0
        };

        let mut f = obj.c;

        let p = if f.x > f.y && f.x > f.z {
            f.x
        } else if f.y > f.z {
            f.y
        } else {
            f.z
        };

        let depth = depth + 1;
        if depth > 5 {
            if Xi.next_f64() >= p {
                return obj.e;
            }

            f = f / p;
        }

        let next = match obj.refl {
            Refl::Diff => {
                let r1 = 2.0 * f64::consts::PI * Xi.next_f64();
                let r2 = Xi.next_f64();
                let r2s = r2.sqrt();
                let w = nl;
                let u = (if w.x.abs() > 0.1 {
                            Vector::new(0.0, 1.0, 0.0)
                        } else {
                            Vector::new(1.0, 0.0, 0.0)
                        })
                        .cross(w)
                        .norm();
                let v = w.cross(u);
                let d = (u * (r1.cos() * r2s) + (v * (r1.sin() * r2s)) + (w * ((1.0 - r2).sqrt())))
                            .norm();
                let ray = Ray::new(x, d);
                radiance(scene, ray, depth, Xi)
            }

            Refl::Spec => {
                let ray = Ray::new(x, ray.d - (n * (2.0 * n.dot(ray.d))));
                obj.e + f * radiance(scene, ray, depth, Xi)
            }

            Refl::Refr => {
                let refl_ray = Ray::new(x, ray.d - (n * (2.0 * n.dot(ray.d))));
                let into = n.dot(nl) > 0.0;
                let nc = 1.0;
                let nt = 1.5;
                let nnt = if into {
                    nc / nt
                } else {
                    nt / nc
                };
                let ddn = ray.d.dot(nl);
                let cos2t = 1.0 - nnt * nnt * (1.0 - ddn * ddn);
                if cos2t < 0.0 {
                    radiance(scene, refl_ray, depth, Xi)
                } else {
                    let tdir = (ray.d * nnt -
                                n *
                                ((if into {
                                   1.0
                               } else {
                                   -1.0
                               }) * (ddn * nnt + cos2t.sqrt())))
                                   .norm();
                    let a = nt - nc;
                    let b = nt + nc;
                    let R0 = a * a / (b * b);
                    let c = 1.0 -
                            (if into {
                        -ddn
                    } else {
                        tdir.dot(n)
                    });
                    let Re = R0 + (1.0 - R0) * c * c * c * c * c;
                    let Tr = 1.0 - Re;
                    let P = 0.25 * 0.5 * Re;
                    let RP = Re / P;
                    let TP = Tr / (1.0 - P);
                    if depth > 2 {
                        if Xi.next_f64() < P {
                            radiance(scene, refl_ray, depth, Xi) * RP
                        } else {
                            let ray = Ray::new(x, tdir);
                            radiance(scene, ray, depth, Xi) * TP
                        }
                    } else {
                        let ray = Ray::new(x, tdir);
                        radiance(scene, refl_ray, depth, Xi) * Re +
                        radiance(scene, ray, depth, Xi) * Tr
                    }
                }
            }
        };

        obj.e + f * next
    } else {
        Vector::new(0.0, 0.0, 0.0)
    }
}
