#![feature(mpsc_select)]

extern crate cairo;
extern crate glib;
extern crate gtk;
extern crate rand;

use cairo::{Context, Format, ImageSurface};
use gtk::prelude::*;
use scene::{Vector, Ray, Sphere};
use scene::Refl::*;
use std::cell::RefCell;
use std::env;
use std::error::Error;
use std::fmt;
use std::io::{Write, stderr};
use std::process;
use std::rc::Rc;
use std::result::Result;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver};
use std::thread;

mod radiance;
mod render;
mod scene;

#[derive(Debug)]
struct AppError<'a>(&'a str);

impl<'a> AppError<'a> {
    pub fn new(desc: &'a str) -> Self {
        AppError(desc)
    }
}

impl<'a> fmt::Display for AppError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl<'a> Error for AppError<'a> {
    fn description(&self) -> &str {
        self.0
    }
}

struct WorkIterator<'a, T: 'a>(&'a Mutex<(Receiver<T>, Receiver<()>)>);

impl<'a, T: 'a> WorkIterator<'a, T> {
    pub fn new(rx: &'a Mutex<(Receiver<T>, Receiver<()>)>) -> Self {
        WorkIterator(rx)
    }
}

impl<'a, T: 'a + Send> Iterator for WorkIterator<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        let (ref rx, ref cancel) = *self.0.lock().unwrap();
        select! {
            value = rx.recv() => Some(value.unwrap()),
            _ = cancel.recv() => None
        }
    }
}

macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

fn run() -> Result<i32, Box<Error>> {
    try!(gtk::init().map_err(|()| AppError::new("Failed to initialise GTK")));
    let window = gtk::Window::new(gtk::WindowType::Toplevel);

    window.set_title("smallpt");
    window.set_border_width(10);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    // Scene: radius, position, emission, color, material
    let scene = Arc::new([Sphere::new(1e5,
                                      Vector::new(1e5 + 1.0, 40.8, 81.6),
                                      Vector::zero(),
                                      Vector::new(0.75, 0.25, 0.25),
                                      Diff), // Left
                          Sphere::new(1e5,
                                      Vector::new(-1e5 + 99.0, 40.8, 81.6),
                                      Vector::zero(),
                                      Vector::new(0.25, 0.25, 0.75),
                                      Diff), // Rght
                          Sphere::new(1e5,
                                      Vector::new(50.0, 40.8, 1e5),
                                      Vector::zero(),
                                      Vector::new(0.75, 0.75, 0.75),
                                      Diff), // Back
                          Sphere::new(1e5,
                                      Vector::new(50.0, 40.8, -1e5 + 170.0),
                                      Vector::zero(),
                                      Vector::zero(),
                                      Diff), // Frnt
                          Sphere::new(1e5,
                                      Vector::new(50.0, 1e5, 81.6),
                                      Vector::zero(),
                                      Vector::new(0.75, 0.75, 0.75),
                                      Diff), // Botm
                          Sphere::new(1e5,
                                      Vector::new(50.0, -1e5 + 81.6, 81.6),
                                      Vector::zero(),
                                      Vector::new(0.75, 0.75, 0.75),
                                      Diff), // Top
                          Sphere::new(16.5,
                                      Vector::new(27.0, 16.5, 47.0),
                                      Vector::zero(),
                                      Vector::new(0.999, 0.999, 0.999),
                                      Spec), // Mirr
                          Sphere::new(16.5,
                                      Vector::new(73.0, 16.5, 78.0),
                                      Vector::zero(),
                                      Vector::new(0.999, 0.999, 0.999),
                                      Refr), // Glas
                          Sphere::new(600.0,
                                      Vector::new(50.0, 681.6 - 0.27, 81.6),
                                      Vector::new(12.0, 12.0, 12.0),
                                      Vector::zero(),
                                      Diff) /* Lite */]);

    let w = 1024;
    let h = 768;
    let stride = w * 4;
    let threads = 4;
    let samps = env::args().nth(1).map(|s| s.parse().unwrap()).unwrap_or(1);
    let cam = Ray::new(Vector::new(50.0, 52.0, 295.6),
                       Vector::new(0.0, -0.042612, -1.0).norm());
    let (tx_work, rx_work) = mpsc::channel();
    let (tx_cancel, rx_cancel) = mpsc::channel();
    let (tx_images, rx_images) = mpsc::channel();
    let work = Arc::new(Mutex::new((rx_work, rx_cancel)));

    for _ in 0..threads {
        thread::spawn(clone!(scene, work, tx_images => move || {
            render::render(&*scene,
                           cam,
                           samps,
                           w,
                           h,
                           stride,
                           &mut WorkIterator::new(&work),
                           &tx_images)
        }));
    }

    for y in 0..h {
        tx_work.send(y).unwrap();
    }

    let area = gtk::DrawingArea::new();
    area.set_size_request(w as i32, h as i32);
    window.add(&area);

    let surface = Rc::new(RefCell::new(ImageSurface::create(Format::Rgb24, w as i32, h as i32)));
    area.connect_draw(clone!(surface => move |_, cr| {
        let surface = surface.borrow();
        cr.set_source_surface(&*surface, 0.0, 0.0);
        cr.paint();
        Inhibit(false)
    }));

    gtk::timeout_add(200,
                     clone!(surface => move || {
        while let Ok((y, line)) = rx_images.try_recv() {
            let y = h - y - 1;
            let line_surface = ImageSurface::create_for_data(line.into_boxed_slice(),
                                                             |_| (),
                                                             Format::Rgb24,
                                                             w as i32,
                                                             1,
                                                             stride as i32);
            let surface = surface.borrow();
            let cr = Context::new(&*surface);
            cr.set_source_surface(&line_surface, 0.0, y as f64);
            cr.paint();
            area.queue_draw_area(0, y as i32, w as i32, 1);
        }

        Continue(true)
    }));

    window.show_all();
    gtk::main();
    Ok(0)
}

fn main() {
    process::exit(run().unwrap_or_else(|err| {
        writeln!(stderr(), "{}", err).unwrap();
        1
    }))
}
