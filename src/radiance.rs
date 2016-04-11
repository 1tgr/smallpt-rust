use rand::Rng;
use scene::{Hit, Ray, Refl, Sphere, Vector};
use std::f64;

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
    min_by_float_key(&mut hits, |ref hit| hit.t)
}

#[allow(non_snake_case)]
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

        let nl = if hit.norm.dot(ray.d) < 0.0 {
            hit.norm
        } else {
            hit.norm * -1.0
        };

        let depth = depth + 1;
        let color = scale * hit.color;
        result = result + scale * hit.emit;

        let color = if depth > 5 {
            let p = color.x.max(color.y).max(color.z);
            if Xi.next_f64() >= p {
                continue;
            }

            color / p
        } else {
            color
        };

        match hit.refl {
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
                let ray = Ray::new(hit.pos, d);
                work.push((color, ray, depth));
            }

            Refl::Spec => {
                let ray = Ray::new(hit.pos, ray.d - (hit.norm * (2.0 * hit.norm.dot(ray.d))));
                work.push((color, ray, depth));
            }

            Refl::Refr => {
                let refl_ray = Ray::new(hit.pos, ray.d - (hit.norm * (2.0 * hit.norm.dot(ray.d))));
                let into = hit.norm.dot(nl) > 0.0;
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
                    work.push((color, refl_ray, depth));
                } else {
                    let tdir = (ray.d * nnt -
                                hit.norm *
                                ((if into {
                                   1.0
                               } else {
                                   -1.0
                               }) * (ddn * nnt + cos2t.sqrt())))
                                   .norm();
                    let trans_ray = Ray::new(hit.pos, tdir);
                    let a = nt - nc;
                    let b = nt + nc;
                    let R0 = a * a / (b * b);
                    let c = 1.0 -
                            (if into {
                        -ddn
                    } else {
                        tdir.dot(hit.norm)
                    });
                    let Re = R0 + (1.0 - R0) * c * c * c * c * c;
                    let Tr = 1.0 - Re;
                    let P = 0.25 * 0.5 * Re;
                    let RP = Re / P;
                    let TP = Tr / (1.0 - P);
                    if depth > 2 {
                        if Xi.next_f64() < P {
                            work.push((color * RP, refl_ray, depth));
                        } else {
                            work.push((color * TP, trans_ray, depth));
                        }
                    } else {
                        work.push((color * Re, refl_ray, depth));
                        work.push((color * Tr, trans_ray, depth));
                    }
                }
            }
        }
    }

    result
}
