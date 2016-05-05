#![feature(const_fn)]
#![feature(mpsc_select)]

extern crate cairo;
extern crate docopt;
extern crate flate2;
extern crate glib;
extern crate gtk;
extern crate hyper;
extern crate iron;
extern crate num_cpus;
extern crate rand;
extern crate rustc_serialize;
extern crate rustless;

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

mod agent;
mod api;
mod gui;
mod radiance;
mod render;
mod scene;

use api::{Sphere, Vector};
use api::Refl::*;
use std::error::Error;
use std::fmt;
use std::io::{Write, stderr};
use std::process;
use std::sync::mpsc::Receiver;
use std::sync::Mutex;

// Scene: radius, position, emission, color, material
static SCENE: &'static [Sphere] = &[Sphere::new(1e5,
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
                                                Diff) /* Lite */];

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

#[derive(RustcDecodable)]
pub struct Args {
    pub cmd_serve: bool,
    pub flag_agent: Vec<String>,
    pub flag_samples: Option<usize>,
    pub flag_threads: Option<usize>,
}

const USAGE: &'static str = "
smallpt, a distributed path tracer.

Usage:
  smallpt [--samples=<n>] [--threads=<n>][--agent=<url>...]
  smallpt serve [--threads=<n>]
  smallpt (-h | --help)
  smallpt --version

Options:
  -h --help      Show this screen.
  --version      Show version.
  --samples=<n>  Number of samples per pixel. Defaults to 1.
  --threads=<n>  Number of threads for parallel rendering. Defaults to the number of CPU cores.
  --agent=<url>  Connect to a remote agent.
";

use docopt::Docopt;

fn main() {
    let args: Args = Docopt::new(USAGE)
                         .and_then(|d| d.decode())
                         .unwrap_or_else(|e| e.exit());

    let run = if args.cmd_serve {
        agent::run
    } else {
        gui::run
    };

    process::exit(run(&args).unwrap_or_else(|err| {
        writeln!(stderr(), "{}", err).unwrap();
        1
    }))
}
