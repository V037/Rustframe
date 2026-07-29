// particle_mesh.rs
use eframe::egui::{Color32, Painter, Pos2, Vec2};
use rand::{Rng, thread_rng};

/// A simple 2‑D particle used for a constellation background.
#[derive(Debug, Clone)]
pub struct Particle {
    pub pos: Pos2,
    pub vel: Vec2,
}

impl Particle {
    /// Create a new particle with random position/velocity inside `bounds`.
    fn spawn(bounds: (f32, f32), speed_range: (f32, f32)) -> Self {
        let mut rng = thread_rng();
        let pos = Pos2::new(
            rng.gen_range(0.0..bounds.0),
            rng.gen_range(0.0..bounds.1),
        );
        let angle: f32 = rng.gen_range(0.0..TAU);
        let speed = rng.gen_range(speed_range.0..speed_range.1);
        let vel = Vec2::new(angle.cos(), angle.sin()) * speed;
        Particle { pos, vel }
    }
}

/// A mesh that owns a list of particles and draws them.
pub struct ParticleMesh {
    pub particles: Vec<Particle>,
    line_dist: f32,
    speed_range: (f32, f32),
}

impl Default for ParticleMesh {
    fn default() -> Self {
        ParticleMesh {
            particles: Vec::new(),
            line_dist: 120.0,
            speed_range: (30.0, 70.0),
        }
    }
}

const TAU: f32 = std::f32::consts::PI * 2.0;

impl ParticleMesh {
    /// Initialise with `count` particles inside the given rectangle.
    pub fn new(count: usize, bounds: (f32, f32)) -> Self {
        let mut mesh = Self::default();
        for _ in 0..count {
            mesh.particles.push(Particle::spawn(bounds, mesh.speed_range));
        }
        mesh
    }

    /// Update particle positions. `delta` is the time elapsed since last call.
    pub fn update(&mut self, delta: f32, bounds: (f32, f32)) {
        for p in &mut self.particles {
            p.pos += p.vel * delta;
            // Bounce off bounds
            if p.pos.x <= 0.0 || p.pos.x >= bounds.0 {
                p.vel.x = -p.vel.x;
            }
            if p.pos.y <= 0.0 || p.pos.y >= bounds.1 {
                p.vel.y = -p.vel.y;
            }
        }
    }

    /// Clamp particles inside current bounds on resize.
    pub fn resize(&mut self, bounds: (f32, f32)) {
        for p in &mut self.particles {
            if p.pos.x > bounds.0 {
                p.pos.x = bounds.0;
            }
            if p.pos.y > bounds.1 {
                p.pos.y = bounds.1;
            }
        }
    }

    /// Draw the mesh using egui's painter.
    pub fn draw(&self, painter: &Painter) {
        let len = self.particles.len();
        for i in 0..len {
            for j in (i + 1)..len {
                let a = &self.particles[i];
                let b = &self.particles[j];
                let dist_sq = (a.pos - b.pos).length_sq();
                if dist_sq <= self.line_dist * self.line_dist {
                    let t = 1.0 - dist_sq.sqrt() / self.line_dist;
                    painter.add(egui::Shape::line_segment(
                        [a.pos, b.pos],
                        egui::Stroke {
                            width: 0.5,
                            color: Color32::from_rgba_unmultiplied(200, 200, 255, (t * 120.0) as u8),
                        },
                    ));
                }
            }
        }

        for p in &self.particles {
            painter.circle_filled(p.pos, 2.0, Color32::WHITE);
        }
    }
}