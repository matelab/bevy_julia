#[derive(Clone, Copy, Debug)]
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl std::ops::Mul<f32> for Color {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Color {
            r: self.r * rhs,
            g: self.g * rhs,
            b: self.b * rhs,
            a: self.a * rhs,
        }
    }
}

impl std::ops::Add for Color {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Color {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
            a: self.a + rhs.a,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tick {
    position: f32,
    color: Color,
}

#[derive(Debug)]
pub struct ColorRamp {
    ticks: Vec<Tick>,
}

impl ColorRamp {
    pub fn new() -> ColorRamp {
        ColorRamp { ticks: vec![] }
    }

    pub fn add(&mut self, position: f32, r: f32, g: f32, b: f32, a: f32) {
        self.ticks.push(Tick {
            position,
            color: Color { r, g, b, a },
        });
        self.ticks
            .sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());
    }

    pub fn interpolate(&self, pos: f32) -> Option<Color> {
        let mut span: Option<(&Tick, &Tick)> = None;
        for i in 0..self.ticks.len() - 1 {
            let t1 = &self.ticks[i];
            let t2 = &self.ticks[i + 1];
            if (pos >= t1.position) && (pos <= t2.position) {
                span = Some((t1, t2));
                break;
            }
        }
        let (t1, t2) = span?;
        let relpos = pos - t1.position;
        let factor = relpos / (t2.position - t1.position);
        Some(t1.color * (1.0 - factor) + t2.color * factor)
    }

    pub fn range(&self) -> Option<(f32, f32)> {
        if self.ticks.len() < 2 {
            None
        } else {
            Some((
                self.ticks.first().unwrap().position,
                self.ticks.last().unwrap().position,
            ))
        }
    }

    pub fn build_texture_data(&self, width: usize, height: usize) -> Option<Vec<u8>> {
        let (t0, t1) = self.range()?;
        let range = t1 - t0;
        let step = range / width as f32;
        let mut result: Vec<u8> = vec![];
        for p in 0..width - 1 {
            let pos = t0 + p as f32 * step;
            let col = self.interpolate(pos).unwrap();
            result.push((col.r * 255.).round() as u8);
            result.push((col.g * 255.).round() as u8);
            result.push((col.b * 255.).round() as u8);
            result.push((col.a * 255.).round() as u8);
        }
        let last = self.interpolate(t1).unwrap();
        result.push((last.r * 255.).round() as u8);
        result.push((last.g * 255.).round() as u8);
        result.push((last.b * 255.).round() as u8);
        result.push((last.a * 255.).round() as u8);
        let mut repeated = result.clone();
        for _ in 0..height - 1 {
            repeated.append(&mut result.clone());
        }
        Some(repeated)
    }
}
