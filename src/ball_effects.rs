//! Wektorowe efekty piłki: brak tekstur, zależności SVG ani zegara renderowania.
use macroquad::prelude::*;
use std::collections::VecDeque;

// Oświetlenie liczone w przestrzeni ekranu: odbłysk nie obraca się razem ze wzorem.
fn sphere_color(base: Vec3, point: Vec2, brightness: f32, alpha: f32) -> Color {
    let normal = vec3(
        point.x,
        point.y,
        (1.0 - point.length_squared()).max(0.0).sqrt(),
    );
    let light = vec3(-0.45, -0.60, 0.90).normalize();
    let diffuse = normal.dot(light).max(0.0);
    let rgb = base * (0.48 + 0.52 * diffuse) + Vec3::splat((brightness - 1.0) * 0.35);
    Color::new(rgb.x.min(1.0), rgb.y.min(1.0), rgb.z.min(1.0), alpha)
}

fn ball_mesh(center: Vec2, radius: f32, rotation: f32, brightness: f32, alpha: f32) -> Mesh {
    const RINGS: usize = 12;
    const SECTORS: usize = 64;
    let mut mesh = Mesh {
        vertices: Vec::with_capacity(RINGS * SECTORS * 4),
        indices: Vec::with_capacity(RINGS * SECTORS * 6),
        texture: None,
    };
    for sector in 0..SECTORS {
        let base = if sector < SECTORS / 2 {
            vec3(0.98, 0.985, 1.0)
        } else {
            vec3(0.96, 0.035, 0.025)
        };
        let angle = rotation + sector as f32 * std::f32::consts::TAU / SECTORS as f32;
        let next = angle + std::f32::consts::TAU / SECTORS as f32;
        for ring in 0..RINGS {
            let inner = ring as f32 / RINGS as f32;
            let outer = (ring + 1) as f32 / RINGS as f32;
            let n = mesh.vertices.len() as u16;
            for (a, r) in [(angle, inner), (next, inner), (next, outer), (angle, outer)] {
                let point = vec2(a.cos(), a.sin()) * r;
                let pos = center + point * radius;
                mesh.vertices.push(Vertex::new(
                    pos.x,
                    pos.y,
                    0.0,
                    0.0,
                    0.0,
                    sphere_color(base, point, brightness, alpha),
                ));
            }
            mesh.indices
                .extend_from_slice(&[n, n + 1, n + 2, n, n + 2, n + 3]);
        }
    }
    mesh
}

fn draw_ball_surface(center: Vec2, radius: f32, rotation: f32, brightness: f32, alpha: f32) {
    let black = Color::new(0.13, 0.14, 0.17, alpha);
    draw_poly_lines(
        center.x,
        center.y,
        64,
        radius * 0.955,
        rotation.to_degrees(),
        radius * 0.045,
        black,
    );
    let inner = radius * 0.955;
    draw_mesh(&ball_mesh(center, inner, rotation, brightness, alpha));
    let direction = vec2(rotation.cos(), rotation.sin()) * inner;
    let a = center + direction;
    let b = center - direction;
    let button_edge = vec2(rotation.cos(), rotation.sin()) * radius * 0.25;
    let start = center + button_edge;
    let end = center - button_edge;
    draw_line(a.x, a.y, start.x, start.y, radius * 0.055, black);
    draw_line(end.x, end.y, b.x, b.y, radius * 0.055, black);
    draw_poly_lines(
        center.x,
        center.y,
        64,
        radius * 0.20,
        0.0,
        radius * 0.05,
        black,
    );
    draw_poly(
        center.x,
        center.y,
        64,
        radius * 0.20,
        0.0,
        Color::new(0.96, 0.97, 1.0, alpha),
    );
}

fn highlight_mesh(center: Vec2, radius: f32, alpha: f32) -> Mesh {
    const RINGS: usize = 12;
    const SECTORS: usize = 48;
    let mut mesh = Mesh {
        vertices: Vec::with_capacity(RINGS * SECTORS * 4),
        indices: Vec::with_capacity(RINGS * SECTORS * 6),
        texture: None,
    };
    let center = center + vec2(-0.52, -0.56) * radius;
    let angle: f32 = -35.0_f32.to_radians();
    for ring in 0..RINGS {
        for sector in 0..SECTORS {
            let n = mesh.vertices.len() as u16;
            for (r, a) in [
                (ring, sector),
                (ring + 1, sector),
                (ring + 1, sector + 1),
                (ring, sector + 1),
            ] {
                let r = r as f32 / RINGS as f32;
                let a = a as f32 * std::f32::consts::TAU / SECTORS as f32;
                let p = vec2(a.cos() * 0.16, a.sin() * 0.11) * radius * r;
                let p = center
                    + vec2(
                        p.x * angle.cos() - p.y * angle.sin(),
                        p.x * angle.sin() + p.y * angle.cos(),
                    );
                let opacity = alpha * (1.0 - r * r).powi(2);
                mesh.vertices.push(Vertex::new(
                    p.x,
                    p.y,
                    0.0,
                    0.0,
                    0.0,
                    Color::new(1.0, 1.0, 1.0, opacity),
                ));
            }
            mesh.indices
                .extend_from_slice(&[n, n + 1, n + 2, n, n + 2, n + 3]);
        }
    }
    mesh
}

pub fn draw_ball(
    center: Vec2,
    size: Vec2,
    rotation: f32,
    brightness: f32,
    alpha: f32,
    light: &Material,
) {
    let strength = ((size.x - size.y).abs() / size.x.max(1.0)).clamp(0.0, 1.0);
    if strength > 0.01 {
        for i in (0..3).rev() {
            let spread = 1.0 + i as f32 * 0.045;
            draw_ellipse(
                center.x,
                center.y,
                size.x * 0.5 * spread,
                size.y * 0.5 * spread,
                0.0,
                Color::new(1.0, 0.98, 0.92, alpha * strength * 0.04),
            );
        }
    }
    // Apply one transform to the complete sphere, including its rim and highlight.
    let transform = Mat4::from_translation(vec3(center.x, center.y, 0.0))
        * Mat4::from_scale(vec3(size.x * 0.5, size.y * 0.5, 1.0));
    unsafe {
        get_internal_gl().quad_gl.push_model_matrix(transform);
    }
    draw_ball_surface(Vec2::ZERO, 1.0, rotation, brightness, alpha);
    gl_use_material(light);
    draw_mesh(&highlight_mesh(Vec2::ZERO, 1.0, alpha));
    gl_use_default_material();
    unsafe {
        get_internal_gl().quad_gl.pop_model_matrix();
    }
}

pub fn draw_shadow(center: Vec2, size: Vec2, alpha: f32) {
    // Warstwy tworzą łagodne brzegi; łączna nieprzezroczystość ~25%, nie 65%.
    for i in 0..10 {
        let p = i as f32 / 10.0;
        let scale = 1.0 - p * 0.55;
        draw_ellipse(
            center.x,
            center.y,
            size.x * 0.5 * scale,
            size.y * 0.5 * scale,
            0.0,
            Color::new(0.26, 0.20, 0.40, alpha * (0.012 + p * 0.035)),
        );
    }
}

/// Visual-only hit feedback. Time advances on simulation ticks, not render frames.
#[derive(Default)]
pub struct BallImpact {
    contacts: [bool; 2],
    squash: f32,
    squash_hold: bool,
    sand: SandBurst,
}

impl BallImpact {
    pub fn advance(
        &mut self,
        contacts: [bool; 2],
        world_bounce: bool,
        power_hit: bool,
        ground: Option<Vec2>,
    ) -> Vec2 {
        let hit = contacts
            .iter()
            .zip(self.contacts)
            .any(|(&now, before)| now && !before);
        self.contacts = contacts;
        if hit || world_bounce || ground.is_some() {
            self.squash = if power_hit { 1.0 } else { 0.2 };
            self.squash_hold = true;
        } else if self.squash_hold {
            self.squash_hold = false;
        } else {
            self.squash = (self.squash - 0.25).max(0.0);
        }
        self.sand.advance();
        if let Some(center) = ground {
            self.sand = SandBurst::new(center, 1.0);
        }
        vec2(1.0 + self.squash * 0.22, 1.0 - self.squash * 0.50)
    }

    pub fn draw_sand(&self, t: f32) {
        self.sand.draw(t);
    }
}

#[derive(Clone, Default)]
pub struct SandBurst {
    center: Vec2,
    scale: f32,
    age: f32,
    previous_age: f32,
    active: bool,
}

impl SandBurst {
    pub fn new(center: Vec2, scale: f32) -> Self {
        Self {
            center,
            scale,
            age: 1.0,
            previous_age: 0.0,
            active: true,
        }
    }

    pub fn advance(&mut self) {
        self.previous_age = self.age;
        if self.active {
            self.age = (self.age + 1.0).min(10.0);
        }
    }

    pub fn finished(&self) -> bool {
        !self.active || self.previous_age >= 10.0
    }

    #[cfg(test)]
    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn draw(&self, t: f32) {
        if !self.active {
            return;
        }
        let progress =
            (self.previous_age + (self.age - self.previous_age) * t.clamp(0.0, 1.0)) / 10.0;
        if progress >= 1.0 {
            return;
        }
        draw_mesh(&sand_dust_mesh(self.center, progress, self.scale));
        for i in 0..110 {
            let r1 = particle_random(i, 1);
            let r2 = particle_random(i, 2);
            let r3 = particle_random(i, 3);
            let delay = r3 * 0.18;
            let age = ((progress - delay) / (1.0 - delay)).clamp(0.0, 1.0);
            if progress < delay || age >= 1.0 {
                continue;
            }
            // Start across the ball's width and follow distinct ballistic arcs.
            let origin_x = (r1 * 2.0 - 1.0) * 20.0;
            let velocity_x = (r2 * 2.0 - 1.0) * 35.0 + origin_x * 0.35;
            let height = 16.0 + r3 * 38.0;
            let position = self.center
                + vec2(
                    origin_x + velocity_x * age,
                    -height * 4.0 * age * (1.0 - age),
                ) * self.scale;
            let fade = (1.0 - age).powf(1.2);
            let radius = (0.6 + r2 * 1.1) * self.scale;
            let color = match i % 3 {
                0 => vec3(0.85, 0.68, 0.47),
                1 => vec3(0.68, 0.48, 0.31),
                _ => vec3(0.96, 0.82, 0.62),
            };
            draw_circle(
                position.x,
                position.y,
                radius * 1.8,
                Color::new(color.x, color.y, color.z, fade * 0.10),
            );
            draw_circle(
                position.x,
                position.y,
                radius,
                Color::new(color.x, color.y, color.z, fade * 0.70),
            );
        }
    }
}

// Stable per-particle variation, independent of the simulation RNG.
pub(crate) fn particle_random(index: u32, seed: u32) -> f32 {
    let mut n = index
        .wrapping_mul(0x9e37_79b9)
        .wrapping_add(seed.wrapping_mul(0x85eb_ca6b));
    n ^= n >> 16;
    n = n.wrapping_mul(0x7feb_352d);
    n ^= n >> 15;
    (n & 0xffff) as f32 / 65535.0
}

fn sand_dust_mesh(center: Vec2, progress: f32, scale: f32) -> Mesh {
    const RINGS: usize = 6;
    const SECTORS: usize = 16;
    let mut mesh = Mesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        texture: None,
    };
    let life = (progress * 8.0).min(1.0) * (1.0 - progress).powi(2);
    for cloud in 0..6 {
        let spread = cloud as f32 / 5.0 * 2.0 - 1.0;
        let origin = center
            + vec2(
                spread * (13.0 + progress * 18.0),
                -progress * (8.0 + (cloud % 3) as f32 * 4.0),
            ) * scale;
        let radius = (5.0 + progress * (11.0 + (cloud % 3) as f32 * 3.0)) * scale;
        for ring in 0..RINGS {
            for sector in 0..SECTORS {
                let start = mesh.vertices.len() as u16;
                for (r, a) in [
                    (ring, sector),
                    (ring + 1, sector),
                    (ring + 1, sector + 1),
                    (ring, sector + 1),
                ] {
                    let fraction = r as f32 / RINGS as f32;
                    let angle = a as f32 * std::f32::consts::TAU / SECTORS as f32;
                    let pos = origin + vec2(angle.cos(), angle.sin() * 0.65) * radius * fraction;
                    let alpha = life * 0.11 * (1.0 - fraction * fraction).powi(2);
                    mesh.vertices.push(Vertex::new(
                        pos.x,
                        pos.y,
                        0.0,
                        0.0,
                        0.0,
                        Color::new(0.84, 0.68, 0.48, alpha),
                    ));
                }
                mesh.indices.extend_from_slice(&[
                    start,
                    start + 1,
                    start + 2,
                    start,
                    start + 2,
                    start + 3,
                ]);
            }
        }
    }
    mesh
}

const TRAIL_LIFETIME: f32 = 0.28;
const TICK: f32 = 1.0 / 25.0;

struct TrailPoint {
    position: Vec2,
    rotation: f32,
    age: f32,
}

#[derive(Default)]
pub struct BallTrail {
    points: VecDeque<TrailPoint>,
}

fn trail_life(age: f32, t: f32) -> f32 {
    (1.0 - (age - TICK * (1.0 - t.clamp(0.0, 1.0))) / TRAIL_LIFETIME).clamp(0.0, 1.0)
}

impl BallTrail {
    pub fn clear(&mut self) {
        self.points.clear();
    }

    pub fn advance(&mut self, position: Vec2, rotation: f32, enabled: bool) {
        for point in &mut self.points {
            point.age += TICK;
        }
        self.points.retain(|point| point.age < TRAIL_LIFETIME);
        if enabled {
            self.points.push_back(TrailPoint {
                position,
                rotation,
                age: 0.0,
            });
        }
    }

    // A jittering label reverses direction too quickly for a smooth ribbon.
    // Radial samples preserve the ball smoke's color and lifetime without folds.
    fn smoke_cloud_mesh(&self, head: Vec2, t: f32) -> Mesh {
        const RINGS: usize = 6;
        const SECTORS: usize = 16;
        let mut mesh = Mesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            texture: None,
        };
        let last = self.points.len().saturating_sub(1);
        for (i, point) in self.points.iter().enumerate() {
            let position = if i == last { head } else { point.position };
            let life = trail_life(point.age, t);
            let radius = 30.0 * life;
            for ring in 0..RINGS {
                for sector in 0..SECTORS {
                    let start = mesh.vertices.len() as u16;
                    for (r, a) in [
                        (ring, sector),
                        (ring + 1, sector),
                        (ring + 1, sector + 1),
                        (ring, sector + 1),
                    ] {
                        let fraction = r as f32 / RINGS as f32;
                        let angle = a as f32 * std::f32::consts::TAU / SECTORS as f32;
                        let p = position + vec2(angle.cos(), angle.sin()) * radius * fraction;
                        let alpha = 0.16 * life.powi(3) * (1.0 - fraction * fraction).powi(2);
                        mesh.vertices.push(Vertex::new(
                            p.x,
                            p.y,
                            0.0,
                            0.0,
                            0.0,
                            Color::new(0.88, 0.95, 1.0, alpha),
                        ));
                    }
                    mesh.indices.extend_from_slice(&[
                        start,
                        start + 1,
                        start + 2,
                        start,
                        start + 2,
                        start + 3,
                    ]);
                }
            }
        }
        mesh
    }

    pub fn draw_smoke_cloud(&self, head: Vec2, t: f32) {
        let mesh = self.smoke_cloud_mesh(head, t);
        // Spread the same soft plume across the width of the GAME OVER text.
        for x in [-64.0, 0.0, 64.0] {
            unsafe {
                get_internal_gl()
                    .quad_gl
                    .push_model_matrix(Mat4::from_translation(vec3(x, 0.0, 0.0)));
            }
            draw_mesh(&mesh);
            unsafe {
                get_internal_gl().quad_gl.pop_model_matrix();
            }
        }
    }

    pub fn draw_smoke(&self, head: Vec2, t: f32) {
        let last = self.points.len().saturating_sub(1);
        // Jedna wstęga zamiast nakładających się addytywnych kół i pasów.
        // Alfa w osi przy piłce wynosi 75%, a starszy ślad szybko zanika.
        let mut smoke = Mesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            texture: None,
        };
        const PROFILE: [(f32, f32); 7] = [
            (-1.0, 0.0),
            (-0.8, 0.04),
            (-0.55, 0.16),
            (0.0, 1.0),
            (0.55, 0.16),
            (0.8, 0.04),
            (1.0, 0.0),
        ];
        for i in 0..=last {
            let Some(point) = self.points.get(i) else {
                break;
            };
            let position = if i == last { head } else { point.position };
            let previous = if i == 0 {
                position
            } else {
                self.points[i - 1].position
            };
            let next = if i + 1 >= last {
                head
            } else {
                self.points[i + 1].position
            };
            let direction = (next - previous).normalize_or_zero();
            let normal = vec2(-direction.y, direction.x);
            let life = trail_life(point.age, t);
            for (offset, coverage) in PROFILE {
                let p = position + normal * offset * 36.0 * life;
                smoke.vertices.push(Vertex::new(
                    p.x,
                    p.y,
                    0.0,
                    0.0,
                    0.0,
                    Color::new(0.88, 0.95, 1.0, 0.75 * life.powi(3) * coverage),
                ));
            }
            if i > 0 {
                for j in 0..PROFILE.len() - 1 {
                    let a = ((i - 1) * PROFILE.len() + j) as u16;
                    let b = (i * PROFILE.len() + j) as u16;
                    smoke
                        .indices
                        .extend_from_slice(&[a, b, b + 1, a, b + 1, a + 1]);
                }
            }
        }
        gl_use_default_material();
        draw_mesh(&smoke);
    }

    pub fn draw(&self, head: Vec2, radius: f32, t: f32, light: &Material) {
        self.draw_smoke(head, t);
        let last = self.points.len().saturating_sub(1);
        gl_use_material(light);
        // Wtórne obrazy mają rzeczywisty kształt/wzór piłki, nie kształt dymu.
        // Od najstarszego do najnowszego, z płynnym zanikiem i interpolacją pozycji.
        for i in 0..last {
            let point = &self.points[i];
            let next = &self.points[i + 1];
            let position = point.position.lerp(next.position, t.clamp(0.0, 1.0));
            let life = trail_life(point.age, t);
            let rotation =
                crate::gfx::lerp_rotation(point.rotation as f64, next.rotation as f64, t as f64)
                    as f32;
            draw_ball_surface(position, radius, rotation, 1.4, 0.42 * life.powf(1.5));
        }
        gl_use_default_material();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impacts_squash_once_per_contact_and_sand_only_on_ground() {
        let mut impact = BallImpact::default();
        assert_eq!(
            impact.advance([false, false], false, false, None),
            Vec2::ONE
        );
        let compressed = impact.advance([true, false], false, true, None);
        assert!((compressed.x - 1.22).abs() < 1e-6);
        assert_eq!(compressed.y, 0.5);
        assert!(!impact.sand.active);
        assert_eq!(
            impact.advance([true, false], false, false, None),
            compressed
        );
        let recovering = impact.advance([true, false], false, false, None);
        assert!(recovering.x < compressed.x && recovering.y > compressed.y);
        impact.advance([false, false], false, false, None);
        let ordinary_hit = impact.advance([false, true], false, false, None);
        assert!((ordinary_hit.y - 0.9).abs() < 1e-6);
        let world_hit = impact.advance([false, false], true, false, None);
        assert!((world_hit.y - 0.9).abs() < 1e-6);
        let ground = vec2(100.0, 272.0);
        impact.advance([false, false], true, false, Some(ground));
        assert!(impact.sand.active);
        assert_eq!(impact.sand.center, ground);
        assert_eq!(impact.sand.previous_age, 0.0);
        for _ in 0..10 {
            impact.advance([false, false], false, false, None);
        }
        assert_eq!(impact.sand.age, 10.0);
    }

    #[test]
    fn sand_dust_mesh_is_bounded_and_fades_at_endpoints() {
        let mesh = sand_dust_mesh(vec2(100.0, 272.0), 0.5, 1.0);
        assert!(mesh.vertices.len() < 5000 && mesh.indices.len() < 5000);
        assert!(mesh
            .indices
            .iter()
            .all(|&i| (i as usize) < mesh.vertices.len()));
        assert!(mesh.vertices.iter().any(|v| v.color[3] > 0));
        for progress in [0.0, 1.0] {
            assert!(sand_dust_mesh(Vec2::ZERO, progress, 1.0)
                .vertices
                .iter()
                .all(|v| v.color[3] == 0));
        }
    }

    #[test]
    fn sphere_lighting_favors_the_upper_left_and_keeps_alpha() {
        let base = vec3(0.8, 0.2, 0.1);
        let lit = sphere_color(base, vec2(-0.3, -0.4), 1.0, 0.6);
        let dark = sphere_color(base, vec2(0.6, 0.6), 1.0, 0.6);
        assert!(lit.r > dark.r && lit.g > dark.g);
        assert_eq!(lit.a, 0.6);
    }

    #[test]
    fn highlight_is_white_with_an_opaque_core_and_a_transparent_edge() {
        let mesh = highlight_mesh(vec2(50.0, 50.0), 20.0, 1.0);
        assert!(mesh
            .vertices
            .iter()
            .all(|v| v.color[..3] == [255, 255, 255]));
        assert!(mesh.vertices.iter().any(|v| v.color[3] == 255));
        assert!(mesh.vertices.iter().any(|v| v.color[3] == 0));
    }

    #[test]
    fn shaded_ball_mesh_has_valid_indices_and_stays_inside_radius() {
        let center = vec2(30.0, 50.0);
        for angle in [0.0, 1.4, 6.2] {
            let mesh = ball_mesh(center, 20.0, angle, 1.3, 1.0);
            assert!(mesh
                .indices
                .iter()
                .all(|&i| (i as usize) < mesh.vertices.len()));
            assert!(mesh
                .vertices
                .iter()
                .all(|v| (v.position.truncate() - center).length() <= 20.001));
        }
    }

    #[test]
    fn jitter_smoke_is_soft_and_fits_renderer_limits() {
        let mut trail = BallTrail::default();
        for i in 0..12 {
            trail.advance(vec2(i as f32, 0.0), 0.0, true);
        }
        let mesh = trail.smoke_cloud_mesh(vec2(12.0, 0.0), 1.0);
        assert!(mesh.vertices.len() < 5000 && mesh.indices.len() < 5000);
        assert!(mesh
            .indices
            .iter()
            .all(|&i| (i as usize) < mesh.vertices.len()));
        assert!(mesh.vertices.iter().any(|v| v.color[3] > 0));
        assert!(mesh.vertices.iter().any(|v| v.color[3] == 0));
    }

    #[test]
    fn trail_fades_after_power_hit_and_reset_clears_it() {
        let mut trail = BallTrail::default();
        for i in 0..30 {
            trail.advance(vec2(i as f32, 0.0), 0.0, true);
        }
        assert!(trail.points.len() <= 8);
        for _ in 0..8 {
            trail.advance(Vec2::ZERO, 0.0, false);
        }
        assert!(trail.points.is_empty());
        trail.advance(Vec2::ZERO, 0.0, true);
        trail.clear();
        assert!(trail.points.is_empty());
    }
}
