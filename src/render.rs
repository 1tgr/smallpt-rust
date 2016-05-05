use api::{Vector, Ray, Rectangle, Session};
use radiance;
use rand::{Rng, SeedableRng, StdRng};

fn clamp(x: f64) -> f64 {
    x.max(0.0).min(1.0)
}

fn to_int(x: f64) -> u8 {
    (clamp(x).powf(1.0 / 2.2) * 255.0 + 0.5) as u8
}

pub fn render<F: FnMut(Vec<u8>)>(xi: &mut StdRng, session: &Session, rect: Rectangle, mut tx: F) {
    let w = session.width;
    let h = session.height;
    let samps = session.samples;
    let cam = session.camera;
    let cx = Vector::new(w as f64 * 0.5135 / h as f64, 0.0, 0.0);
    let cy = cx.cross(cam.d).norm() * 0.5135;
    let mut acc = vec![Vector::zero(); rect.width * rect.height];
    for samp in 0..samps {
        xi.reseed(&[samp * samp * samp]);
        let mut image = Vec::with_capacity(rect.width * 4 * rect.height);
        for y in rect.top..rect.top + rect.height {
            let offset = ((y - rect.top) * rect.width) as isize - rect.left as isize;
            let y = h - y - 1;
            for x in rect.left..rect.left + rect.width {
                let mut r = &mut acc[(offset + x as isize) as usize];
                for sy in 0..2 {
                    for sx in 0..2 {
                        let r1 = 2.0 * xi.next_f64();
                        let r2 = 2.0 * xi.next_f64();
                        let dx = if r1 < 1.0 {
                            r1.sqrt() - 1.0
                        } else {
                            1.0 - (2.0 - r1).sqrt()
                        };
                        let dy = if r2 < 1.0 {
                            r2.sqrt() - 1.0
                        } else {
                            1.0 - (2.0 - r2).sqrt()
                        };
                        let d = cx *
                                (((sx as f64 + 0.5 + dx) / 2.0 + x as f64) / (w as f64) - 0.5) +
                                cy *
                                (((sy as f64 + 0.5 + dy) / 2.0 + y as f64) / (h as f64) - 0.5) +
                                cam.d;

                        let ray = Ray::new(cam.o + d * 140.0, d.norm());
                        *r += radiance::radiance(&session.scene, ray, 0, xi);
                    }
                }

                let f = 0.25 / (samp + 1) as f64;
                image.push(to_int(clamp(r.x * f)));
                image.push(to_int(clamp(r.y * f)));
                image.push(to_int(clamp(r.z * f)));
                image.push(0);
            }
        }

        tx(image)
    }
}
