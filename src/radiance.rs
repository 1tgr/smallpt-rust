#![allow(non_snake_case)]
use api::{Ray, Refl, Sphere, Vector};
use rand::Rng;
use scene::Hit;
use std::f64;

fn diffuse<R: Rng>(_depth: i32,
                   Xi: &mut R,
                   pos: Vector,
                   dir: Vector,
                   norm: Vector,
                   cast: &mut FnMut(f64, Ray)) {
    let nl = if norm.dot(dir) < 0.0 {
        norm
    } else {
        norm * -1.0
    };

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
    let d = (u * r1.cos() * r2s + v * r1.sin() * r2s + w * (1.0 - r2).sqrt()).norm();
    cast(1.0, Ray::new(pos, d))
}

fn specular<R: Rng>(_depth: i32,
                    _Xi: &mut R,
                    pos: Vector,
                    dir: Vector,
                    norm: Vector,
                    cast: &mut FnMut(f64, Ray)) {
    cast(1.0, Ray::new(pos, dir - norm * 2.0 * norm.dot(dir)))
}

fn glossy_refraction<R: Rng>(depth: i32,
                             Xi: &mut R,
                             pos: Vector,
                             dir: Vector,
                             norm: Vector,
                             cast: &mut FnMut(f64, Ray)) {
    let refl_ray = Ray::new(pos, dir - norm * 2.0 * norm.dot(dir));

    let nl = if norm.dot(dir) < 0.0 {
        norm
    } else {
        norm * -1.0
    };

    let into = norm.dot(nl) > 0.0;
    let nc = 1.0;
    let nt = 1.5;
    let nnt = if into {
        nc / nt
    } else {
        nt / nc
    };
    let ddn = dir.dot(nl);
    let cos2t = 1.0 - nnt * nnt * (1.0 - ddn * ddn);
    if cos2t < 0.0 {
        cast(1.0, refl_ray);
    } else {
        let tdir = (dir * nnt -
                    norm *
                    ((if into {
                       1.0
                   } else {
                       -1.0
                   }) * (ddn * nnt + cos2t.sqrt())))
                       .norm();
        let trans_ray = Ray::new(pos, tdir);
        let a = nt - nc;
        let b = nt + nc;
        let R0 = a * a / (b * b);
        let c = 1.0 -
                (if into {
            -ddn
        } else {
            tdir.dot(norm)
        });
        let Re = R0 + (1.0 - R0) * c * c * c * c * c;
        let Tr = 1.0 - Re;
        let P = 0.25 * 0.5 * Re;
        let RP = Re / P;
        let TP = Tr / (1.0 - P);
        if depth > 2 {
            if Xi.next_f64() < P {
                cast(RP, refl_ray);
            } else {
                cast(TP, trans_ray);
            }
        } else {
            cast(Re, refl_ray);
            cast(Tr, trans_ray);
        }
    }
}

fn _refraction<R: Rng>(_depth: i32,
                       _Xi: &mut R,
                       pos: Vector,
                       dir: Vector,
                       norm: Vector,
                       cast: &mut FnMut(f64, Ray)) {
    let nl = if norm.dot(dir) < 0.0 {
        norm
    } else {
        norm * -1.0
    };

    let into = norm.dot(nl) > 0.0;
    let nc = 1.0;
    let nt = 1.5;
    let nnt = if into {
        nc / nt
    } else {
        nt / nc
    };
    let ddn = dir.dot(nl);
    let cos2t = 1.0 - nnt * nnt * (1.0 - ddn * ddn);
    if cos2t < 0.0 {
        cast(1.0, Ray::new(pos, dir - norm * 2.0 * norm.dot(dir)));
    } else {
        let tdir = (dir * nnt -
                    norm *
                    ((if into {
                       1.0
                   } else {
                       -1.0
                   }) * (ddn * nnt + cos2t.sqrt())))
                       .norm();
        cast(1.0, Ray::new(pos, tdir));
    }
}

fn material<R: Rng>(refl: &Refl,
                    depth: i32,
                    Xi: &mut R,
                    pos: Vector,
                    norm: Vector,
                    dir: Vector,
                    cast: &mut FnMut(f64, Ray)) {
    match refl {
        &Refl::Diff => diffuse(depth, Xi, pos, norm, dir, cast),
        &Refl::Spec => specular(depth, Xi, pos, norm, dir, cast),
        &Refl::Refr => glossy_refraction(depth, Xi, pos, norm, dir, cast),
        &Refl::Mix(factor, ref r1, ref r2) => {
            material(&*r1,
                     depth,
                     Xi,
                     pos,
                     norm,
                     dir,
                     &mut |scale, ray| cast((1.0 - factor) * scale, ray));
            material(&*r2,
                     depth,
                     Xi,
                     pos,
                     norm,
                     dir,
                     &mut |scale, ray| cast(factor * scale, ray));
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

fn intersect(scene: &[Sphere], ray: Ray) -> Option<Hit> {
    let mut hits = scene.iter().filter_map(|s| s.intersect(ray));
    min_by_float_key(&mut hits, |&(t, _)| t).map(|(_, hit)| hit)
}

pub fn radiance<R: Rng>(scene: &[Sphere], ray: Ray, depth: i32, Xi: &mut R) -> Vector {
    let mut result = Vector::zero();
    let mut work = Vec::new();
    work.push((Vector::new(1.0, 1.0, 1.0), ray, depth));
    while let Some((scale, ray, depth)) = work.pop() {
        let hit = match intersect(scene, ray) {
            Some(hit) => hit,
            None => {
                continue;
            }
        };

        let depth = depth + 1;
        let color = scale * hit.color;
        result += scale * hit.emit;

        let color = if depth > 5 {
            let p = color.x.max(color.y).max(color.z);
            if Xi.next_f64() >= p {
                continue;
            }

            color / p
        } else {
            color
        };

        material(&hit.refl,
                 depth,
                 Xi,
                 hit.pos,
                 ray.d,
                 hit.norm,
                 &mut |scale, ray| work.push((color * scale, ray, depth)));
    }

    result
}
