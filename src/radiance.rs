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
    if let Some(hit) = intersect(scene, ray) {
        let x = hit.pos;
        let n = hit.norm;
        let nl = if n.dot(ray.d) < 0.0 {
            n
        } else {
            n * -1.0
        };

        let depth = depth + 1;
        let color = hit.color;

        let color = if depth > 5 {
            let p = color.x.max(color.y).max(color.z);
            if Xi.next_f64() >= p {
                return hit.emit;
            }

            color / p
        } else {
            color
        };

        let next = match hit.refl {
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
                radiance(scene, ray, depth, Xi)
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

        hit.emit + color * next
    } else {
        Vector::zero()
    }
}
