use minifb::{Key, Window, WindowOptions};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

const WIDTH: usize = 640;
const HEIGHT: usize = 480;

#[derive(Copy, Clone)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(BufReader::new(file).lines())
}

fn load_obj(path: &str) -> (Vec<Vec3>, Vec<[usize; 3]>) {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();

    if let Ok(lines) = read_lines(path) {
        for line in lines.flatten() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "v" => {
                    if parts.len() >= 4 {
                        let x = parts[1].parse::<f32>().unwrap_or(0.0);
                        let y = parts[2].parse::<f32>().unwrap_or(0.0);
                        let z = parts[3].parse::<f32>().unwrap_or(0.0);
                        vertices.push(Vec3 { x, y, z });
                    }
                }
                "f" => {
                    if parts.len() >= 4 {
                        let v1 = parts[1].split('/').next().unwrap().parse::<usize>().unwrap_or(1) - 1;
                        let v2 = parts[2].split('/').next().unwrap().parse::<usize>().unwrap_or(1) - 1;
                        let v3 = parts[3].split('/').next().unwrap().parse::<usize>().unwrap_or(1) - 1;
                        faces.push([v1, v2, v3]);
                    }
                }
                _ => {}
            }
        }
    }

    (vertices, faces)
}

// ======== ROTACIONES ========

fn rotate_y(v: Vec3, angle: f32) -> Vec3 {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    Vec3 {
        x: v.x * cos_a + v.z * sin_a,
        y: v.y,
        z: -v.x * sin_a + v.z * cos_a,
    }
}

fn rotate_x(v: Vec3, angle: f32) -> Vec3 {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    Vec3 {
        x: v.x,
        y: v.y * cos_a - v.z * sin_a,
        z: v.y * sin_a + v.z * cos_a,
    }
}

// ======== FUNCIONES DE DIBUJO ========

fn draw_pixel(buffer: &mut [u32], x: i32, y: i32, color: u32) {
    if x >= 0 && y >= 0 && (x as usize) < WIDTH && (y as usize) < HEIGHT {
        buffer[y as usize * WIDTH + x as usize] = color;
    }
}

fn draw_line(buffer: &mut [u32], x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
    let mut x0 = x0;
    let mut y0 = y0;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        draw_pixel(buffer, x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

// ======== PROYECCIÓN 3D =======

fn project(v: Vec3, zoom: f32) -> (i32, i32) {
    let x = (WIDTH as f32 / 2.0 + v.x * zoom) as i32;
    let y = (HEIGHT as f32 / 2.0 - v.y * zoom) as i32;
    (x, y)
}

// ======== RELLENADO DE TRIÁNGULOS MEJORADO ========

fn draw_triangle(buffer: &mut [u32], v1: Vec3, v2: Vec3, v3: Vec3, color: u32, zoom: f32) {
    let (x1, y1) = project(v1, zoom);
    let (x2, y2) = project(v2, zoom);
    let (x3, y3) = project(v3, zoom);

    let min_x = x1.min(x2).min(x3).max(0);
    let max_x = x1.max(x2).max(x3).min(WIDTH as i32 - 1);
    let min_y = y1.min(y2).min(y3).max(0);
    let max_y = y1.max(y2).max(y3).min(HEIGHT as i32 - 1);

    if min_x > max_x || min_y > max_y {
        return;
    }

    let x1f = x1 as f32;
    let y1f = y1 as f32;
    let x2f = x2 as f32;
    let y2f = y2 as f32;
    let x3f = x3 as f32;
    let y3f = y3 as f32;

    let area = (x1f * (y2f - y3f) + x2f * (y3f - y1f) + x3f * (y1f - y2f));

    if area.abs() < 0.1 {
        return;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let xf = x as f32;
            let yf = y as f32;

            let w1 = (x2f * (y3f - yf) + x3f * (yf - y2f) + xf * (y2f - y3f)) / area;
            let w2 = (x3f * (y1f - yf) + x1f * (yf - y3f) + xf * (y3f - y1f)) / area;
            let w3 = (x1f * (y2f - yf) + x2f * (yf - y1f) + xf * (y1f - y2f)) / area;

            if w1 >= -0.001 && w2 >= -0.001 && w3 >= -0.001 {
                draw_pixel(buffer, x, y, color);
            }
        }
    }
}

// ======== MAIN ========

fn main() {
    let (vertices, faces) = load_obj("nave.obj");
    println!("Vertices: {}, Caras: {}", vertices.len(), faces.len());

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut window = Window::new("Modelo 3D - Triángulos rellenos", WIDTH, HEIGHT, WindowOptions::default()).unwrap();

    let mut angle_x = 0.0;
    let mut angle_y = 0.0;
    let mut zoom: f32 = 100.0;

    println!("Controles:");
    println!(" ← → : rotar eje Y");
    println!(" ↑ ↓ : rotar eje X");
    println!(" W / S : zoom in/out");
    println!(" ESC : salir");

    while window.is_open() && !window.is_key_down(Key::Escape) {
        buffer.fill(0x222222);
        if window.is_key_down(Key::Left) { angle_y -= 0.05; }
        if window.is_key_down(Key::Right) { angle_y += 0.05; }
        if window.is_key_down(Key::Up) { angle_x -= 0.05; }
        if window.is_key_down(Key::Down) { angle_x += 0.05; }
        if window.is_key_down(Key::W) { zoom *= 1.02; }
        if window.is_key_down(Key::S) { zoom *= 0.98; }
        zoom = zoom.clamp(30.0, 400.0);

        let rotated: Vec<Vec3> = vertices
            .iter()
            .map(|&v| rotate_x(rotate_y(v, angle_y), angle_x))
            .collect();

        for face in &faces {
            let v1 = rotated[face[0]];
            let v2 = rotated[face[1]];
            let v3 = rotated[face[2]];

            draw_triangle(&mut buffer, v1, v2, v3, 0x0077FF, zoom);

            let (x1, y1) = project(v1, zoom);
            let (x2, y2) = project(v2, zoom);
            let (x3, y3) = project(v3, zoom);
            draw_line(&mut buffer, x1, y1, x2, y2, 0xFFFFFF);
            draw_line(&mut buffer, x2, y2, x3, y3, 0xFFFFFF);
            draw_line(&mut buffer, x3, y3, x1, y1, 0xFFFFFF);
        }

        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}