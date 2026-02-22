# RustFractol-Macroquad

A small fractal explorer (Mandelbrot) written in Rust with **macroquad**.  
Goal: smooth interaction, progressive rendering, and enough precision to zoom very deep.

This project started from a simple need: navigate a fractal **without stutter**, with a progressive render that stays readable while moving.

## What you can do

- Zoom and pan in real time
- Increase/decrease iterations to reveal more detail
- Enjoy progressive rendering (fast preview, HQ when idle)
- Go much deeper with **perturbation-based deep zoom**

## Controls

- `Mouse wheel`: zoom
- `Left click + drag`: pan
- `Up/Down arrows`: increase / decrease iterations
- `R`: reset

## Run it

```bash
cargo run
```

## Dependencies

- `macroquad` for rendering
- `rayon` for parallel computation
- `rug` for deep zoom (high precision)

If `rug` builds slowly or fails, install GMP/MPFR and `m4`.

Examples:

Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install -y m4 libgmp-dev libmpfr-dev
```

Arch
```bash
sudo pacman -S m4 gmp mpfr
```

Fedora
```bash
sudo dnf install m4 gmp-devel mpfr-devel
```

## Precision note

Even with `f64`, you eventually hit a precision limit.  
Perturbation-based deep zoom lets you go **much further** while keeping good performance.

